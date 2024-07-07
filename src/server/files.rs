use crate::proto::files::create_file_metadata::AttachId;
use crate::proto::files::files_server::{Files, FilesServer};
use crate::proto::files::{CreateFileMetadata, CreateFileReq, DeleteFileReq, DownloadFileMetadata, DownloadFileReq, Empty, File, FileData};
use crate::types::{AppState, HandleServiceError, ServiceResult};

use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Streaming, Status};

pub fn get_service(state: AppState) -> FilesServer<AppState> {
    let chunk_size = state.chunk_size;
    FilesServer::new(state)
        .max_decoding_message_size(1024 * 1024 * (chunk_size + 1))  // 1 extra mb for fields other than data
}

#[derive(Debug)]
struct FileDefer {
    file_path: String,
    delete: bool,
}

// the struct's drop here is used as a golang-like defer.
// in theory, synchronously deleting a file might not be great (performance-wise).
// if it turns out to be a big issue, this crate could be used https://crates.io/crates/defer-drop
impl Drop for FileDefer {
    fn drop(&mut self) {
        if self.delete {
            if let Err(e) = std::fs::remove_file(&self.file_path) {
                println!("Could not delete a file: {}; {:?}", self.file_path, e);
            }
        }
    }
}

#[tonic::async_trait]
impl Files for AppState {
    async fn create_file(
        &self,
        request: Request<Streaming<CreateFileReq>>,
    ) -> ServiceResult<File> {

        println!("\nGOT CREATE FILE REQ");

        let mut stream = request.into_inner();

        // processing the first part

        let first_part = stream.next().await
            .ok_or(Status::invalid_argument("invalid field"))??;

        let Some(CreateFileMetadata {
            user_id,
            attach_id: Some(attach_id),
            name: file_name,
        }) = first_part.metadata else {
            return Err(Status::invalid_argument("invalid field"));
        };

        println!("metadata: {}, {:?}, {}", user_id, attach_id, file_name);

        // making sure the note or the shelf that the file is going to be attached to exists

        let (query, attach_id_val) = match attach_id {
            AttachId::NoteId(note_id) => (
                sqlx::query("SELECT id FROM notes WHERE id = $1 AND user_id = $2;"),
                note_id,
            ),
            AttachId::ShelfId(shelf_id) => (
                sqlx::query("SELECT id FROM shelves WHERE id = $1 AND user_id = $2;"),
                shelf_id,
            ),
        };

        query
            .bind(attach_id_val).bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_to_status()?;

        // preparing file stuff

        let file_hash = uuid::Uuid::new_v4().to_string();
        let file_path = format!("./files/{}", file_hash);

        let mut file = tokio::fs::File::create_new(&file_path).await?;
        file.set_max_buf_size(1024 * 1024 * self.chunk_size);

        let mut file_defer = FileDefer {
            file_path: file_path.clone(),
            delete: true,
        };

        println!("file_path: {}", file_path);

        // processing the rest of the parts

        let mut i = 0;
        while let Some(file_part) = stream.next().await {
            i += 1;

            let file_part = file_part?;
            let bytes_written = file.write(&file_part.data).await?;

            if i < 10 || (i < 100 && i % 10 == 0) || (i < 1000 && i % 100 == 0) || i % 1000 == 0 {
                println!("part {}: {} ({})", i, file_part.data.len(), bytes_written);
            }
        }

        // saving the file data

        let size = file.metadata().await?.len() as i64;
        let mut transaction = self.pool
            .begin()
            .await
            .map_to_status()?;

        let mut new_file_info = sqlx::query_as::<_, File>("INSERT INTO files (user_id, hash, name, size) VALUES ($1, $2, $3, $4) RETURNING *;")
            .bind(user_id).bind(file_hash).bind(file_name).bind(size)
            .fetch_one(&mut *transaction)
            .await
            .map_to_status()?;

        let query = match attach_id {
            AttachId::NoteId(_) => sqlx::query("INSERT INTO note_files (note_id, file_id) VALUES ($1, $2);"),
            AttachId::ShelfId(_) => sqlx::query("INSERT INTO shelf_files (shelf_id, file_id) VALUES ($1, $2);"),
        };

        query
            .bind(attach_id_val).bind(new_file_info.id)
            .execute(&mut *transaction)
            .await
            .map_to_status()?;

        transaction
            .commit()
            .await
            .map_to_status()?;

        file_defer.delete = false;

        new_file_info.attach_id = Some(attach_id_val);
        Ok(Response::new(new_file_info))
    }

