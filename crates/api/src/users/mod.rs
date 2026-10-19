mod routes;
mod store;

use std::fmt;

use serde::Deserialize;
use time::OffsetDateTime;
use uuid::Uuid;

pub use routes::{__path_me, __path_session, internal_routes, me, routes, session};
pub use store::{UserError, create, find};

const DISPLAY_NAME_MAX_CHARACTERS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UserId(Uuid);

impl UserId {
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl From<Uuid> for UserId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// 1–64 characters, with no control or invisible formatting characters and no surrounding whitespace.
/// Not `Debug`: it's personal data.
#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct DisplayName(String);

#[derive(Debug, thiserror::Error)]
#[error(
    "display name must be 1–64 characters, with no control or invisible formatting characters and no surrounding whitespace"
)]
pub struct InvalidDisplayName;

impl TryFrom<String> for DisplayName {
    type Error = InvalidDisplayName;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        let is_valid = !name.is_empty()
            && name.trim() == name
            && name.chars().count() <= DISPLAY_NAME_MAX_CHARACTERS
            && !name
                .chars()
                .any(|character| character.is_control() || is_invisible_formatting(character));
        is_valid.then_some(Self(name)).ok_or(InvalidDisplayName)
    }
}

impl DisplayName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct User {
    pub id: UserId,
    pub display_name: DisplayName,
    pub created_at: OffsetDateTime,
}

// Bidi overrides and isolates, zero-width characters and direction marks, and the BOM can make one name pass for another.
fn is_invisible_formatting(character: char) -> bool {
    matches!(character, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ordinary_display_names() {
        for name in ["Ada", "Ada Lovelace", "Åse Øvreås", &"x".repeat(64)] {
            assert!(DisplayName::try_from(name.to_owned()).is_ok(), "{name}");
        }
    }

    #[test]
    fn rejects_empty_padded_long_control_or_invisible_display_names() {
        for name in [
            "",
            " Ada",
            "Ada ",
            "Ada\nLovelace",
            "Ada\u{7}",
            &"x".repeat(65),
            "Ada\u{200B}",
            "Ad\u{200F}a",
            "\u{202E}adA",
            "Ada\u{2066}x\u{2069}",
            "\u{FEFF}Ada",
        ] {
            assert!(DisplayName::try_from(name.to_owned()).is_err(), "{name:?}");
        }
    }
}
