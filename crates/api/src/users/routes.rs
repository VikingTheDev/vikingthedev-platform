use axum::{Json, Router, http::header::CACHE_CONTROL, response::IntoResponse, routing::get};
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use super::User;
use crate::{AppState, problem::Problem, sessions::CurrentUser};

const PRIVATE_NO_STORE: &str = "private, no-store";

#[derive(Serialize, ToSchema)]
#[schema(as = User)]
pub struct UserResponse {
    id: Uuid,
    display_name: String,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    created_at: OffsetDateTime,
}

impl From<&User> for UserResponse {
    fn from(user: &User) -> Self {
        Self {
            id: user.id.as_uuid(),
            display_name: user.display_name.as_str().to_owned(),
            created_at: user.created_at,
        }
    }
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    #[expect(
        dead_code,
        reason = "read from PostgreSQL from M3; published now so the contract lists both roles"
    )]
    Admin,
}

#[derive(Serialize, ToSchema)]
#[schema(as = SessionUser)]
pub struct SessionUserResponse {
    user: UserResponse,
    role: Role,
    /// App names the user may use, e.g. "stash".
    grants: Vec<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/me", get(me))
}

/// Served only on the internal listener (`:8081`), for Astro's server-side session check.
pub fn internal_routes() -> Router<AppState> {
    Router::new().route("/session", get(session))
}

#[utoipa::path(
    get,
    path = "/api/me",
    tag = "users",
    summary = "The logged-in user",
    responses(
        (status = 200, body = UserResponse),
        (status = 401, description = "No valid session", body = Problem, content_type = "application/problem+json"),
        (status = 500, description = "Session store or database unavailable", body = Problem, content_type = "application/problem+json"),
    )
)]
#[tracing::instrument(skip_all)]
pub async fn me(CurrentUser(user): CurrentUser) -> impl IntoResponse {
    (
        [(CACHE_CONTROL, PRIVATE_NO_STORE)],
        Json(UserResponse::from(&user)),
    )
}

#[utoipa::path(
    get,
    path = "/session",
    tag = "internal",
    summary = "The session user, for server-side rendering",
    description = "Internal listener (`:8081`) only, never routed publicly. Astro forwards the browser's `Cookie` header.",
    responses(
        (status = 200, body = SessionUserResponse),
        (status = 401, description = "No valid session", body = Problem, content_type = "application/problem+json"),
        (status = 500, description = "Session store or database unavailable", body = Problem, content_type = "application/problem+json"),
    )
)]
#[tracing::instrument(skip_all)]
pub async fn session(CurrentUser(user): CurrentUser) -> impl IntoResponse {
    let body = SessionUserResponse {
        user: UserResponse::from(&user),
        // TODO(M3): read the role and grants from PostgreSQL once roles exist (D18).
        role: Role::User,
        grants: Vec::new(),
    };
    ([(CACHE_CONTROL, PRIVATE_NO_STORE)], Json(body))
}
