use crate::proto::tags::tags_server::{Tags, TagsServer};
use crate::proto::tags::{CreateTagReq, ReadTagsReq, UpdateTagReq, DeleteTagReq, Tag, TagList, Empty};
use crate::models::ServiceResult;

use tonic::{Request, Response, Status};
use sqlx::PgPool;

pub fn get_service(pool: PgPool) -> TagsServer<TagServiceState> {
    let service_state = TagServiceState { pool };
    TagsServer::new(service_state)
}

#[derive(Debug)]
pub struct TagServiceState {
    pool: PgPool,
}

#[tonic::async_trait]
impl Tags for TagServiceState {
    async fn create_tag(
        &self,
        request: Request<CreateTagReq>,
    ) -> ServiceResult<Tag> {
        Err(Status::unimplemented(""))
    }

    async fn read_tags(
        &self,
        request: Request<ReadTagsReq>,
    ) -> ServiceResult<TagList> {
        Err(Status::unimplemented(""))
    }

    async fn update_tag(
        &self,
        request: Request<UpdateTagReq>,
    ) -> ServiceResult<Tag> {
        Err(Status::unimplemented(""))
    }

    async fn delete_tag(
        &self,
        request: Request<DeleteTagReq>,
    ) -> ServiceResult<Empty> {
        Err(Status::unimplemented(""))
    }
}
