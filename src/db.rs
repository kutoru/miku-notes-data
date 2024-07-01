use sqlx::{postgres::PgPoolOptions, PgPool};
use anyhow::{Result, anyhow};
use tokio::fs;

pub async fn get_pool(db_url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await
        .map_err(|e| anyhow!(e))
}

async fn _run_script(pool: &PgPool, script_path: &str) -> Result<()> {
    let file = fs::read_to_string(script_path).await?;
    let mut transaction = pool.begin().await?;

    for query in file.split(';') {
        sqlx::query(query).execute(&mut *transaction).await?;
    }

    transaction.commit().await?;
    Ok(())
}

pub async fn _reset(pool: &PgPool) -> Result<()> {
    println!("Resetting the DB");
    _run_script(pool, "./migrations/reset.sql").await?;

    println!("Clearing the files directory");
    tokio::fs::remove_dir_all("./files").await?;
    tokio::fs::create_dir("./files").await?;

    Ok(())
}

pub async fn _test_insert(pool: &PgPool) -> Result<()> {
    println!("Inserting test data into the DB");
    _run_script(pool, "./migrations/test_insert.sql").await
}
