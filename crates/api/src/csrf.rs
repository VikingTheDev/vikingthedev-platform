use axum::{
    extract::{Request, State},
    http::{
        HeaderMap, HeaderValue, Method,
        header::{CONTENT_TYPE, ORIGIN},
    },
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{AppState, problem::ProblemType};

const SEC_FETCH_SITE: &str = "sec-fetch-site";
const SAME_ORIGIN: &str = "same-origin";
const JSON_MEDIA_TYPE: &str = "application/json";

/// Rejects writes that don't come from the same origin. Every subdomain is same-site, so `SameSite` alone can't.
pub async fn reject_cross_origin_writes(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let is_write = !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    if is_write && !is_same_origin(request.headers(), &state.config.allowed_origins) {
        return ProblemType::Forbidden.into_response();
    }
    next.run(request).await
}

fn is_same_origin(headers: &HeaderMap, allowed_origins: &[HeaderValue]) -> bool {
    match headers.get(SEC_FETCH_SITE) {
        Some(site) => site == SAME_ORIGIN,
        None => has_allowed_origin(headers, allowed_origins) && is_json(headers),
    }
}

fn has_allowed_origin(headers: &HeaderMap, allowed_origins: &[HeaderValue]) -> bool {
    headers
        .get(ORIGIN)
        .is_some_and(|origin| allowed_origins.contains(origin))
}

fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case(JSON_MEDIA_TYPE))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALLOWED: [HeaderValue; 1] = [HeaderValue::from_static("https://vikingthe.dev")];

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        pairs
            .iter()
            .map(|(name, value)| {
                (
                    axum::http::HeaderName::from_static(name),
                    HeaderValue::from_static(value),
                )
            })
            .collect()
    }

    #[test]
    fn fetch_metadata_decides_when_present() {
        assert!(is_same_origin(
            &headers(&[(SEC_FETCH_SITE, "same-origin")]),
            &ALLOWED
        ));
        for site in ["same-site", "cross-site", "none"] {
            let request = headers(&[
                (SEC_FETCH_SITE, site),
                ("origin", "https://vikingthe.dev"),
                ("content-type", "application/json"),
            ]);
            assert!(!is_same_origin(&request, &ALLOWED), "{site}");
        }
    }

    #[test]
    fn without_fetch_metadata_needs_allowed_origin_and_json() {
        let allowed = headers(&[
            ("origin", "https://vikingthe.dev"),
            ("content-type", "application/json; charset=utf-8"),
        ]);
        assert!(is_same_origin(&allowed, &ALLOWED));

        let wrong_origin = headers(&[
            ("origin", "https://canary.vikingthe.dev"),
            ("content-type", "application/json"),
        ]);
        let form_body = headers(&[
            ("origin", "https://vikingthe.dev"),
            ("content-type", "text/plain"),
        ]);
        let no_origin = headers(&[("content-type", "application/json")]);
        for request in [wrong_origin, form_body, no_origin] {
            assert!(!is_same_origin(&request, &ALLOWED));
        }
    }
}
