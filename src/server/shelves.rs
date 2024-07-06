use crate::{proto::{files::File, notes::Note, shelves::{shelves_server::{Shelves, ShelvesServer}, ClearShelfReq, ConvertToNoteReq, ReadShelfReq, Shelf, UpdateShelfReq}}, types::{fill_tuple_placeholder, AppState, BindIter, HandleServiceError, IDWrapper, ServiceResult}};

use tonic::{Request, Response};

pub fn get_service(state: AppState) -> ShelvesServer<AppState> {
    ShelvesServer::new(state)
}

#[tonic::async_trait]
impl Shelves for AppState {
    async fn read_shelf(
        &self,
        request: Request<ReadShelfReq>,
    ) -> ServiceResult<Shelf> {

        let req_body = request.into_inner();

        let db_res = sqlx::query_as::<_, Shelf>("SELECT * FROM shelves WHERE user_id = $1;")
            .bind(req_body.user_id)
            .fetch_one(&self.pool)
            .await;

        let mut shelf = match db_res {
            Ok(shelf) => shelf,
            Err(sqlx::Error::RowNotFound) => {
                sqlx::query_as::<_, Shelf>("INSERT INTO shelves (user_id, text) VALUES ($1, '') RETURNING *;")
                    .bind(req_body.user_id)
                    .fetch_one(&self.pool)
                    .await
                    .map_to_status()?
            },
            _ => db_res.map_to_status()?,
        };

        let files = sqlx::query_as::<_, File>(r"
            SELECT f.*, sf.shelf_id AS attach_id FROM files AS f
            INNER JOIN shelf_files AS sf ON sf.file_id = f.id
            WHERE sf.shelf_id = $1
            ORDER BY f.id ASC;
        ")
            .bind(shelf.id)
            .fetch_all(&self.pool)
            .await
            .map_to_status()?;

        shelf.files = files;
        Ok(Response::new(shelf))
    }

    async fn update_shelf(
        &self,
        request: Request<UpdateShelfReq>,
    ) -> ServiceResult<Shelf> {

        let req_body = request.into_inner();

        let updated_shelf = sqlx::query_as::<_, Shelf>(r"
            UPDATE shelves
            SET text = $1, last_edited = NOW(), times_edited = times_edited + 1
            WHERE user_id = $2 RETURNING *;
        ")
            .bind(&req_body.text).bind(req_body.user_id)
            .fetch_one(&self.pool)
            .await
            .map_to_status()?;

        Ok(Response::new(updated_shelf))
    }

    async fn clear_shelf(
        &self,
        request: Request<ClearShelfReq>,
    ) -> ServiceResult<Shelf> {

        let req_body = request.into_inner();

        let mut transaction = self.pool
            .begin()
            .await
            .map_to_status()?;

        let shelf = sqlx::query_as::<_, Shelf>(r"
            UPDATE shelves
            SET text = '', last_edited = NOW(), times_edited = times_edited + 1
            WHERE user_id = $1 RETURNING *;
        ")
            .bind(req_body.user_id)
            .fetch_one(&mut *transaction)
            .await
            .map_to_status()?;

        let file_ids: Vec<_> = sqlx::query_as::<_, IDWrapper>("DELETE FROM shelf_files WHERE shelf_id = $1 RETURNING file_id AS id;")
            .bind(shelf.id)
            .fetch_all(&mut *transaction)
            .await
            .map_to_status()?
            .iter()
            .map(|v| v.id)
            .collect();

        let files = match file_ids.len() {
            0 => Vec::new(),
            _ => sqlx::query_as::<_, File>(&fill_tuple_placeholder(
                "DELETE FROM files WHERE user_id = $1 AND id IN () RETURNING *;",
                &file_ids, 1,
            ))
                .bind(req_body.user_id).bind_iter(file_ids)
                .fetch_all(&mut *transaction)
                .await
                .map_to_status()?
        };

        transaction
            .commit()
            .await
            .map_to_status()?;

        for file in &files {
            let file_path = format!("./files/{}", file.hash);
            if let Err(e) = tokio::fs::remove_file(file_path).await {
                println!("Could not delete a file: {:?};\nBecause error: {:?};", file, e);
            }
        }

        Ok(Response::new(shelf))
    }

    async fn convert_to_note(
        &self,
        request: Request<ConvertToNoteReq>,
    ) -> ServiceResult<Shelf> {

        let req_body = request.into_inner();

        let mut transaction = self.pool
            .begin()
            .await
            .map_to_status()?;

        let shelf = sqlx::query_as::<_, Shelf>(r"
            UPDATE shelves
            SET text = '', last_edited = NOW(), times_edited = times_edited + 1
            WHERE user_id = $1 RETURNING *;
        ")
            .bind(req_body.user_id)
            .fetch_one(&mut *transaction)
            .await
            .map_to_status()?;

        let file_ids: Vec<_> = sqlx::query_as::<_, IDWrapper>("DELETE FROM shelf_files WHERE shelf_id = $1 RETURNING file_id AS id;")
            .bind(shelf.id)
            .fetch_all(&mut *transaction)
            .await
            .map_to_status()?
            .iter()
            .map(|v| v.id)
            .collect();

        let note = sqlx::query_as::<_, Note>("INSERT INTO notes (user_id, title, text) VALUES ($1, $2, $3) RETURNING *;")
            .bind(req_body.user_id).bind(&req_body.note_title).bind(&req_body.note_text)
            .fetch_one(&mut *transaction)
            .await
            .map_to_status()?;

        sqlx::QueryBuilder::new("INSERT INTO note_files (note_id, file_id) ")
            .push_values(file_ids, |mut builder, file_id| {
                builder
                    .push_bind(note.id)
                    .push_bind(file_id);
            })
            .build()
            .execute(&mut *transaction)
            .await
            .map_to_status()?;

        transaction
            .commit()
            .await
            .map_to_status()?;

        Ok(Response::new(shelf))
    }
}
