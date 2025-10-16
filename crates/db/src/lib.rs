use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;

pub type DbPool = Arc<PgPool>;

pub async fn make_pool(database_url: &str, max_connections: u32) -> anyhow::Result<DbPool> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await?;

    Ok(Arc::new(pool))
}