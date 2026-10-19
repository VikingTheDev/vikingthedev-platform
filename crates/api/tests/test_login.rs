//! The `e2e` tests need PostgreSQL and Valkey: `docker compose -f compose.dev.yaml up -d --wait`.
mod common;

use std::error::Error;

use api::AppState;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    response::Response,
};
use tower::ServiceExt;

async fn send_test_login(
    state: AppState,
    sec_fetch_site: &str,
    body: &str,
) -> Result<Response, Box<dyn Error>> {
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/test/login")
        .header("sec-fetch-site", sec_fetch_site)
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))?;
    Ok(api::public_router(state).oneshot(request).await?)
}

#[cfg(not(feature = "e2e"))]
#[tokio::test]
async fn production_builds_have_no_test_login() -> Result<(), Box<dyn Error>> {
    let response = send_test_login(
        common::test_state()?,
        "same-origin",
        r#"{"display_name":"Ada"}"#,
    )
    .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    Ok(())
}

#[cfg(feature = "e2e")]
mod e2e {
    use super::*;
    use api::sessions::{SessionToken, cookie::SESSION_COOKIE_NAME};
    use axum::http::header::{CACHE_CONTROL, SET_COOKIE};
    use time::OffsetDateTime;

    fn session_token_set_by(response: &Response) -> Option<SessionToken> {
        let cookie = response.headers().get(SET_COOKIE)?.to_str().ok()?;
        let value = cookie
            .strip_prefix(SESSION_COOKIE_NAME)?
            .strip_prefix('=')?
            .split(';')
            .next()?;
        SessionToken::parse(value)
    }

    async fn log_in_as_new_user() -> Result<api::sessions::Session, Box<dyn Error>> {
        let response = send_test_login(
            common::migrated_state().await?,
            "same-origin",
            r#"{"display_name":"Ada"}"#,
        )
        .await?;

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            response
                .headers()
                .get(CACHE_CONTROL)
                .ok_or("no Cache-Control")?,
            "private, no-store"
        );
        let cookie = response.headers().get(SET_COOKIE).ok_or("no Set-Cookie")?;
        assert!(cookie.to_str()?.contains("; Domain=vikingthe.test;"));
        let token = session_token_set_by(&response).ok_or("no session cookie")?;
        Ok(common::test_state()?
            .sessions
            .load(&token, OffsetDateTime::now_utc())
            .await?
            .ok_or("no session")?)
    }

    #[tokio::test]
    async fn each_test_login_starts_a_session_for_a_new_user() -> Result<(), Box<dyn Error>> {
        let first = log_in_as_new_user().await?;
        let second = log_in_as_new_user().await?;
        assert_ne!(first.user_id, second.user_id);
        Ok(())
    }

    #[tokio::test]
    async fn test_login_rejects_invalid_bodies() -> Result<(), Box<dyn Error>> {
        for body in [
            r#"{"display_name":""}"#,
            r#"{"display_name":"Ada","role":"admin"}"#,
            "{}",
            "not json",
        ] {
            let response =
                send_test_login(common::migrated_state().await?, "same-origin", body).await?;
            assert_eq!(
                response.status(),
                StatusCode::UNPROCESSABLE_ENTITY,
                "{body}"
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_login_rejects_cross_origin_writes() -> Result<(), Box<dyn Error>> {
        let response = send_test_login(
            common::migrated_state().await?,
            "same-site",
            r#"{"display_name":"Ada"}"#,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        Ok(())
    }
}
