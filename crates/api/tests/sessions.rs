//! Needs Valkey: `docker compose -f compose.dev.yaml up -d --wait`, or `VALKEY_URL`.
mod common;

use std::error::Error;

use api::{
    sessions::{
        SessionStore,
        store::{INACTIVITY_TIMEOUT, SESSION_LIFETIME, session_key, user_sessions_key},
    },
    users::UserId,
};
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;

async fn raw_connection() -> Result<redis::aio::MultiplexedConnection, Box<dyn Error>> {
    Ok(redis::Client::open(common::valkey_url())?
        .get_multiplexed_async_connection()
        .await?)
}

async fn expire_time(key: &str) -> Result<i64, Box<dyn Error>> {
    Ok(redis::cmd("EXPIRETIME")
        .arg(key)
        .query_async(&mut raw_connection().await?)
        .await?)
}

async fn exists(key: &str) -> Result<bool, Box<dyn Error>> {
    let count: u32 = redis::cmd("EXISTS")
        .arg(key)
        .query_async(&mut raw_connection().await?)
        .await?;
    Ok(count == 1)
}

fn store() -> Result<SessionStore, Box<dyn Error>> {
    Ok(SessionStore::connect(&common::valkey_url())?)
}

#[tokio::test]
async fn valkey_holds_only_the_token_hash() -> Result<(), Box<dyn Error>> {
    let token = store()?
        .create(UserId::generate(), OffsetDateTime::now_utc())
        .await?;

    assert!(exists(&session_key(&token.storage_id())).await?);
    assert!(!exists(&format!("sess:{}", token.to_cookie_value())).await?);
    let stored: Vec<String> = redis::cmd("HVALS")
        .arg(session_key(&token.storage_id()))
        .query_async(&mut raw_connection().await?)
        .await?;
    assert!(
        stored
            .iter()
            .all(|value| !value.contains(&token.to_cookie_value()))
    );
    Ok(())
}

#[tokio::test]
async fn load_returns_the_session_and_records_last_seen_hourly() -> Result<(), Box<dyn Error>> {
    let store = store()?;
    let user_id = UserId::generate();
    let start = OffsetDateTime::now_utc().replace_nanosecond(0)?;
    let token = store.create(user_id, start).await?;

    let soon = store
        .load(&token, start + Duration::minutes(5))
        .await?
        .ok_or("missing")?;
    assert_eq!(soon.user_id, user_id);
    assert_eq!(soon.last_seen_at, start);

    let later = start + Duration::hours(2);
    let refreshed = store.load(&token, later).await?.ok_or("missing")?;
    assert_eq!(refreshed.last_seen_at, later);
    assert_eq!(refreshed.created_at, start);
    assert_eq!(refreshed.last_auth_at, start);
    Ok(())
}

#[tokio::test]
async fn sessions_end_thirty_days_after_login() -> Result<(), Box<dyn Error>> {
    let store = store()?;
    let start = OffsetDateTime::now_utc();
    let token = store.create(UserId::generate(), start).await?;

    assert!(
        store
            .load(&token, start + SESSION_LIFETIME)
            .await?
            .is_none()
    );
    assert!(!exists(&session_key(&token.storage_id())).await?);
    Ok(())
}

#[tokio::test]
async fn sessions_end_after_seven_idle_days_and_are_unlisted() -> Result<(), Box<dyn Error>> {
    let store = store()?;
    let user = UserId::generate();
    let start = OffsetDateTime::now_utc();
    let idle = store.create(user, start).await?;
    let recent = store.create(user, start + Duration::days(1)).await?;

    assert!(
        store
            .load(&idle, start + INACTIVITY_TIMEOUT)
            .await?
            .is_none()
    );
    assert!(!exists(&session_key(&idle.storage_id())).await?);
    let listed: Vec<String> = redis::cmd("SMEMBERS")
        .arg(user_sessions_key(user))
        .query_async(&mut raw_connection().await?)
        .await?;
    assert_eq!(listed, vec![recent.storage_id().as_str().to_owned()]);
    Ok(())
}

