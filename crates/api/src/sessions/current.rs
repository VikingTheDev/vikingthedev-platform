use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, request::Parts},
    response::{IntoResponse, Response},
};
use time::OffsetDateTime;

use super::cookie::sole_session_token;
use crate::{
    AppState,
    problem::{AppError, ProblemType},
    users::{self, User},
};

/// The user behind the request's session; rejects with 401 `unauthenticated` otherwise.
pub struct CurrentUser(pub User);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Response> {
        match current_user(state, &parts.headers, OffsetDateTime::now_utc()).await {
            Ok(Some(user)) => Ok(Self(user)),
            Ok(None) => Err(ProblemType::Unauthenticated.into_response()),
            Err(error) => Err(error.into_response()),
        }
    }
}

async fn current_user(
    state: &AppState,
    headers: &HeaderMap,
    now: OffsetDateTime,
) -> Result<Option<User>, AppError> {
    let Some(token) = sole_session_token(headers) else {
        return Ok(None);
    };
    let Some(session) = state.sessions.load(&token, now).await? else {
        return Ok(None);
    };
    Ok(users::find(&state.database, session.user_id).await?)
}
