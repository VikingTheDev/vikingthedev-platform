use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use tracing::Instrument as _;
use uuid::Uuid;

pub const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

tokio::task_local! {
    static REQUEST_ID: Uuid;
}

/// The current request's ID, for problem `instance` (ADR 0006).
pub fn current() -> Option<Uuid> {
    REQUEST_ID.try_with(|id| *id).ok()
}

/// Gives every request a fresh `UUIDv7`. An incoming `X-Request-Id` is ignored, so clients can't plant values in our logs.
pub async fn assign_request_id(request: Request, next: Next) -> Response {
    let id = Uuid::now_v7();
    let span = tracing::info_span!(
        "request",
        request_id = %id,
        client_ip = tracing::field::Empty
    );
    let mut response = REQUEST_ID
        .scope(id, next.run(request))
        .instrument(span)
        .await;
    if let Ok(value) = HeaderValue::try_from(id.to_string()) {
        response.headers_mut().insert(REQUEST_ID_HEADER, value);
    }
    response
}
