//! Needs PostgreSQL with a role that may create databases, as in `compose.dev.yaml`.
mod common;

use std::error::Error;

use diesel::{QueryableByName, sql_query, sql_types::BigInt};
use diesel_async::{AsyncConnection as _, AsyncPgConnection, RunQueryDsl as _};
use uuid::Uuid;

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[tokio::test]
async fn api_processes_starting_together_apply_each_migration_once() -> Result<(), Box<dyn Error>> {
    let admin_url = common::database_url();
    let (server_url, _) = admin_url
        .rsplit_once('/')
        .ok_or("DATABASE_URL has no database name")?;
    let name = format!("migrations_test_{}", Uuid::now_v7().simple());
    let fresh_url = format!("{server_url}/{name}");
    let mut admin = AsyncPgConnection::establish(&admin_url).await?;
    sql_query(format!("CREATE DATABASE {name}"))
        .execute(&mut admin)
        .await?;

    let (first, second) = tokio::join!(
        api::database::run_migrations(&fresh_url),
        api::database::run_migrations(&fresh_url)
    );
    let applied = count_applied_migrations(&fresh_url).await;
    sql_query(format!("DROP DATABASE {name} WITH (FORCE)"))
        .execute(&mut admin)
        .await?;

    first?;
    second?;
    assert_eq!(applied?, 1);
    Ok(())
}

async fn count_applied_migrations(database_url: &str) -> Result<i64, Box<dyn Error>> {
    let applied: Count = sql_query("SELECT count(*) AS count FROM __diesel_schema_migrations")
        .get_result(&mut AsyncPgConnection::establish(database_url).await?)
        .await?;
    Ok(applied.count)
}
