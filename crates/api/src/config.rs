use std::env;

use anyhow::{Context, bail};
use axum::http::HeaderValue;

use crate::sessions::cookie::CookieDomain;

const HTTPS_SCHEME: &str = "https://";

/// Runtime configuration (ADR 0002 point 4). Not `Debug`: the URLs can hold passwords.
pub struct Config {
    /// Plain TCP on the host's internal Docker network; TLS is mandatory if the database ever moves off-host (step 6 gate).
    pub database_url: String,
    pub valkey_url: String,
    pub cookie_domain: CookieDomain,
    /// Exact `Origin` values accepted for writes when `Sec-Fetch-Site` is absent.
    pub allowed_origins: Vec<HeaderValue>,
}

impl Config {
    /// # Errors
    /// Fails if a variable is missing or invalid.
    pub fn from_environment() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: required("DATABASE_URL")?,
            valkey_url: required("VALKEY_URL")?,
            cookie_domain: CookieDomain::try_from(required("COOKIE_DOMAIN")?)?,
            allowed_origins: parse_origins(&required("ALLOWED_ORIGINS")?)?,
        })
    }
}

fn required(name: &str) -> anyhow::Result<String> {
    env::var(name).with_context(|| format!("{name} must be set"))
}

fn parse_origins(list: &str) -> anyhow::Result<Vec<HeaderValue>> {
    list.split(',').map(str::trim).map(parse_origin).collect()
}

fn parse_origin(origin: &str) -> anyhow::Result<HeaderValue> {
    let Some(host) = origin.strip_prefix(HTTPS_SCHEME) else {
        bail!("allowed origin {origin:?} must start with {HTTPS_SCHEME}");
    };
    if host.is_empty() || host.contains(['/', '?', '#']) {
        bail!("allowed origin {origin:?} must be a bare origin");
    }
    if host.chars().any(|c| c.is_ascii_uppercase()) {
        bail!("allowed origin {origin:?} must be lowercase, as browsers send it");
    }
    Ok(HeaderValue::try_from(origin)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_comma_separated_origin_list() -> anyhow::Result<()> {
        let origins = parse_origins("https://vikingthe.dev, https://canary.vikingthe.dev")?;
        assert_eq!(origins.len(), 2);
        assert_eq!(origins[1], "https://canary.vikingthe.dev");
        Ok(())
    }

    #[test]
    fn rejects_non_https_and_non_bare_origins() {
        for list in [
            "http://vikingthe.dev",
            "https://vikingthe.dev/",
            "https://",
            "vikingthe.dev",
            "https://Vikingthe.dev",
        ] {
            assert!(parse_origins(list).is_err(), "{list}");
        }
    }
}
