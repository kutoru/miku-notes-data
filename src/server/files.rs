use crate::proto::files::files_server::{Files, FilesServer};
use crate::proto::files::{CreateFileReq, DeleteFileReq, File, Empty};
use crate::types::ServiceResult;

use tonic::{Request, Response, Streaming, Status};
use sqlx::PgPool;

pub fn get_service(pool: PgPool) -> FilesServer<FileServiceState> {
    let service_state = FileServiceState { pool };
    FilesServer::new(service_state)
}

#[derive(Debug)]
pub struct FileServiceState {
    pool: PgPool,
}

#[tonic::async_trait]
impl Files for FileServiceState {
    async fn create_file(
        &self,
        request: Request<Streaming<CreateFileReq>>,
    ) -> ServiceResult<File> {
        Err(Status::unimplemented(""))
    }

    async fn delete_file(
        &self,
        request: Request<DeleteFileReq>,
    ) -> ServiceResult<Empty> {
        Err(Status::unimplemented(""))
    }
}
