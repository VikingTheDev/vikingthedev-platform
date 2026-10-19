use axum::{
    Json, Router,
    extract::{State, rejection::JsonRejection},
    http::{
        StatusCode,
        header::{CACHE_CONTROL, SET_COOKIE},
    },
    response::{IntoResponse, Response},
    routing::post,
};
use serde::Deserialize;
use time::OffsetDateTime;

use super::cookie::session_cookie;
use crate::{
    AppState,
    problem::{AppError, ProblemType},
    users::{self, DisplayName},
};

// No `ToSchema`: the route is deliberately left out of the published OpenAPI.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TestLoginRequest {
    display_name: DisplayName,
}

/// Compiled only into `e2e` builds for the CI stack (ADR 0002 point 3).
pub fn routes() -> Router<AppState> {
    Router::new().route("/test/login", post(test_login))
}

#[tracing::instrument(skip_all)]
async fn test_login(
    State(state): State<AppState>,
    payload: Result<Json<TestLoginRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Ok(Json(request)) = payload else {
        return Ok(ProblemType::Validation.into_response());
    };
    let token = users::register_with_session(
        &state.sessions,
        &request.display_name,
        OffsetDateTime::now_utc(),
    )
    .await?;
    let cookie = session_cookie(&token, &state.config.cookie_domain)?;
    Ok((
        StatusCode::NO_CONTENT,
        [(SET_COOKIE, cookie)],
        [(CACHE_CONTROL, "private, no-store")],
    )
        .into_response())
}
