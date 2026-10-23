mod common;

use std::{
    error::Error,
    io,
    sync::{Arc, Mutex, PoisonError},
};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct CapturedLogs(Arc<Mutex<Vec<u8>>>);

impl io::Write for CapturedLogs {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl MakeWriter<'_> for CapturedLogs {
    type Writer = Self;

    fn make_writer(&self) -> Self {
        self.clone()
    }
}

impl CapturedLogs {
    fn text(&self) -> Result<String, Box<dyn Error>> {
        let bytes = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(std::str::from_utf8(&bytes)?.to_owned())
    }

    fn request_lines(&self) -> Result<Vec<Value>, Box<dyn Error>> {
        let lines = self
            .text()?
            .lines()
            .map(serde_json::from_str::<Value>)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(lines
            .into_iter()
            .filter(|line| line["fields"]["message"] == "request")
            .collect())
    }
}

// The subscriber is set per thread, which covers the whole request on the test's current-thread runtime.
async fn send_logged(
    router: Router,
    request: Request<Body>,
) -> Result<(StatusCode, CapturedLogs), Box<dyn Error>> {
    let logs = CapturedLogs::default();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_writer(logs.clone())
        .finish();
    let status = {
        let _default = tracing::subscriber::set_default(subscriber);
        router.oneshot(request).await?.status()
    };
    Ok((status, logs))
}

#[tokio::test]
async fn public_requests_log_the_ip_from_the_trusted_header_only() -> Result<(), Box<dyn Error>> {
    let request = Request::get("/api/health?token=secret")
        .header(common::CLIENT_IP_HEADER, "203.0.113.7")
        .header("x-forwarded-for", "198.51.100.1")
        .body(Body::empty())?;

    let (status, logs) = send_logged(api::public_router(common::test_state()?), request).await?;

    assert_eq!(status, StatusCode::OK);
    let text = logs.text()?;
    assert!(
        !text.contains("secret") && !text.contains("198.51.100.1"),
        "{text}"
    );
    let lines = logs.request_lines()?;
    let [line] = lines.as_slice() else {
        return Err(format!("expected one request line, got {lines:?}").into());
    };
    assert_eq!(line["span"]["client_ip"], "203.0.113.7");
    assert!(line["span"]["request_id"].is_string());
    assert_eq!(line["fields"]["method"], "GET");
    assert_eq!(line["fields"]["path"], "/api/health");
    assert_eq!(line["fields"]["status"], 200);
    assert!(line["fields"]["elapsed_ms"].is_f64());
    Ok(())
}

#[tokio::test]
async fn public_requests_without_a_valid_client_ip_still_proceed() -> Result<(), Box<dyn Error>> {
    let request = Request::get("/api/health")
        .header(common::CLIENT_IP_HEADER, "203.0.113.7, 198.51.100.1")
        .body(Body::empty())?;

    let (status, logs) = send_logged(api::public_router(common::test_state()?), request).await?;

    assert_eq!(status, StatusCode::OK);
    let lines = logs.request_lines()?;
    let [line] = lines.as_slice() else {
        return Err(format!("expected one request line, got {lines:?}").into());
    };
    assert!(line["span"].get("client_ip").is_none());
    Ok(())
}

#[tokio::test]
async fn internal_requests_are_logged_without_a_client_ip() -> Result<(), Box<dyn Error>> {
    let request = Request::get("/health")
        .header(common::CLIENT_IP_HEADER, "203.0.113.7")
        .body(Body::empty())?;

    let (status, logs) = send_logged(api::internal_router(common::test_state()?), request).await?;

    assert_eq!(status, StatusCode::OK);
    let lines = logs.request_lines()?;
    let [line] = lines.as_slice() else {
        return Err(format!("expected one request line, got {lines:?}").into());
    };
    assert_eq!(line["fields"]["path"], "/health");
    assert!(line["span"].get("client_ip").is_none());
    Ok(())
}
