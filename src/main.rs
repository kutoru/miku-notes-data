use types::AppState;

mod db;
mod proto;
mod types;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    if !tokio::fs::try_exists("./files").await? {
        tokio::fs::create_dir("./files").await?;
    }

    let db_url = dotenvy::var("DATABASE_URL")?;
    let service_port = dotenvy::var("SERVICE_PORT")?.parse()?;
    let service_token = dotenvy::var("SERVICE_TOKEN")?;
    let chunk_size = dotenvy::var("MAX_FILE_CHUNK_SIZE")?.parse()?;

    let pool = db::get_pool(&db_url).await?;
    let state = AppState { pool, chunk_size };

    server::start(&state, service_port, &service_token).await?;

    Ok(())
}
