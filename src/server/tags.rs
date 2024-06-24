use crate::proto::tags::tags_server::{Tags, TagsServer};
use crate::proto::tags::{CreateTagReq, ReadTagsReq, UpdateTagReq, DeleteTagReq, Tag, TagList, Empty};
use crate::types::{AppState, ServiceResult};

use tonic::{Request, Response, Status};

pub fn get_service(state: AppState) -> TagsServer<AppState> {
    TagsServer::new(state)
}

#[tonic::async_trait]
impl Tags for AppState {
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
