use anyhow::anyhow;
use axum::{extract::{Path, State}, http::{header, HeaderMap, StatusCode}, response::IntoResponse, routing::get, Router, body::Body};
use tokio_util::io::ReaderStream;

use crate::{proto::files::File, types::AppState};

pub fn get_router(state: &AppState) -> Router {
    Router::new()
        .route("/files/:hash", get(files_get))
        .with_state(state.clone())
}

fn is_authenticated(state: &AppState, headers: &HeaderMap) -> anyhow::Result<()> {
    let header_value = headers.get("authorization")
        .ok_or(anyhow!("Missing authorization in the headers"))?;
    let value_str = header_value.to_str()?;
    let value_split: Vec<_> = value_str.split(' ').collect();

    (!(
        value_split.len() != 2 ||
        value_split[0] != "Bearer:" ||
        // value_split[1] != state.service_token
        value_split[1] != "awawaw"
    ))
        .then_some(())
        .ok_or(anyhow!("Authorization value is invalid"))
}

fn get_header_user_id(headers: &HeaderMap) -> anyhow::Result<i32> {
    let header_value = headers.get("user-id")
        .ok_or(anyhow!("Missing user-id in the headers"))?;
    let value_str = header_value.to_str()?;
    let user_id = value_str.parse()?;
    Ok(user_id)
}

async fn files_get(
    State(state): State<AppState>,
    Path(file_hash): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {

    println!("files_get with hash: {}", file_hash);

    // checking the headers

    if is_authenticated(&state, &headers).is_err() {
        return Err(StatusCode::NOT_FOUND);
    }

    let user_id = get_header_user_id(&headers)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // checking the file

    let file_info = sqlx::query_as::<_, File>("SELECT * FROM files WHERE hash = $1 AND user_id = $2;")
        .bind(&file_hash).bind(user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // preparing and sending the request

    let content_type = mime_guess::from_path(&file_info.name).first_raw()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let file_path = "./files/".to_owned() + &file_hash;
    let file = tokio::fs::File::open(file_path).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let headers = [
        (
            header::CONTENT_TYPE,
            content_type,
        ),
        (
            header::CONTENT_DISPOSITION,
            &format!("attachment; filename=\"{}\"", file_info.name),
        ),
    ];

    Ok((headers, body).into_response())
}
