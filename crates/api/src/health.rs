use axum::{Json, Router, http::header, response::IntoResponse, routing::get};
use serde::Serialize;
use utoipa::ToSchema;

use crate::AppState;

const GIT_SHA: &str = match option_env!("GIT_SHA") {
    Some(sha) => sha,
    None => "dev",
};

#[derive(Serialize, ToSchema)]
pub struct Health {
    status: HealthStatus,
    sha: &'static str,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum HealthStatus {
    Ok,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

#[utoipa::path(get, path = "/api/health", tag = "health", responses((status = 200, body = Health)))]
#[tracing::instrument(skip_all)]
pub async fn health() -> impl IntoResponse {
    let body = Health {
        status: HealthStatus::Ok,
        sha: GIT_SHA,
    };
    ([(header::CACHE_CONTROL, "no-store")], Json(body))
}
