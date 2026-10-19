pub mod health;
pub mod openapi;

use axum::Router;

pub fn public_router() -> Router {
    Router::new().nest("/api", health::routes())
}

pub fn internal_router() -> Router {
    health::routes()
}
