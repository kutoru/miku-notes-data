use crate::proto::notes::notes_server::{Notes, NotesServer};
use crate::proto::notes::{AttachTagReq, CreateNoteReq, DeleteNoteReq, DetachTagReq, Empty, Note, NoteList, ReadNotesReq, UpdateNoteReq};
use crate::proto::{files::File, tags::Tag};
use crate::types::{AppState, BindSlice, HandleServiceError, IDWrapper, ServiceResult};

use tonic::{Request, Response};

pub fn get_service(state: AppState) -> NotesServer<AppState> {
    NotesServer::new(state)
}

/// Finds the first occurence of "()" inside of the query and pushes Postgres' "$" placeholders into it
fn fill_tuple_placeholder<V>(query: &str, vec: &Vec<V>, index_offset: usize) -> String {
    let Some(paren_idx) = query.find("()") else {
        return query.to_owned();
    };

    let placeholders_str = (1..=vec.len())
        .map(|i| format!("${}", i + index_offset))
        .collect::<Vec<String>>()
        .join(",");

    let (s, e) = query.split_at(paren_idx + 1);
    let query_str = format!("{s}{placeholders_str}{e}");

    query_str
}

#[tonic::async_trait]
impl Notes for AppState {
    async fn create_note(
        &self,
        request: Request<CreateNoteReq>,
    ) -> ServiceResult<Note> {

        let req_body = request.into_inner();

        let new_note = sqlx::query_as::<_, Note>("INSERT INTO notes (user_id, title, text) VALUES ($1, $2, $3) RETURNING *;")
            .bind(req_body.user_id).bind(req_body.title).bind(req_body.text)
            .fetch_one(&self.pool)
            .await
            .map_to_status()?;

        Ok(Response::new(new_note))
    }

    async fn read_notes(
        &self,
        request: Request<ReadNotesReq>,
    ) -> ServiceResult<NoteList> {

        let req_body = request.into_inner();

        // fetching notes

        let mut transaction = self.pool
            .begin()
            .await
            .map_to_status()?;

        let mut notes = sqlx::query_as::<_, Note>("SELECT * FROM notes WHERE user_id = $1 ORDER BY id;")
            .bind(req_body.user_id)
            .fetch_all(&mut *transaction)
            .await
            .map_to_status()?;

        let note_ids: Vec<_> = notes.iter().map(|n| n.id).collect();

        if note_ids.is_empty() {
            return Ok(Response::new(NoteList { notes: vec![] }));
        }

        // fetching relevant tags and files

        let mut tags = sqlx::query_as::<_, Tag>(&fill_tuple_placeholder(
            r"
                SELECT t.*, nt.note_id FROM tags AS t
                INNER JOIN note_tags AS nt
                ON nt.note_id IN () AND nt.tag_id = t.id
                ORDER BY nt.note_id ASC, t.id DESC;
            ",
            &note_ids, 0,
        ))
            .bind_slice(&note_ids)
            .fetch_all(&mut *transaction)
            .await
            .map_to_status()?;

        let mut files = sqlx::query_as::<_, File>(&fill_tuple_placeholder(
            r"
                SELECT f.*, nf.note_id FROM files AS f
                INNER JOIN note_files AS nf
                ON nf.note_id IN () AND nf.file_id = f.id
                ORDER BY nf.note_id ASC, f.id DESC;
            ",
            &note_ids, 0,
        ))
            .bind_slice(&note_ids)
            .fetch_all(&mut *transaction)
            .await
            .map_to_status()?;

        transaction
            .commit()
            .await
            .map_to_status()?;

        // assinging tags and files to their respective notes.
        // since all the arrays are sorted by note id,
        // in theory this implementation iterates through each loop only once

        for note in notes.iter_mut().rev() {

            while !tags.is_empty() {
                let note_id = tags[tags.len() - 1].note_id
                    .ok_or(tonic::Status::internal("Could not get a note id from a tag"))?;

                if note_id == note.id {
                    note.tags.push(tags.pop().unwrap());
                } else {
                    break;
                }
            }

            while !files.is_empty() {
                let note_id = files[files.len() - 1].note_id
                    .ok_or(tonic::Status::internal("Could not get a note id from a file"))?;

                if note_id == note.id {
                    note.files.push(files.pop().unwrap());
                } else {
                    break;
                }
            }

        }

        Ok(Response::new(NoteList { notes }))
    }

