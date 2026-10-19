pub mod config;
pub mod csrf;
pub mod health;
pub mod openapi;
pub mod problem;
pub mod sessions;
pub mod users;

use std::sync::Arc;

use axum::{Router, middleware};

use crate::{config::Config, sessions::SessionStore};

#[derive(Clone)]
pub struct AppState {
    pub sessions: SessionStore,
    pub config: Arc<Config>,
}

pub fn public_router(state: AppState) -> Router {
    let api = Router::new()
        .merge(health::routes())
        .merge(sessions::routes())
        // route_layer wraps only the routes merged above it, so it must stay last.
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            csrf::reject_cross_origin_writes,
        ));
    Router::new().nest("/api", api).with_state(state)
}

pub fn internal_router(state: AppState) -> Router {
    health::routes().with_state(state)
}
