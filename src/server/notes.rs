use crate::proto::notes::notes_server::{Notes, NotesServer};
use crate::proto::notes::{CreateNoteReq, ReadNotesReq, UpdateNoteReq, DeleteNoteReq, Note, NoteList, Empty};
use crate::models::{ServiceResult, IDWrapper, HandleServiceError};

use tonic::{Request, Response};
use sqlx::PgPool;

pub fn get_service(pool: PgPool) -> NotesServer<NoteServiceState> {
    let service_state = NoteServiceState { pool };
    NotesServer::new(service_state)
}

#[derive(Debug)]
pub struct NoteServiceState {
    pool: PgPool,
}

#[tonic::async_trait]
impl Notes for NoteServiceState {
    async fn create_note(
        &self,
        request: Request<CreateNoteReq>,
    ) -> ServiceResult<Note> {

        let req_note = request.into_inner();

        let note_id = sqlx::query_as::<_, IDWrapper>("INSERT INTO notes (user_id, title, text) VALUES ($1, $2, $3) RETURNING id;")
            .bind(req_note.user_id).bind(req_note.title).bind(req_note.text)
            .fetch_one(&self.pool)
            .await
            .map_status(1)?
            .id;

        let new_note = sqlx::query_as::<_, Note>("SELECT * FROM notes WHERE id = $1;")
            .bind(note_id)
            .fetch_one(&self.pool)
            .await
            .map_status(2)?;

        Ok(Response::new(new_note))
    }

    async fn read_notes(
        &self,
        request: Request<ReadNotesReq>,
    ) -> ServiceResult<NoteList> {

        let user_id = request.into_inner().user_id;
        let notes = sqlx::query_as::<_, Note>("SELECT * FROM notes WHERE user_id = $1 ORDER BY id;")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_status(1)?;

        Ok(Response::new(NoteList { notes }))
    }

    async fn update_note(
        &self,
        request: Request<UpdateNoteReq>,
    ) -> ServiceResult<Note> {

        let req_note = request.into_inner();

        sqlx::query("UPDATE notes SET title = $1, text = $2, last_edited = NOW() WHERE id = $3;")
            .bind(req_note.title).bind(req_note.text).bind(req_note.id)
            .execute(&self.pool)
            .await
            .map_status(1)?;

        let updated_note = sqlx::query_as::<_, Note>("SELECT * FROM notes WHERE id = $1;")
            .bind(req_note.id)
            .fetch_one(&self.pool)
            .await
            .map_status(2)?;

        Ok(Response::new(updated_note))
    }

    async fn delete_note(
        &self,
        request: Request<DeleteNoteReq>,
    ) -> ServiceResult<Empty> {

        let delete_id = request.into_inner().id;

        sqlx::query("DELETE FROM notes WHERE id = $1;")
            .bind(delete_id)
            .execute(&self.pool)
            .await
            .map_status(1)?;

        Ok(Response::new(Empty {}))
    }
}
