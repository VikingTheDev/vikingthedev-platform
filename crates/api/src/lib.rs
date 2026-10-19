pub mod cli;
pub mod config;
pub mod csrf;
pub mod database;
pub mod health;
pub mod openapi;
pub mod problem;
pub mod request_id;
mod schema;
pub mod sessions;
pub mod users;

use std::sync::Arc;

use axum::{Router, middleware};

use crate::{config::Config, database::DatabasePool, sessions::SessionStore};

#[derive(Clone)]
pub struct AppState {
    pub database: DatabasePool,
    pub sessions: SessionStore,
    pub config: Arc<Config>,
}

pub fn public_router(state: AppState) -> Router {
    let api = Router::new()
        .merge(health::routes())
        .merge(sessions::routes())
        .merge(users::routes());
    #[cfg(feature = "e2e")]
    let api = api.merge(sessions::test_login::routes());
    // route_layer wraps only the routes merged above it, so it must stay last.
    let api = api.route_layer(middleware::from_fn_with_state(
        state.clone(),
        csrf::reject_cross_origin_writes,
    ));
    Router::new()
        .nest("/api", api)
        .with_state(state)
        .layer(middleware::from_fn(request_id::assign_request_id))
}

pub fn internal_router(state: AppState) -> Router {
    health::routes()
        .merge(users::internal_routes())
        .with_state(state)
        .layer(middleware::from_fn(request_id::assign_request_id))
}
