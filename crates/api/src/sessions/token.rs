use std::fmt::Write;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

const TOKEN_BYTES: usize = 32;

/// The secret half of a session; only its hash is ever stored.
pub struct SessionToken([u8; TOKEN_BYTES]);

/// The SHA-256 of a session token, as lowercase hex: the only form Valkey sees.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageId(String);

impl SessionToken {
    /// # Errors
    /// Fails if the operating system's random source is unavailable.
    pub fn generate() -> Result<Self, getrandom::Error> {
        let mut bytes = [0; TOKEN_BYTES];
        getrandom::fill(&mut bytes)?;
        Ok(Self(bytes))
    }

    pub fn parse(cookie_value: &str) -> Option<Self> {
        let bytes = URL_SAFE_NO_PAD.decode(cookie_value).ok()?;
        bytes.try_into().ok().map(Self)
    }

    pub fn to_cookie_value(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0)
    }

    pub fn storage_id(&self) -> StorageId {
        let digest = Sha256::digest(self.0);
        let hex = digest.iter().fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        });
        StorageId(hex)
    }
}

impl StorageId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_value_round_trips() -> Result<(), getrandom::Error> {
        let token = SessionToken::generate()?;
        let parsed = SessionToken::parse(&token.to_cookie_value());
        assert_eq!(
            parsed.map(|parsed| parsed.storage_id()),
            Some(token.storage_id())
        );
        Ok(())
    }

    #[test]
    fn cookie_value_is_43_url_safe_characters() -> Result<(), getrandom::Error> {
        let value = SessionToken::generate()?.to_cookie_value();
        assert_eq!(value.len(), 43);
        assert!(
            value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
        Ok(())
    }

    #[test]
    fn storage_id_is_sha256_hex_not_the_token() -> Result<(), getrandom::Error> {
        let token = SessionToken::generate()?;
        let id = token.storage_id();
        assert_eq!(id.as_str().len(), 64);
        assert!(id.as_str().chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(id.as_str(), token.to_cookie_value());
        Ok(())
    }

    #[test]
    fn tokens_are_unique() -> Result<(), getrandom::Error> {
        let first = SessionToken::generate()?;
        let second = SessionToken::generate()?;
        assert_ne!(first.storage_id(), second.storage_id());
        Ok(())
    }

    #[test]
    fn rejects_malformed_cookie_values() {
        assert!(SessionToken::parse("").is_none());
        assert!(SessionToken::parse("not base64!").is_none());
        assert!(SessionToken::parse(&URL_SAFE_NO_PAD.encode([0u8; 16])).is_none());
    }
}
