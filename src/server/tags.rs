use crate::proto::tags::tags_server::{Tags, TagsServer};
use crate::proto::tags::{CreateTagReq, ReadTagsReq, UpdateTagReq, DeleteTagReq, Tag, TagList, Empty};
use crate::types::{AppState, HandleServiceError, ServiceResult};

use tonic::{Request, Response};

pub fn get_service(state: AppState) -> TagsServer<AppState> {
    TagsServer::new(state)
}

#[tonic::async_trait]
impl Tags for AppState {
    async fn create_tag(
        &self,
        request: Request<CreateTagReq>,
    ) -> ServiceResult<Tag> {

        let req_body = request.into_inner();

        let new_tag = sqlx::query_as::<_, Tag>("INSERT INTO tags (user_id, name) VALUES ($1, $2) RETURNING *;")
            .bind(req_body.user_id).bind(req_body.name)
            .fetch_one(&self.pool)
            .await
            .map_to_status()?;

        Ok(Response::new(new_tag))
    }

    async fn read_tags(
        &self,
        request: Request<ReadTagsReq>,
    ) -> ServiceResult<TagList> {

        let req_body = request.into_inner();

        let tags = sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE user_id = $1 ORDER BY id;")
            .bind(req_body.user_id)
            .fetch_all(&self.pool)
            .await
            .map_to_status()?;

        Ok(Response::new(TagList { tags }))
    }

    async fn update_tag(
        &self,
        request: Request<UpdateTagReq>,
    ) -> ServiceResult<Tag> {

        let req_body = request.into_inner();

        let updated_tag = sqlx::query_as::<_, Tag>("UPDATE tags SET name = $1 WHERE id = $2 AND user_id = $3 RETURNING *;")
            .bind(req_body.name).bind(req_body.id).bind(req_body.user_id)
            .fetch_one(&self.pool)
            .await
            .map_to_status()?;

        Ok(Response::new(updated_tag))
    }

    async fn delete_tag(
        &self,
        request: Request<DeleteTagReq>,
    ) -> ServiceResult<Empty> {

        let req_body = request.into_inner();

        let mut transaction = self.pool
            .begin()
            .await
            .map_to_status()?;

        sqlx::query("DELETE FROM note_tags WHERE tag_id = $1;")
            .bind(req_body.id)
            .execute(&mut *transaction)
            .await
            .map_to_status()?;

        sqlx::query("DELETE FROM tags WHERE id = $1 AND user_id = $2;")
            .bind(req_body.id).bind(req_body.user_id)
            .execute(&mut *transaction)
            .await
            .map_to_status()?
            .rows_affected()
            .eq(&1)
            .then_some(())
            .ok_or(sqlx::Error::RowNotFound)
            .map_to_status()?;

        transaction
            .commit()
            .await
            .map_to_status()?;

        Ok(Response::new(Empty {}))
    }
}
