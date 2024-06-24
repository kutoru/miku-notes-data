use crate::proto::files::files_server::{Files, FilesServer};
use crate::proto::files::{CreateFileReq, DeleteFileReq, File, Empty};
use crate::types::{AppState, HandleServiceError, ServiceResult};

use tokio::io::AsyncWriteExt;
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

        let first_part = stream.next().await
            .ok_or(Status::invalid_argument("First message in the stream is invalid"))??;

        let (user_id, note_id, file_name, expected_parts) = match first_part.metadata {
            Some(m) => (m.user_id, m.note_id, m.name, m.expected_parts),
            None => return Err(Status::invalid_argument("First message in the stream is invalid")),
        };

        println!("metadata: {}, {}, {}, {}", user_id, note_id, file_name, expected_parts);

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
            return Err(Status::invalid_argument("Amount of parts exceeded the expected amount"));
        }

        let size = file.metadata().await?.len() as i64;
        let mut transaction = self.pool.begin().await.map_to_status()?;

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

    async fn delete_file(
        &self,
        request: Request<DeleteFileReq>,
    ) -> ServiceResult<Empty> {
        Err(Status::unimplemented(""))
    }
}
