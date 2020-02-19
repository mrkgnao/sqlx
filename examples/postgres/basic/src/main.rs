use sqlx::{Connect, Connection, Cursor, Executor, PgConnection, Row};

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let mut conn = PgConnection::connect("postgres://").await?;

    let row1 = sqlx::query("SELECT 1, 2, 3").fetch_one(&mut conn).await?;

    let v0 = row1.try_get(0)?;
    let v1 = row1.try_get(1)?;
    let v2 = row1.try_get(2)?;

    let row2 = sqlx::query("SELECT 'Hello World'")
        .fetch_one(&mut conn)
        .await?;

    conn.close().await?;

    Ok(())
}
