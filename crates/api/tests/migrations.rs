//! Needs PostgreSQL with a role that may create databases, as in `compose.dev.yaml`.
mod common;

use std::{error::Error, time::Duration};

use api::database::MIGRATION_LOCK_KEY;
use diesel::{QueryableByName, sql_query, sql_types::BigInt};
use diesel_async::{AsyncConnection as _, AsyncPgConnection, RunQueryDsl as _};
use uuid::Uuid;

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

// Production's DATABASE_URL carries a server-side statement_timeout (step 6a review).
const SHORT_STATEMENT_TIMEOUT: &str = "?options=-c%20statement_timeout%3D100";
const LOCK_HELD_FOR: Duration = Duration::from_millis(400);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const BLOCKED_WITHIN: Duration = Duration::from_secs(5);

struct FreshDatabase {
    admin: AsyncPgConnection,
    name: String,
    url: String,
}

impl FreshDatabase {
    async fn create() -> Result<Self, Box<dyn Error>> {
        let admin_url = common::database_url();
        let (server_url, _) = admin_url
            .rsplit_once('/')
            .ok_or("DATABASE_URL has no database name")?;
        let name = format!("migrations_test_{}", Uuid::now_v7().simple());
        let url = format!("{server_url}/{name}");
        let mut admin = AsyncPgConnection::establish(&admin_url).await?;
        sql_query(format!("CREATE DATABASE {name}"))
            .execute(&mut admin)
            .await?;
        Ok(Self { admin, name, url })
    }

    async fn remove(mut self) -> Result<(), Box<dyn Error>> {
        sql_query(format!("DROP DATABASE {} WITH (FORCE)", self.name))
            .execute(&mut self.admin)
            .await?;
        Ok(())
    }
}

#[tokio::test]
async fn api_processes_starting_together_apply_each_migration_once() -> Result<(), Box<dyn Error>> {
    let database = FreshDatabase::create().await?;

    let (first, second) = tokio::join!(
        api::database::run_migrations(&database.url),
        api::database::run_migrations(&database.url)
    );
    let applied = count_applied_migrations(&database.url).await;
    database.remove().await?;

    first?;
    second?;
    assert_eq!(applied?, 1);
    Ok(())
}

#[tokio::test]
async fn migrations_outlast_a_server_side_statement_timeout() -> Result<(), Box<dyn Error>> {
    let database = FreshDatabase::create().await?;
    let mut lock_holder = AsyncPgConnection::establish(&database.url).await?;
    sql_query("SELECT pg_advisory_lock($1)")
        .bind::<BigInt, _>(MIGRATION_LOCK_KEY)
        .execute(&mut lock_holder)
        .await?;

    let timed_out_url = format!("{}{SHORT_STATEMENT_TIMEOUT}", database.url);
    let migrating =
        tokio::spawn(async move { api::database::run_migrations(&timed_out_url).await });
    tokio::time::timeout(BLOCKED_WITHIN, wait_until_blocked_on_lock(&mut lock_holder)).await??;
    tokio::time::sleep(LOCK_HELD_FOR).await;
    sql_query("SELECT pg_advisory_unlock($1)")
        .bind::<BigInt, _>(MIGRATION_LOCK_KEY)
        .execute(&mut lock_holder)
        .await?;
    let migrated = migrating.await?;
    drop(lock_holder);
    database.remove().await?;

    migrated?;
    Ok(())
}

// Starts the clock only once the migration is really queued behind the lock, so the test can't pass without it.
async fn wait_until_blocked_on_lock(
    connection: &mut AsyncPgConnection,
) -> Result<(), Box<dyn Error>> {
    loop {
        let waiting: Count = sql_query(
            "SELECT count(*) AS count FROM pg_locks \
             WHERE locktype = 'advisory' AND NOT granted \
             AND database = (SELECT oid FROM pg_database WHERE datname = current_database())",
        )
        .get_result(connection)
        .await?;
        if waiting.count > 0 {
            return Ok(());
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

async fn count_applied_migrations(database_url: &str) -> Result<i64, Box<dyn Error>> {
    let applied: Count = sql_query("SELECT count(*) AS count FROM __diesel_schema_migrations")
        .get_result(&mut AsyncPgConnection::establish(database_url).await?)
        .await?;
    Ok(applied.count)
}
