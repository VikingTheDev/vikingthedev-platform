//! Needs PostgreSQL and Valkey: `docker compose -f compose.dev.yaml up -d --wait`.
mod common;

use std::error::Error;

use api::{sessions::SessionToken, users::UserId};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    response::Response,
};
use serde_json::Value;
use time::OffsetDateTime;
use tower::ServiceExt;

const BODY_LIMIT: usize = 4096;

async fn get(
    router: Router,
    path: &str,
    tokens: &[&SessionToken],
) -> Result<Response, Box<dyn Error>> {
    let builder = tokens.iter().fold(Request::get(path), |builder, token| {
        builder.header(
            header::COOKIE,
            format!("__Secure-vt_sid={}", token.to_cookie_value()),
        )
    });
    Ok(router.oneshot(builder.body(Body::empty())?).await?)
}

fn header_of<'a>(response: &'a Response, name: &str) -> Option<&'a str> {
    response.headers().get(name)?.to_str().ok()
}

async fn json_body(response: Response) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(
        &to_bytes(response.into_body(), BODY_LIMIT).await?,
    )?)
}

#[tokio::test]
async fn me_returns_the_logged_in_user() -> Result<(), Box<dyn Error>> {
    let state = common::migrated_state().await?;
    let (user, token) = common::log_in_new_user(&state, "Ada").await?;

    let response = get(api::public_router(state), "/api/me", &[&token]).await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header_of(&response, "cache-control"),
        Some("private, no-store")
    );
    assert!(header_of(&response, "x-request-id").is_some());
    let body = json_body(response).await?;
    assert_eq!(body["id"], user.id.to_string());
    assert_eq!(body["display_name"], "Ada");
    let created_at = body["created_at"].as_str().ok_or("no created_at")?;
    assert!(created_at.ends_with('Z'), "{created_at}");
    Ok(())
}

#[tokio::test]
async fn me_without_a_session_is_an_unauthenticated_problem() -> Result<(), Box<dyn Error>> {
    let response = get(api::public_router(common::test_state()?), "/api/me", &[]).await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        header_of(&response, "content-type"),
        Some("application/problem+json")
    );
    let request_id = header_of(&response, "x-request-id")
        .ok_or("no X-Request-Id")?
        .to_owned();
    let body = json_body(response).await?;
    assert_eq!(
        body["type"],
        "https://vikingthe.dev/problems/unauthenticated"
    );
    assert_eq!(body["instance"], format!("urn:uuid:{request_id}"));
    Ok(())
}

#[tokio::test]
async fn several_distinct_session_cookies_are_unauthenticated() -> Result<(), Box<dyn Error>> {
    let state = common::migrated_state().await?;
    let (_, own) = common::log_in_new_user(&state, "Ada").await?;
    let (_, planted) = common::log_in_new_user(&state, "Mallory").await?;

    let response = get(api::public_router(state), "/api/me", &[&planted, &own]).await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn a_session_whose_user_is_gone_is_unauthenticated() -> Result<(), Box<dyn Error>> {
    let state = common::migrated_state().await?;
    let token = state
        .sessions
        .create(UserId::generate(), OffsetDateTime::now_utc())
        .await?;

    let response = get(api::public_router(state), "/api/me", &[&token]).await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn internal_session_returns_the_session_user() -> Result<(), Box<dyn Error>> {
    let state = common::migrated_state().await?;
    let (user, token) = common::log_in_new_user(&state, "Ada").await?;

    let response = get(api::internal_router(state), "/session", &[&token]).await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(header_of(&response, "x-request-id").is_some());
    assert_eq!(
        header_of(&response, "cache-control"),
        Some("private, no-store")
    );
    let body = json_body(response).await?;
    assert_eq!(body["user"]["id"], user.id.to_string());
    assert_eq!(body["user"]["display_name"], "Ada");
    assert_eq!(body["role"], "user");
    assert_eq!(body["grants"], Value::Array(Vec::new()));
    Ok(())
}

#[tokio::test]
async fn internal_session_is_never_served_publicly() -> Result<(), Box<dyn Error>> {
    let router = api::public_router(common::test_state()?);
    for path in ["/session", "/api/session"] {
        let response = get(router.clone(), path, &[]).await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        assert!(header_of(&response, "x-request-id").is_some(), "{path}");
    }
    Ok(())
}
