use crate::proto::notes::notes_server::{Notes, NotesServer};
use crate::proto::notes::{CreateNoteReq, ReadNotesReq, UpdateNoteReq, DeleteNoteReq, Note, NoteList, Empty};
use crate::proto::{files::File, tags::Tag};
use crate::types::{AppState, BindVec, HandleServiceError, IDWrapper, ServiceResult};

use tonic::{Request, Response};

pub fn get_service(state: AppState) -> NotesServer<AppState> {
    NotesServer::new(state)
}

/// Finds the first occurence of "()" inside of the query and pushes Postgres' "$" placeholders into it
fn fill_tuple_placeholder<V>(query: &str, vec: &Vec<V>, index_offset: usize) -> String {
    let paren_idx = match query.find("()") {
        Some(index) => index,
        None => return query.to_owned(),
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

        // fetching relevant tags and files

        let mut tags = sqlx::query_as::<_, Tag>(&fill_tuple_placeholder(
            r#"
                SELECT t.*, nt.note_id FROM tags AS t
                INNER JOIN note_tags AS nt
                ON nt.note_id IN () AND nt.tag_id = t.id
                ORDER BY nt.note_id ASC, t.id DESC;
            "#,
            &note_ids, 0,
        ))
            .bind_vec(&note_ids)
            .fetch_all(&mut *transaction)
            .await
            .map_to_status()?;

        let mut files = sqlx::query_as::<_, File>(&fill_tuple_placeholder(
            r#"
                SELECT f.*, nf.note_id FROM files AS f
                INNER JOIN note_files AS nf
                ON nf.note_id IN () AND nf.file_id = f.id
                ORDER BY nf.note_id ASC, f.id DESC;
            "#,
            &note_ids, 0,
        ))
            .bind_vec(&note_ids)
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

            while tags.len() > 0 {
                let note_id = tags[tags.len() - 1].note_id
                    .ok_or(tonic::Status::internal("Could not get a note id from a tag"))?;

                if note_id == note.id {
                    note.tags.push(tags.pop().unwrap());
                } else {
                    break;
                }
            }

            while files.len() > 0 {
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

        let updated_note = sqlx::query_as::<_, Note>("UPDATE notes SET title = $1, text = $2, last_edited = NOW() WHERE id = $3 AND user_id = $4 RETURNING *;")
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

        sqlx::query(r#"
            DELETE FROM note_tags AS nt
            USING notes AS n
            WHERE n.id = nt.note_id AND n.id = $1 AND n.user_id = $2;
        "#)
            .bind(req_body.id).bind(req_body.user_id)
            .execute(&mut *transaction)
            .await
            .map_to_status()?;

        let file_ids: Vec<_> = sqlx::query_as::<_, IDWrapper>(r#"
            DELETE FROM note_files AS nf
            USING notes AS n
            WHERE n.id = nf.note_id AND n.id = $1 AND n.user_id = $2
            RETURNING nf.file_id AS id;
        "#)
            .bind(req_body.id).bind(req_body.user_id)
            .fetch_all(&mut *transaction)
            .await
            .map_to_status()?
            .into_iter()
            .map(|w| w.id)
            .collect();

        let files = match file_ids.len() {
            l if l == 0 => vec![],
            _ => sqlx::query_as::<_, File>(&fill_tuple_placeholder(
                r#"
                DELETE FROM files
                WHERE user_id = $1 AND id IN ()
                RETURNING *;
                "#,
                &file_ids, 1,
            ))
                .bind(req_body.user_id).bind_vec(&file_ids)
                .fetch_all(&mut *transaction)
                .await
                .map_to_status()?,
        };

        sqlx::query("DELETE FROM notes WHERE id = $1 AND user_id = $2;")
            .bind(req_body.id).bind(req_body.user_id)
            .execute(&mut *transaction)
            .await
            .map_to_status()?
            .rows_affected()
            .ge(&1)
            .then_some(())
            .ok_or_else(|| sqlx::Error::RowNotFound)
            .map_to_status()?;

        transaction
            .commit()
            .await
            .map_to_status()?;

        // after successfull transaction, make sure to delete the files from the disk
        for file in files.iter() {
            println!("*delete file with name \"{}\"*", file.hash)
        }

        Ok(Response::new(Empty {}))
    }
}