    async fn update_note(
        &self,
        request: Request<UpdateNoteReq>,
    ) -> ServiceResult<Note> {

        let req_body = request.into_inner();

        let updated_note = sqlx::query_as::<_, Note>(r"
            UPDATE notes
            SET title = $1, text = $2, last_edited = NOW(), times_edited = times_edited + 1
            WHERE id = $3 AND user_id = $4
            RETURNING *;
        ")
            .bind(req_body.title).bind(req_body.text).bind(req_body.id).bind(req_body.user_id)
            .fetch_one(&self.pool)
            .await
            .map_to_status()?;

        Ok(Response::new(updated_note))
    }

    async fn delete_note(
        &self,
        request: Request<DeleteNoteReq>,
    ) -> ServiceResult<Empty> {

        let req_body = request.into_inner();

        let mut transaction = self.pool
            .begin()
            .await
            .map_to_status()?;

        // deleting tag and file relations

        sqlx::query(r"
            DELETE FROM note_tags AS nt
            USING notes AS n
            WHERE n.id = nt.note_id AND n.id = $1 AND n.user_id = $2;
        ")
            .bind(req_body.id).bind(req_body.user_id)
            .execute(&mut *transaction)
            .await
            .map_to_status()?;

        let file_ids: Vec<_> = sqlx::query_as::<_, IDWrapper>(r"
            DELETE FROM note_files AS nf
            USING notes AS n
            WHERE n.id = nf.note_id AND n.id = $1 AND n.user_id = $2
            RETURNING nf.file_id AS id;
        ")
            .bind(req_body.id).bind(req_body.user_id)
            .fetch_all(&mut *transaction)
            .await
            .map_to_status()?
            .into_iter()
            .map(|w| w.id)
            .collect();

        // deleting related files from the db

        let files = match file_ids.len() {
            0 => vec![],
            _ => sqlx::query_as::<_, File>(&fill_tuple_placeholder(
                r"
                    DELETE FROM files
                    WHERE user_id = $1 AND id IN ()
                    RETURNING *;
                ",
                &file_ids, 1,
            ))
                .bind(req_body.user_id).bind_slice(&file_ids)
                .fetch_all(&mut *transaction)
                .await
                .map_to_status()?,
        };

        // deleting the note itself

        sqlx::query("DELETE FROM notes WHERE id = $1 AND user_id = $2;")
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

        // deleting related files from the disk

        for file in &files {
            let file_path = "./files/".to_owned() + &file.hash;
            tokio::fs::remove_file(file_path)
                .await
                .unwrap_or_else(|e| println!("Could not delete a file: {:?};\nBecause error: {:?};", file, e));
        }

        Ok(Response::new(Empty {}))
    }

    async fn attach_tag(
        &self,
        request: Request<AttachTagReq>,
    ) -> ServiceResult<Empty> {

        let req_body = request.into_inner();

        // trying to insert a new note-tag relation while making sure
        // that both the note and the tag belong to the user

        sqlx::query(r"
            INSERT INTO note_tags (note_id, tag_id)
            SELECT (
                SELECT id FROM notes WHERE id = $1 AND user_id = $3
            ), (
                SELECT id FROM tags WHERE id = $2 AND user_id = $3
            );
        ")
            .bind(req_body.note_id).bind(req_body.tag_id).bind(req_body.user_id)
            .execute(&self.pool)
            .await
            .map_to_status()?;

        Ok(Response::new(Empty {}))
    }

    async fn detach_tag(
        &self,
        request: Request<DetachTagReq>,
    ) -> ServiceResult<Empty> {

        let req_body = request.into_inner();

        let mut transaction = self.pool
            .begin()
            .await
            .map_to_status()?;

        sqlx::query(r"
            DELETE FROM note_tags
            WHERE note_id = (
                SELECT id FROM notes WHERE id = $1 AND user_id = $3
            ) AND tag_id = (
                SELECT id FROM tags WHERE id = $2 AND user_id = $3
            );
        ")
            .bind(req_body.note_id).bind(req_body.tag_id).bind(req_body.user_id)
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