    type DownloadFileStream = ReceiverStream<Result<FileData, Status>>;

    async fn download_file(
        &self,
        request: Request<DownloadFileReq>,
    ) -> ServiceResult<Self::DownloadFileStream> {

        let req_body = dbg!(request.into_inner());

        // checking the file in the db

        let file_info = sqlx::query_as::<_, File>("SELECT * FROM files WHERE hash = $1 AND user_id = $2;")
            .bind(&req_body.file_hash).bind(req_body.user_id)
            .fetch_one(&self.pool)
            .await
            .map_to_status()?;

        // preparing the file and size info

        let mut file = tokio::fs::File::open(format!("./files/{}", file_info.hash)).await
            .map_err(|_| tonic::Status::internal("could not find the file"))?;

        let file_size = file.metadata().await?.len() as usize;
        let chunk_size = 1024 * 1024 * self.chunk_size;

        file.set_max_buf_size(chunk_size);
        let mut buffer = vec![0; chunk_size];

        println!("size, chunk: {}, {}", file_size, chunk_size);

        // defining a channel that yields FileData objects with file data

        let (sender, receiver) = mpsc::channel(4);

        tokio::spawn(async move {
            println!("stream start");

            // send the metadata without any file data first

            let metadata_part = FileData {
                data: Vec::default(),
                metadata: Some(DownloadFileMetadata {
                    name: file_info.name,
                    size: file_size as i64,
                }),
            };

            match sender.send(Ok(metadata_part)).await {
                Ok(_) => (),
                Err(e) => println!("SENDER SEND ERR: {}", e),
            };

            // and then send the actual file data by reading the file

            let mut i = 0;
            loop {
                i += 1;

                let bytes_read = match file.read(&mut buffer).await {
                    Ok(l) => l,
                    Err(e) => {
                        println!("FILE READ ERR: {:#?}", e);
                        break;
                    },
                };

                if bytes_read == 0 {
                    println!("buf len == 0");
                    break;
                }

                let data = buffer[0..bytes_read].to_vec();

                if i < 10 || (i < 100 && i % 10 == 0) || (i < 1000 && i % 100 == 0) || i % 1000 == 0 {
                    println!("buf {}: {}", i, data.len());
                }

                let data_part = FileData {
                    data,
                    metadata: None,
                };

                match sender.send(Ok(data_part)).await {
                    Ok(_) => (),
                    Err(e) => { println!("SENDER SEND ERR: {}", e); break; },
                };
            }
        });

        Ok(Response::new(ReceiverStream::new(receiver)))
    }

    async fn delete_file(
        &self,
        request: Request<DeleteFileReq>,
    ) -> ServiceResult<Empty> {

        let req_body = request.into_inner();

        let mut transaction = self.pool
            .begin()
            .await
            .map_to_status()?;

        sqlx::query("DELETE FROM note_files WHERE file_id = $1;")
            .bind(req_body.id)
            .execute(&mut *transaction)
            .await
            .map_to_status()?;

        sqlx::query("DELETE FROM shelf_files WHERE file_id = $1;")
            .bind(req_body.id)
            .execute(&mut *transaction)
            .await
            .map_to_status()?;

        let deleted_file = sqlx::query_as::<_, File>("DELETE FROM files WHERE id = $1 AND user_id = $2 RETURNING *;")
            .bind(req_body.id).bind(req_body.user_id)
            .fetch_one(&mut *transaction)
            .await
            .map_to_status()?;

        transaction
            .commit()
            .await
            .map_to_status()?;

        let file_path = "./files/".to_owned() + &deleted_file.hash;
        tokio::fs::remove_file(file_path)
            .await
            .unwrap_or_else(|e| println!("Could not delete a file: {:?};\nBecause error: {:?};", deleted_file, e));

        Ok(Response::new(Empty {}))
    }
}