#[tokio::test]
async fn activity_keeps_a_session_alive_past_seven_days() -> Result<(), Box<dyn Error>> {
    let store = store()?;
    let start = OffsetDateTime::now_utc();
    let token = store.create(UserId::generate(), start).await?;

    store
        .load(&token, start + Duration::days(6))
        .await?
        .ok_or("missing")?;
    assert!(
        store
            .load(&token, start + Duration::days(12))
            .await?
            .is_some()
    );
    Ok(())
}

#[tokio::test]
async fn valkey_expires_sessions_thirty_days_after_login_and_never_extends_them()
-> Result<(), Box<dyn Error>> {
    let store = store()?;
    let user = UserId::generate();
    let start = OffsetDateTime::now_utc();
    let token = store.create(user, start).await?;
    let key = session_key(&token.storage_id());
    let expires_at = (start + SESSION_LIFETIME).unix_timestamp();

    assert_eq!(expire_time(&key).await?, expires_at);
    assert_eq!(expire_time(&user_sessions_key(user)).await?, expires_at);

    store
        .load(&token, start + Duration::hours(2))
        .await?
        .ok_or("missing")?;
    assert_eq!(expire_time(&key).await?, expires_at);
    Ok(())
}

#[tokio::test]
async fn revoke_all_ends_every_session_of_that_user_only() -> Result<(), Box<dyn Error>> {
    let store = store()?;
    let now = OffsetDateTime::now_utc();
    let user = UserId::generate();
    let other_user = UserId::generate();
    let first = store.create(user, now).await?;
    let second = store.create(user, now).await?;
    let other = store.create(other_user, now).await?;

    store.revoke_all(user).await?;

    assert!(store.load(&first, now).await?.is_none());
    assert!(store.load(&second, now).await?.is_none());
    assert!(!exists(&user_sessions_key(user)).await?);
    assert!(store.load(&other, now).await?.is_some());
    Ok(())
}

#[tokio::test]
async fn revoke_ends_one_session_and_unlists_it() -> Result<(), Box<dyn Error>> {
    let store = store()?;
    let now = OffsetDateTime::now_utc();
    let user = UserId::generate();
    let ended = store.create(user, now).await?;
    let kept = store.create(user, now).await?;

    store.revoke(&ended).await?;

    assert!(store.load(&ended, now).await?.is_none());
    assert!(store.load(&kept, now).await?.is_some());
    let listed: Vec<String> = redis::cmd("SMEMBERS")
        .arg(user_sessions_key(user))
        .query_async(&mut raw_connection().await?)
        .await?;
    assert_eq!(listed, vec![kept.storage_id().as_str().to_owned()]);
    Ok(())
}

#[tokio::test]
async fn logout_revokes_the_session_server_side() -> Result<(), Box<dyn Error>> {
    let state = common::test_state()?;
    let token = state
        .sessions
        .create(UserId::generate(), OffsetDateTime::now_utc())
        .await?;
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/logout")
        .header("sec-fetch-site", "same-origin")
        .header(
            "cookie",
            format!("__Secure-vt_sid={}", token.to_cookie_value()),
        )
        .body(Body::from("{}"))?;

    let response = api::public_router(state.clone()).oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(
        state
            .sessions
            .load(&token, OffsetDateTime::now_utc())
            .await?
            .is_none()
    );
    Ok(())
}

#[tokio::test]
async fn logout_revokes_every_session_cookie_sent() -> Result<(), Box<dyn Error>> {
    let state = common::test_state()?;
    let now = OffsetDateTime::now_utc();
    let planted = state.sessions.create(UserId::generate(), now).await?;
    let own = state.sessions.create(UserId::generate(), now).await?;
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/logout")
        .header("sec-fetch-site", "same-origin")
        .header(
            "cookie",
            format!(
                "__Secure-vt_sid={}; __Secure-vt_sid={}",
                planted.to_cookie_value(),
                own.to_cookie_value()
            ),
        )
        .body(Body::from("{}"))?;

    let response = api::public_router(state.clone()).oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(state.sessions.load(&planted, now).await?.is_none());
    assert!(state.sessions.load(&own, now).await?.is_none());
    Ok(())
}
