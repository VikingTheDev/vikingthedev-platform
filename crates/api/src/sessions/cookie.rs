use axum::http::{
    HeaderMap, HeaderValue,
    header::{COOKIE, InvalidHeaderValue},
};

use super::{store::SESSION_LIFETIME, token::SessionToken};

pub const SESSION_COOKIE_NAME: &str = "__Secure-vt_sid";
const SESSION_COOKIE_MAX_AGE_SECONDS: i64 = SESSION_LIFETIME.whole_seconds();

/// A registrable domain such as `vikingthe.dev`, validated once at startup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CookieDomain(String);

#[derive(Debug, thiserror::Error)]
#[error("cookie domain must be lowercase DNS labels with at least one dot")]
pub struct InvalidCookieDomain;

impl TryFrom<String> for CookieDomain {
    type Error = InvalidCookieDomain;

    fn try_from(domain: String) -> Result<Self, Self::Error> {
        let labels: Vec<&str> = domain.split('.').collect();
        let is_valid = labels.len() >= 2 && labels.iter().all(|label| is_dns_label(label));
        is_valid.then_some(Self(domain)).ok_or(InvalidCookieDomain)
    }
}

fn is_dns_label(label: &str) -> bool {
    !label.is_empty()
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// # Errors
/// Never in practice: the token is base64url and the domain is validated.
pub fn session_cookie(
    token: &SessionToken,
    domain: &CookieDomain,
) -> Result<HeaderValue, InvalidHeaderValue> {
    cookie_header(
        &token.to_cookie_value(),
        domain,
        SESSION_COOKIE_MAX_AGE_SECONDS,
    )
}

/// # Errors
/// Never in practice: the domain is validated.
pub fn expired_session_cookie(domain: &CookieDomain) -> Result<HeaderValue, InvalidHeaderValue> {
    cookie_header("", domain, 0)
}

fn cookie_header(
    value: &str,
    domain: &CookieDomain,
    max_age_seconds: i64,
) -> Result<HeaderValue, InvalidHeaderValue> {
    HeaderValue::try_from(format!(
        "{SESSION_COOKIE_NAME}={value}; Domain={}; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age={max_age_seconds}",
        domain.0
    ))
}

/// Every well-formed session token among the request's cookies; a sibling subdomain can plant extra ones.
pub fn session_tokens_from(headers: &HeaderMap) -> impl Iterator<Item = SessionToken> + '_ {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|header| header.to_str().ok())
        .flat_map(|header| header.split(';'))
        .filter_map(|pair| {
            pair.trim()
                .strip_prefix(SESSION_COOKIE_NAME)?
                .strip_prefix('=')
        })
        .filter_map(SessionToken::parse)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn domain() -> Result<CookieDomain, InvalidCookieDomain> {
        CookieDomain::try_from("vikingthe.dev".to_owned())
    }

    #[test]
    fn session_cookie_has_exactly_the_adr_0001_attributes() -> Result<(), Box<dyn std::error::Error>>
    {
        let token = SessionToken::generate()?;
        let cookie = session_cookie(&token, &domain()?)?;
        let expected = format!(
            "__Secure-vt_sid={}; Domain=vikingthe.dev; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=2592000",
            token.to_cookie_value()
        );
        assert_eq!(cookie.to_str()?, expected);
        Ok(())
    }

    #[test]
    fn expired_cookie_keeps_the_domain() -> Result<(), Box<dyn std::error::Error>> {
        let cookie = expired_session_cookie(&domain()?)?;
        assert_eq!(
            cookie.to_str()?,
            "__Secure-vt_sid=; Domain=vikingthe.dev; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=0"
        );
        Ok(())
    }

    #[test]
    fn rejects_invalid_domains() {
        for domain in [
            "",
            "localhost",
            ".vikingthe.dev",
            "Vikingthe.dev",
            "a..dev",
            "-a.dev",
            "a b.dev",
        ] {
            assert!(
                CookieDomain::try_from(domain.to_owned()).is_err(),
                "{domain}"
            );
        }
    }

    #[test]
    fn finds_the_token_among_other_cookies() -> Result<(), Box<dyn std::error::Error>> {
        let token = SessionToken::generate()?;
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::try_from(format!(
                "theme=dark; __Secure-vt_sid={}; other=1",
                token.to_cookie_value()
            ))?,
        );
        let found: Vec<_> = session_tokens_from(&headers)
            .map(|found| found.storage_id())
            .collect();
        assert_eq!(found, vec![token.storage_id()]);
        Ok(())
    }

    #[test]
    fn finds_every_session_token() -> Result<(), Box<dyn std::error::Error>> {
        let first = SessionToken::generate()?;
        let second = SessionToken::generate()?;
        let mut headers = HeaderMap::new();
        headers.append(
            COOKIE,
            HeaderValue::try_from(format!("__Secure-vt_sid={}", first.to_cookie_value()))?,
        );
        headers.append(
            COOKIE,
            HeaderValue::try_from(format!("__Secure-vt_sid={}", second.to_cookie_value()))?,
        );
        let found: Vec<_> = session_tokens_from(&headers)
            .map(|found| found.storage_id())
            .collect();
        assert_eq!(found, vec![first.storage_id(), second.storage_id()]);
        Ok(())
    }

    #[test]
    fn ignores_lookalike_and_malformed_cookies() {
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_static("__Secure-vt_sidx=abc; __Secure-vt_sid=short"),
        );
        assert_eq!(session_tokens_from(&headers).count(), 0);
    }
}
