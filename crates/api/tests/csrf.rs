mod common;

use std::error::Error;

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use serde_json::Value;
use tower::ServiceExt;

const BODY_LIMIT: usize = 4096;

async fn send(request: Request<Body>) -> Result<axum::response::Response, Box<dyn Error>> {
    Ok(api::public_router(common::test_state()?)
        .oneshot(request)
        .await?)
}

fn logout_request(headers: &[(&str, &str)]) -> Result<Request<Body>, Box<dyn Error>> {
    let builder = headers.iter().fold(
        Request::builder()
            .method(Method::POST)
            .uri("/api/auth/logout"),
        |builder, (name, value)| builder.header(*name, *value),
    );
    Ok(builder.body(Body::from("{}"))?)
}

#[tokio::test]
async fn same_origin_logout_succeeds_and_expires_the_cookie() -> Result<(), Box<dyn Error>> {
    let response = send(logout_request(&[("sec-fetch-site", "same-origin")])?).await?;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("no Set-Cookie")?
        .to_str()?;
    assert!(cookie.starts_with("__Secure-vt_sid=;"));
    assert!(cookie.contains("Domain=vikingthe.test"));
    assert!(cookie.contains("Max-Age=0"));
    Ok(())
}

#[tokio::test]
async fn same_site_and_cross_site_writes_are_forbidden() -> Result<(), Box<dyn Error>> {
    for site in ["same-site", "cross-site"] {
        let response = send(logout_request(&[
            ("sec-fetch-site", site),
            ("origin", "https://vikingthe.test"),
            ("content-type", "application/json"),
        ])?)
        .await?;

        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{site}");
        assert!(response.headers().get(header::SET_COOKIE).is_none());
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .map(axum::http::HeaderValue::as_bytes),
            Some(b"application/problem+json".as_slice())
        );
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), BODY_LIMIT).await?)?;
        assert_eq!(body["type"], "https://vikingthe.dev/problems/forbidden");
        assert_eq!(body["status"], 403);
    }
    Ok(())
}

#[tokio::test]
async fn without_fetch_metadata_only_allowed_json_origins_may_write() -> Result<(), Box<dyn Error>>
{
    let allowed = send(logout_request(&[
        ("origin", "https://vikingthe.test"),
        ("content-type", "application/json"),
    ])?)
    .await?;
    assert_eq!(allowed.status(), StatusCode::NO_CONTENT);

    let sibling = send(logout_request(&[
        ("origin", "https://canary.vikingthe.test"),
        ("content-type", "application/json"),
    ])?)
    .await?;
    assert_eq!(sibling.status(), StatusCode::FORBIDDEN);

    let plain_text = send(logout_request(&[
        ("origin", "https://vikingthe.test"),
        ("content-type", "text/plain"),
    ])?)
    .await?;
    assert_eq!(plain_text.status(), StatusCode::FORBIDDEN);
    Ok(())
}

#[tokio::test]
async fn reads_are_not_subject_to_csrf_checks() -> Result<(), Box<dyn Error>> {
    let request = Request::get("/api/health")
        .header("sec-fetch-site", "cross-site")
        .body(Body::empty())?;
    assert_eq!(send(request).await?.status(), StatusCode::OK);
    Ok(())
}
