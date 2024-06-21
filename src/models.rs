use sqlx::{postgres::PgRow, Row};
use tonic::{Response, Status, Code};
use crate::proto::{notes::Note, tags::Tag, files::File};

pub type ServiceResult<T> = Result<Response<T>, Status>;

// convert potential service errors into tonic::Status
pub trait HandleServiceError<T, E: std::fmt::Debug> {
    fn map_status(self, code: isize) -> Result<T, Status>;
}
impl<T, E: std::fmt::Debug> HandleServiceError<T, E> for Result<T, E> {
    fn map_status(self, code: isize) -> Result<T, Status> {
        self.map_err(|e| { Status::new(Code::Unknown, format!("Code: {}; {:#?}", code, e)) })
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

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct IDWrapper {
    pub id: i32,
}

impl sqlx::FromRow<'_, PgRow> for File {
    fn from_row(row: &'_ PgRow) -> Result<Self, sqlx::Error> {
        Ok(File {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            hash: row.try_get("hash")?,
            ext: row.try_get("ext")?,
            name: row.try_get("name")?,
            size: row.try_get("size")?,
            created: row.try_get_unix("created")?,
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
