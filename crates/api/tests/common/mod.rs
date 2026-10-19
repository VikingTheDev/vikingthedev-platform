#![allow(
    dead_code,
    reason = "each test binary uses a different subset of these helpers"
)]

use std::sync::Arc;

use api::{
    AppState,
    config::Config,
    database,
    sessions::{SessionStore, SessionToken, cookie::CookieDomain},
    users::{self, DisplayName, User},
};
use axum::http::HeaderValue;
use time::OffsetDateTime;

pub const DEFAULT_DATABASE_URL: &str = "postgres://vikingthedev@127.0.0.1:5432/vikingthedev";
pub const DEFAULT_VALKEY_URL: &str = "redis://127.0.0.1:6379";

pub fn database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned())
}

pub fn valkey_url() -> String {
    std::env::var("VALKEY_URL").unwrap_or_else(|_| DEFAULT_VALKEY_URL.to_owned())
}

/// Connections are lazy, so tests that never touch a session or a user need neither store.
pub fn test_state() -> Result<AppState, Box<dyn std::error::Error>> {
    let config = Config {
        database_url: database_url(),
        valkey_url: valkey_url(),
        cookie_domain: CookieDomain::try_from("vikingthe.test".to_owned())?,
        allowed_origins: vec![HeaderValue::from_static("https://vikingthe.test")],
    };
    Ok(AppState {
        database: database::connect(&config.database_url)?,
        sessions: SessionStore::connect(&config.valkey_url)?,
        config: Arc::new(config),
    })
}

/// Needs PostgreSQL: `docker compose -f compose.dev.yaml up -d --wait`, or `DATABASE_URL`.
pub async fn migrated_state() -> Result<AppState, Box<dyn std::error::Error>> {
    database::run_migrations(&database_url()).await?;
    test_state()
}

pub async fn log_in_new_user(
    state: &AppState,
    display_name: &str,
) -> Result<(User, SessionToken), Box<dyn std::error::Error>> {
    let now = OffsetDateTime::now_utc();
    let user = users::create(
        &state.database,
        &DisplayName::try_from(display_name.to_owned())?,
        now,
    )
    .await?;
    let token = state.sessions.create(user.id, now).await?;
    Ok((user, token))
}
