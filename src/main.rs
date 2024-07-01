use types::AppState;

mod db;
mod proto;
mod types;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let db_url = dotenvy::var("DATABASE_URL")?;
    let service_addr = dotenvy::var("SERVICE_ADDR")?;
    let service_token = dotenvy::var("SERVICE_TOKEN")?;
    let chunk_size = dotenvy::var("MAX_FILE_CHUNK_SIZE")?.parse()?;

    if !tokio::fs::try_exists("./files").await? {
        tokio::fs::create_dir("./files").await?;
    }

    let pool = db::get_pool(&db_url).await?;
    // db::_reset(&pool).await?;
    // db::_test_insert(&pool).await?;

    let state = AppState { pool, chunk_size };

    server::start(&state, &service_addr, &service_token).await?;

    Ok(())
}
