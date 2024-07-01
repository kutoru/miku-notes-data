use crate::proto::files::files_server::{Files, FilesServer};
use crate::proto::files::{CreateFileReq, DeleteFileReq, DownloadFileMetadata, DownloadFileReq, Empty, File, FileData};
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
            .ok_or(Status::invalid_argument("First message in the stream is invalid"))??;

        let (user_id, note_id, file_name, expected_parts) = match first_part.metadata {
            Some(m) => (m.user_id, m.note_id, m.name, m.expected_parts),
            None => return Err(Status::invalid_argument("First message in the stream is invalid")),
        };

        println!("metadata: {}, {}, {}, {}", user_id, note_id, file_name, expected_parts);

        // making sure the note that the file is going to be attached to exists

        sqlx::query("SELECT id FROM notes WHERE id = $1 AND user_id = $2;")
            .bind(note_id).bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_to_status()?;

        let file_hash = uuid::Uuid::new_v4().to_string();
        let file_path = "./files/".to_owned() + &file_hash;

        let mut file = tokio::fs::File::create_new(&file_path).await?;
        file.set_max_buf_size(1024 * 1024 * self.chunk_size);
        let bytes_written = file.write(&first_part.data).await?;

        println!("hash, path: {}, {}", file_hash, file_path);
        println!("part 1: {} ({})", first_part.data.len(), bytes_written);

        // processing the rest of the parts

        let mut current_part = 1;
        while let Some(file_part) = stream.next().await {
            current_part += 1;

            if current_part > expected_parts {
                tokio::fs::remove_file(file_path).await?;
                return Err(Status::invalid_argument("Amount of parts exceeded the expected amount"));
            }

            let file_part = file_part?;
            let bytes_written = file.write(&file_part.data).await?;
            println!("part {}: {} ({})", current_part, file_part.data.len(), bytes_written);
        }

        if current_part < expected_parts {
            tokio::fs::remove_file(file_path).await?;
            return Err(Status::invalid_argument("Amount of parts is smaller than the expected amount"));
        }

        // saving the file data

        let size = file.metadata().await?.len() as i64;
        let mut transaction = self.pool
            .begin()
            .await
            .map_to_status()?;

        let mut new_file_info = sqlx::query_as::<_, File>("INSERT INTO files (user_id, hash, name, size) VALUES ($1, $2, $3, $4) RETURNING *;")
            .bind(user_id).bind(file_hash).bind(file_name).bind(size).bind(note_id)
            .fetch_one(&mut *transaction)
            .await
            .map_to_status()?;

        sqlx::query("INSERT INTO note_files (note_id, file_id) VALUES ($1, $2);")
            .bind(note_id).bind(new_file_info.id)
            .execute(&mut *transaction)
            .await
            .map_to_status()?;

        transaction
            .commit()
            .await
            .map_to_status()?;

        new_file_info.note_id = Some(note_id);
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

        let file_size = file.metadata().await.unwrap().len();
        // let chunk_size = (1024 * 1024 * self.chunk_size) as u64;
        let chunk_size = 1024 * 1024 * 3 + 1024 * 512;  // unforunately, anything around and above 4mb doesn't get accepted by browsers or something
        let expected_parts = (file_size / chunk_size) as i32 + (file_size % chunk_size > 0) as i32;
        let last_part_len = (file_size % chunk_size) as usize;

        file.set_max_buf_size(chunk_size as usize);
        let mut buffer = vec![0; chunk_size as usize];

        println!("size, chunk, parts: {}, {}, {}", file_size, chunk_size, expected_parts);

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
                    expected_parts: expected_parts,
                }),
            };

            match sender.send(Ok(metadata_part)).await {
                Ok(_) => (),
                Err(e) => println!("SENDER SEND ERR: {}", e),
            };

            // and then send the actual file data by reading the file

            for i in 1..=expected_parts {
                match file.read(&mut buffer).await {
                    Ok(len) => if len == 0 { println!("FILE READ LEN == 0"); break; },
                    Err(e) => { println!("FILE READ ERR: {:#?}", e); break; },
                }

                let data = match i == expected_parts {
                    true => buffer[0..last_part_len].to_vec(),
                    false => buffer.clone(),
                };

                if i < 10 || (i < 100 && i % 10 == 0) || i % 100 == 0 || i == expected_parts {
                    println!("buf {}: {}", i, data.len());
                }

                let data_part = FileData {
                    data: data,
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
