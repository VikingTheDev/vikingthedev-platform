use axum::{
    Router,
    extract::State,
    http::{
        HeaderMap, StatusCode,
        header::{CACHE_CONTROL, SET_COOKIE},
    },
    response::IntoResponse,
    routing::post,
};

use serde::Deserialize;
use utoipa::ToSchema;

use super::cookie::{expired_session_cookie, session_tokens_from};
use crate::{
    AppState,
    problem::{AppError, Problem},
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/auth/logout", post(logout))
}

/// The contract's `{}` body; the handler ignores it, so logout stays idempotent.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LogoutRequest {}

/// # Errors
/// Fails if Valkey is unavailable.
#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "auth",
    summary = "Log out",
    description = "Revokes every session cookie sent and expires the cookie. Idempotent: 204 with or without a live session.",
    request_body = LogoutRequest,
    responses(
        (status = 204, description = "Logged out"),
        (status = 403, description = "Cross-origin write", body = Problem, content_type = "application/problem+json"),
        (status = 500, description = "Session store unavailable", body = Problem, content_type = "application/problem+json"),
    )
)]
#[tracing::instrument(skip_all)]
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    for token in session_tokens_from(&headers) {
        state.sessions.revoke(&token).await?;
    }
    let expired_cookie = expired_session_cookie(&state.config.cookie_domain)?;
    Ok((
        StatusCode::NO_CONTENT,
        [(SET_COOKIE, expired_cookie)],
        [(CACHE_CONTROL, "private, no-store")],
    ))
}
