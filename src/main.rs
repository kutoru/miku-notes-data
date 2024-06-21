mod db;
mod proto;
mod models;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Start");

    let db_url = dotenvy::var("DATABASE_URL")?;
    let addr = dotenvy::var("SERVICE_ADDR")?;

    let pool = db::get_pool(&db_url).await?;
    db::_reset(&pool).await?;
    db::_test_insert(&pool).await?;

    println!("Listening on {}", addr);
    server::start(pool, &addr).await?;

    println!("End");

    Ok(())
}
