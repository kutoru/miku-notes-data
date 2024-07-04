use crate::proto::notes::notes_server::{Notes, NotesServer};
use crate::proto::notes::sort;
use crate::proto::notes::{AttachTagReq, CreateNoteReq, DeleteNoteReq, DetachTagReq, Empty, Note, NoteList, ReadNotesReq, UpdateNoteReq};
use crate::proto::{files::File, tags::Tag};
use crate::types::{AppState, BindIter, HandleServiceError, IDWrapper, ServiceResult};

use helpers::*;
use tonic::{Request, Response};

mod helpers;

pub fn get_service(state: AppState) -> NotesServer<AppState> {
    NotesServer::new(state)
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
        println!("READ NOTES BODY: {:#?}", req_body);

        // extracting parameters from the body and building the query

        let user_id = req_body.user_id;
        let pagination = req_body.pagination.unwrap();
        let sort = req_body.sort.unwrap();
        let filters = req_body.filters.unwrap();

        let (query_str, count_str) = dbg!(build_read_notes_query_strs(&sort, &filters));
        let (query, count_query) = build_read_notes_queries(&query_str, &count_str, user_id, &pagination, &filters);

        // executing the two queries

        let mut transaction = self.pool
            .begin()
            .await
            .map_to_status()?;

        let mut notes = query
            .fetch_all(&mut *transaction)
            .await
            .map_to_status()?;

        let total_count = count_query
            .fetch_one(&mut *transaction)
            .await
            .map_to_status()?
            .count as i32;

        let note_ids: Vec<_> = notes.iter().map(|n| n.id).collect();

        if note_ids.is_empty() {
            return Ok(Response::new(NoteList { notes: Vec::new(), total_count }));
        }

        // fetching relevant tags and files

        let attachment_sort_field = match sort.sort_field() {
            sort::Field::Date => SortField::CREATED,
            sort::Field::DateModif => SortField::LAST_EDITED,
            sort::Field::Title => SortField::TITLE,
        };

        // the type here is reversed on purpose. specifically, it allows
        // efficient assignment of attachments to their respective notes
        let attachment_sort_type = match sort.sort_type() {
            sort::Type::Asc => SortType::DESC,
            sort::Type::Desc => SortType::ASC,
        };

        let mut tags = sqlx::query_as::<_, Tag>(&fill_tuple_placeholder(
            &format!(r"
                SELECT t.*, n.id AS note_id FROM tags AS t
                INNER JOIN note_tags AS nt ON nt.tag_id = t.id
                INNER JOIN notes AS n ON nt.note_id = n.id
                WHERE n.id IN ()
                ORDER BY n.{} {}, n.id ASC, t.id DESC;
            ", attachment_sort_field, attachment_sort_type),
            &note_ids, 0,
        ))
            .bind_iter(&note_ids)
            .fetch_all(&mut *transaction)
            .await
            .map_to_status()?;

        let mut files = sqlx::query_as::<_, File>(&fill_tuple_placeholder(
            &format!(r"
                SELECT f.*, n.id AS attach_id FROM files AS f
                INNER JOIN note_files AS nf ON nf.file_id = f.id
                INNER JOIN notes AS n ON nf.note_id = n.id
                WHERE n.id IN ()
                ORDER BY n.{} {}, n.id ASC, f.id DESC;
            ", attachment_sort_field, attachment_sort_type),
            &note_ids, 0,
        ))
            .bind_iter(&note_ids)
            .fetch_all(&mut *transaction)
            .await
            .map_to_status()?;

        transaction
            .commit()
            .await
            .map_to_status()?;

        // assinging tags and files to their respective notes.
        // since the arrays are properly sorted, this implementation
        // "iterates" through each of these three arrays only once

        let mut tag_note_id = match tags.last() {
            Some(t) => t.note_id.unwrap(),
            None => 0,
        };

        let mut file_note_id = match files.last() {
            Some(f) => f.attach_id.unwrap(),
            None => 0,
        };

        for note in &mut notes {

            while !tags.is_empty() && tag_note_id == note.id {
                note.tags.push(tags.pop().unwrap());

                if let Some(t) = tags.last() {
                    tag_note_id = t.note_id.unwrap();
                }
            }

            while !files.is_empty() && file_note_id == note.id {
                note.files.push(files.pop().unwrap());

                if let Some(f) = files.last() {
                    file_note_id = f.attach_id.unwrap();
                }
            }

        }

        // delete this later
        assert_eq!(tags, Vec::new());
        assert_eq!(files, Vec::new());

        Ok(Response::new(NoteList { notes, total_count }))
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
            0 => Vec::new(),
            _ => sqlx::query_as::<_, File>(&fill_tuple_placeholder(
                r"
                    DELETE FROM files
                    WHERE user_id = $1 AND id IN ()
                    RETURNING *;
                ",
                &file_ids, 1,
            ))
                .bind(req_body.user_id).bind_iter(&file_ids)
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
