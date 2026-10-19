use std::fmt;

use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UserId(Uuid);

impl UserId {
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
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
