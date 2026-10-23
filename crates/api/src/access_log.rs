use std::time::Instant;

use axum::{extract::Request, middleware::Next, response::Response};

/// Logs one line per request inside its span, so the line carries the request ID and client IP.
pub async fn log_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    // The path only: a query string could carry a token.
    let path = request.uri().path().to_owned();
    let started = Instant::now();
    let response = next.run(request).await;
    tracing::info!(
        %method,
        %path,
        status = response.status().as_u16(),
        elapsed_ms = started.elapsed().as_secs_f64() * 1000.0,
        "request"
    );
    response
}
