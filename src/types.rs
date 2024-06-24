use sqlx::{postgres::{PgArguments, PgRow}, prelude::FromRow, PgPool, Postgres, Row};
use tonic::{Response, Status};

use crate::proto::{notes::Note, tags::Tag, files::File};

pub type ServiceResult<T> = Result<Response<T>, Status>;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub chunk_size: usize,
}

#[derive(FromRow)]
pub struct IDWrapper {
    pub id: i32,
}

// convert service errors into tonic::Status
pub trait HandleServiceError<T> {
    fn map_to_status(self) -> Result<T, Status>;
}
impl<T> HandleServiceError<T> for Result<T, sqlx::Error> {
    fn map_to_status(self) -> Result<T, Status> {
        self.map_err(|e| {
            println!("DB ERR: {:#?}", e);
            match e {
                sqlx::Error::Database(e) => match e.kind() {
                    sqlx::error::ErrorKind::Other => Status::unknown("Unknown"),
                    _ => Status::invalid_argument("Invalid argument")
                },

                sqlx::Error::RowNotFound => Status::not_found("Not found"),
                _ => Status::unknown("Unknown")
            }
        })
    }
}

// immediately try to convert db's timestamp type into unix ms
trait FieldToUnix {
    fn try_get_unix(&self, field_name: &str) -> Result<i64, sqlx::Error>;
}
impl FieldToUnix for PgRow {
    fn try_get_unix(&self, field_name: &str) -> Result<i64, sqlx::Error> {
        Ok(
            self
                .try_get::<chrono::NaiveDateTime, &str>(field_name)?
                .and_utc()
                .timestamp()
        )
    }
}

// method on sqlx queries to bind values directly from a vector
pub trait BindVec<'a> {
    fn bind_vec<'t, V>(self, vec: &'t Vec<V>) -> Self
    where
        V: std::marker::Sync + sqlx::Encode<'t, Postgres> + sqlx::Type<Postgres>,
        'a: 't,
        't: 'a,
    ;
}
impl<'a, T> BindVec<'a> for sqlx::query::QueryAs<'a, Postgres, T, PgArguments> {
    fn bind_vec<'t, V>(mut self, vec: &'t Vec<V>) -> Self
    where
        V: std::marker::Sync + sqlx::Encode<'t, Postgres> + sqlx::Type<Postgres>,
        'a: 't,
        't: 'a,
    {
        for item in vec.iter() {
            self = self.bind(item);
        }
        self
    }
}

impl sqlx::FromRow<'_, PgRow> for File {
    fn from_row(row: &'_ PgRow) -> Result<Self, sqlx::Error> {
        Ok(File {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            hash: row.try_get("hash")?,
            name: row.try_get("name")?,
            size: row.try_get("size")?,
            created: row.try_get_unix("created")?,
            note_id: row.try_get("note_id").ok(),
        })
    }
}

impl sqlx::FromRow<'_, PgRow> for Tag {
    fn from_row(row: &'_ PgRow) -> Result<Self, sqlx::Error> {
        Ok(Tag {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            name: row.try_get("name")?,
            created: row.try_get_unix("created")?,
            note_id: row.try_get("note_id").ok(),
        })
    }
}

impl sqlx::FromRow<'_, PgRow> for Note {
    fn from_row(row: &'_ PgRow) -> Result<Self, sqlx::Error> {
        Ok(Note {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            title: row.try_get("title")?,
            text: row.try_get("text")?,
            created: row.try_get_unix("created")?,
            last_edited: row.try_get_unix("last_edited")?,
            times_edited: row.try_get("times_edited")?,
            tags: vec![],
            files: vec![],
        })
    }
}
