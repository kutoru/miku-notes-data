use futures::FutureExt;
use types::AppState;

mod db;
mod proto;
mod types;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let db_url = dotenvy::var("DATABASE_URL")?;
    let grpc_addr = dotenvy::var("GRPC_SERVICE_ADDR")?;
    let file_addr = dotenvy::var("FILE_SERVICE_ADDR")?;
    let chunk_size = dotenvy::var("MAX_FILE_CHUNK_SIZE_IN_MB")?.parse()?;

    if !tokio::fs::try_exists("./files").await? {
        tokio::fs::create_dir("./files").await?;
    }

    let pool = db::get_pool(&db_url).await?;
    db::_reset(&pool).await?;
    db::_test_insert(&pool).await?;

    let state = AppState { pool, chunk_size };

    futures::future::try_join_all([
        server::start_grpc_server(&state, &grpc_addr).boxed(),
        server::start_file_server(&state, &file_addr).boxed(),
    ]).await?;

    Ok(())
}
