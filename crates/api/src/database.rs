use std::time::Duration;

use anyhow::anyhow;
use diesel::{Connection as _, RunQueryDsl as _, sql_query, sql_types::BigInt};
use diesel_async::{
    AsyncPgConnection,
    async_connection_wrapper::AsyncConnectionWrapper,
    pooled_connection::{
        AsyncDieselConnectionManager,
        deadpool::{BuildError, Pool, PoolError},
    },
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

pub type DatabasePool = Pool<AsyncPgConnection>;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
// Any fixed key works, as long as every API process uses the same one.
const MIGRATION_LOCK_KEY: i64 = 0x7669_6b69_6e67;
const MAX_CONNECTIONS: usize = 16;
// deadpool has no default timeouts, so a stalled database would otherwise hang requests forever.
const QUERY_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("no database connection: {0}")]
    Pool(#[from] PoolError),
    #[error("database query failed: {0}")]
    Query(#[from] diesel::result::Error),
    #[error("database did not answer within {QUERY_TIMEOUT:?}")]
    Timeout,
}

/// Connects lazily: connections open on first use.
///
/// # Errors
/// Fails if the pool can't be configured.
pub fn connect(database_url: &str) -> Result<DatabasePool, BuildError> {
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    Pool::builder(manager).max_size(MAX_CONNECTIONS).build()
}

/// Bounds a connection checkout and its queries by `QUERY_TIMEOUT`.
///
/// # Errors
/// Fails with `Timeout` when the deadline passes, or with the query's own error.
pub async fn within_timeout<T>(
    query: impl Future<Output = Result<T, DatabaseError>>,
) -> Result<T, DatabaseError> {
    tokio::time::timeout(QUERY_TIMEOUT, query)
        .await
        .map_err(|_| DatabaseError::Timeout)?
}

/// Applies pending migrations under an advisory lock, so API processes starting together apply each one once (D16).
///
/// # Errors
/// Fails if the database is unreachable or a migration fails.
pub async fn run_migrations(database_url: &str) -> anyhow::Result<()> {
    let database_url = database_url.to_owned();
    tokio::task::spawn_blocking(move || {
        let mut connection = AsyncConnectionWrapper::<AsyncPgConnection>::establish(&database_url)?;
        sql_query("SELECT pg_advisory_lock($1)")
            .bind::<BigInt, _>(MIGRATION_LOCK_KEY)
            .execute(&mut connection)?;
        let applied = connection
            .run_pending_migrations(MIGRATIONS)
            .map(|_| ())
            .map_err(|error| anyhow!(error));
        let unlocked = sql_query("SELECT pg_advisory_unlock($1)")
            .bind::<BigInt, _>(MIGRATION_LOCK_KEY)
            .execute(&mut connection);
        applied?;
        unlocked?;
        Ok(())
    })
    .await?
}
