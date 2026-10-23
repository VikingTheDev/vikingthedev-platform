use std::{fmt, net::IpAddr};

use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderName},
    middleware::Next,
    response::Response,
};

/// The caller's address, read only from the configured trusted header (ADR 0003 point 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientIp(IpAddr);

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ClientIpError {
    #[error("the client-IP header is missing")]
    Missing,
    #[error("the client-IP header must be sent exactly once")]
    Repeated,
    #[error("the client-IP header is not an IP address")]
    Invalid,
}

impl ClientIp {
    /// # Errors
    /// Fails unless `header` appears once and holds a single IP address.
    pub fn from_headers(headers: &HeaderMap, header: &HeaderName) -> Result<Self, ClientIpError> {
        let mut values = headers.get_all(header).iter();
        let value = values.next().ok_or(ClientIpError::Missing)?;
        if values.next().is_some() {
            return Err(ClientIpError::Repeated);
        }
        let address = value
            .to_str()
            .ok()
            .and_then(|text| text.parse::<IpAddr>().ok())
            .ok_or(ClientIpError::Invalid)?;
        Ok(Self(address.to_canonical()))
    }
}

impl fmt::Display for ClientIp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Adds the caller's IP to the request span. Requests without one proceed; only rate-limited ones need it (ADR 0003).
pub async fn record_client_ip(
    State(header): State<HeaderName>,
    request: Request,
    next: Next,
) -> Response {
    if let Ok(client_ip) = ClientIp::from_headers(request.headers(), &header) {
        tracing::Span::current().record("client_ip", tracing::field::display(client_ip));
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    const HEADER: HeaderName = HeaderName::from_static("cf-connecting-ip");

    fn parse(values: &[&'static str]) -> Result<ClientIp, ClientIpError> {
        let mut headers = HeaderMap::new();
        for value in values {
            headers.append(HEADER, HeaderValue::from_static(value));
        }
        ClientIp::from_headers(&headers, &HEADER)
    }

    #[test]
    fn reads_ipv4_and_ipv6_addresses() -> Result<(), ClientIpError> {
        assert_eq!(parse(&["203.0.113.7"])?.to_string(), "203.0.113.7");
        assert_eq!(parse(&["2001:db8::1"])?.to_string(), "2001:db8::1");
        Ok(())
    }

    #[test]
    fn unwraps_ipv4_mapped_ipv6_addresses() -> Result<(), ClientIpError> {
        assert_eq!(parse(&["::ffff:203.0.113.7"])?.to_string(), "203.0.113.7");
        Ok(())
    }

    #[test]
    fn rejects_missing_repeated_and_malformed_values() {
        assert_eq!(parse(&[]), Err(ClientIpError::Missing));
        assert_eq!(
            parse(&["203.0.113.7", "198.51.100.1"]),
            Err(ClientIpError::Repeated)
        );
        for value in [
            "",
            "203.0.113.7, 198.51.100.1",
            "203.0.113.7:443",
            "unknown",
        ] {
            assert_eq!(parse(&[value]), Err(ClientIpError::Invalid), "{value}");
        }
    }
}
