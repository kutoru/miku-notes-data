use types::AppState;

mod db;
mod proto;
mod types;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let db_url = dotenvy::var("DATABASE_URL")?;
    let addr = dotenvy::var("SERVICE_ADDR")?;
    let chunk_size = dotenvy::var("MAX_FILE_CHUNK_SIZE_IN_MB")?.parse()?;

    let pool = db::get_pool(&db_url).await?;
    db::_reset(&pool).await?;
    db::_test_insert(&pool).await?;

    let state = AppState { pool, chunk_size };

    println!("Listening on {}", addr);
    server::start(state, &addr).await?;

    Ok(())
}
