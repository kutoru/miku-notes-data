use sqlx::{postgres::{PgArguments, PgRow}, prelude::FromRow, PgPool, Postgres, Row};
use tonic::{async_trait, transport::Body, Response, Status};
use tonic_middleware::RequestInterceptor;

use crate::proto::{files::File, notes::Note, shelves::Shelf, tags::Tag};

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

#[derive(FromRow)]
pub struct CountWrapper {
    pub count: i64,
}

#[derive(Clone)]
pub struct Interceptor {
    pub auth_value: String,
}

#[async_trait]
impl RequestInterceptor for Interceptor {
    async fn intercept(
        &self,
        req: tonic::codegen::http::Request<Body>
    ) -> Result<tonic::codegen::http::Request<Body>, Status> {
        println!("Request: {} -> {}", req.method(), req.uri().path());

        match req.headers().get("authorization").map(|v| v.to_str()) {
            Some(Ok(h)) if h == self.auth_value => (),
            _ => return Err(Status::unauthenticated("invalid authorization token")),
        }

        Ok(req)
    }
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

/// finds the first occurence of "()" inside of the `query`,
/// and for the length of the `arr`, pushes Postgres' "$" placeholders into it
pub fn fill_tuple_placeholder<V>(query: &str, arr: &[V], index_offset: usize) -> String {
    let Some(paren_idx) = query.find("()") else {
        return query.to_owned();
    };

    let placeholders_str = (1..=arr.len())
        .map(|i| format!("${}", i + index_offset))
        .collect::<Vec<String>>()
        .join(",");

    let (s, e) = query.split_at(paren_idx + 1);
    let query_str = format!("{s}{placeholders_str}{e}");

    query_str
}

// method on sqlx queries to bind values directly from a slice
pub trait BindIter<'q> {
    fn bind_iter<V>(self, _: V) -> Self
    where
        V: IntoIterator,
        V::Item: 'q + Send + Sync + sqlx::Encode<'q, Postgres> + sqlx::Type<Postgres>,
    ;
}
impl<'q, T> BindIter<'q> for sqlx::query::QueryAs<'q, Postgres, T, PgArguments> {
    fn bind_iter<V>(mut self, iter: V) -> Self
    where
        V: IntoIterator,
        V::Item: 'q + Send + Sync + sqlx::Encode<'q, Postgres> + sqlx::Type<Postgres>,
    {
        for item in iter {
            self = self.bind(item);
        }
        self
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

impl sqlx::FromRow<'_, PgRow> for File {
    fn from_row(row: &'_ PgRow) -> Result<Self, sqlx::Error> {
        Ok(File {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            hash: row.try_get("hash")?,
            name: row.try_get("name")?,
            size: row.try_get("size")?,
            created: row.try_get_unix("created")?,
            attach_id: row.try_get("attach_id").ok(),
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

impl sqlx::FromRow<'_, PgRow> for Shelf {
    fn from_row(row: &'_ PgRow) -> Result<Self, sqlx::Error> {
        Ok(Shelf {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            text: row.try_get("text")?,
            created: row.try_get_unix("created")?,
            last_edited: row.try_get_unix("last_edited")?,
            times_edited: row.try_get("times_edited")?,
            files: vec![],
        })
    }
}
