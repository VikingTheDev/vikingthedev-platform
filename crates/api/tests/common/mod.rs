use std::sync::Arc;

use api::{
    AppState,
    config::Config,
    sessions::{SessionStore, cookie::CookieDomain},
};
use axum::http::HeaderValue;

pub const DEFAULT_VALKEY_URL: &str = "redis://127.0.0.1:6379";

pub fn valkey_url() -> String {
    std::env::var("VALKEY_URL").unwrap_or_else(|_| DEFAULT_VALKEY_URL.to_owned())
}

/// The connection is lazy, so tests that never touch a session don't need Valkey.
pub fn test_state() -> Result<AppState, Box<dyn std::error::Error>> {
    let config = Config {
        valkey_url: valkey_url(),
        cookie_domain: CookieDomain::try_from("vikingthe.test".to_owned())?,
        allowed_origins: vec![HeaderValue::from_static("https://vikingthe.test")],
    };
    Ok(AppState {
        sessions: SessionStore::connect(&config.valkey_url)?,
        config: Arc::new(config),
    })
}
