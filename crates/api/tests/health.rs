use std::error::Error;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use serde_json::Value;
use tower::ServiceExt;

const BODY_LIMIT: usize = 4096;

async fn get_health(
    router: Router,
    path: &str,
) -> Result<(StatusCode, Option<String>, Value), Box<dyn Error>> {
    let response = router
        .oneshot(Request::get(path).body(Body::empty())?)
        .await?;
    let status = response.status();
    let cache_control = response
        .headers()
        .get(header::CACHE_CONTROL)
        .map(|value| value.to_str().map(str::to_owned))
        .transpose()?;
    let body = serde_json::from_slice(&to_bytes(response.into_body(), BODY_LIMIT).await?)?;
    Ok((status, cache_control, body))
}

#[tokio::test]
async fn public_health_reports_ok_and_is_never_cached() -> Result<(), Box<dyn Error>> {
    let (status, cache_control, body) = get_health(api::public_router(), "/api/health").await?;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(cache_control.as_deref(), Some("no-store"));
    assert_eq!(body["status"], "ok");
    assert!(body["sha"].as_str().is_some_and(|sha| !sha.is_empty()));
    Ok(())
}

#[tokio::test]
async fn internal_health_reports_ok() -> Result<(), Box<dyn Error>> {
    let (status, _, body) = get_health(api::internal_router(), "/health").await?;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    Ok(())
}
