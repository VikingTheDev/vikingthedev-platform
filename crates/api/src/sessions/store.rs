use std::{collections::HashMap, sync::LazyLock};

use redis::{
    Client, RedisError, Script,
    aio::{ConnectionManager, ConnectionManagerConfig},
};
use time::{Duration, OffsetDateTime};

use super::token::{SessionToken, StorageId};
use crate::users::UserId;

/// Fixed from login and never extended, matching the cookie's `Max-Age` (step 2 gate, 2026-10-19).
pub const SESSION_LIFETIME: Duration = Duration::days(30);
/// ASVS 7.3.1; measured from `last_seen_at`, so a session can end up to an hour early.
pub const INACTIVITY_TIMEOUT: Duration = Duration::days(7);
// `last_seen_at` is written at most hourly to avoid a write per request.
const LAST_SEEN_REFRESH_INTERVAL: Duration = Duration::hours(1);
const VALKEY_RESPONSE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);

const SESSION_KEY_PREFIX: &str = "sess:";
const USER_SESSIONS_KEY_PREFIX: &str = "user_sess:";
const USER_ID_FIELD: &str = "user_id";
const CREATED_AT_FIELD: &str = "created_at";
const LAST_SEEN_AT_FIELD: &str = "last_seen_at";
const LAST_AUTH_AT_FIELD: &str = "last_auth_at";

// Only touches a session that still exists, so a concurrent logout can't resurrect it. HSET keeps the TTL.
static TOUCH_SCRIPT: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        r"
if redis.call('EXISTS', KEYS[1]) == 1 then
  redis.call('HSET', KEYS[1], ARGV[1], ARGV[2])
end
",
    )
});
static REVOKE_SCRIPT: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        r"
local user_id = redis.call('HGET', KEYS[1], ARGV[1])
redis.call('DEL', KEYS[1])
if user_id then redis.call('SREM', ARGV[2] .. user_id, ARGV[3]) end
",
    )
});
static REVOKE_ALL_SCRIPT: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        r"
for _, id in ipairs(redis.call('SMEMBERS', KEYS[1])) do
  redis.call('DEL', ARGV[1] .. id)
end
redis.call('DEL', KEYS[1])
",
    )
});

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub user_id: UserId,
    pub created_at: OffsetDateTime,
    pub last_seen_at: OffsetDateTime,
    pub last_auth_at: OffsetDateTime,
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("session store request failed: {0}")]
    Valkey(#[from] RedisError),
    #[error("random source unavailable: {0}")]
    Random(#[from] getrandom::Error),
    #[error("session record is malformed")]
    MalformedRecord,
}

#[derive(Clone)]
pub struct SessionStore {
    connection: ConnectionManager,
}

impl SessionStore {
    /// Connects lazily: the first command opens the connection, and it reconnects on its own.
    ///
    /// # Errors
    /// Fails if `valkey_url` is not a valid Redis URL.
    pub fn connect(valkey_url: &str) -> Result<Self, SessionError> {
        let config =
            ConnectionManagerConfig::new().set_response_timeout(Some(VALKEY_RESPONSE_TIMEOUT));
        let connection = Client::open(valkey_url)?.get_connection_manager_lazy(config)?;
        Ok(Self { connection })
    }

    /// # Errors
    /// Fails if randomness or Valkey is unavailable.
    #[tracing::instrument(skip_all, fields(user_id = %user_id))]
    pub async fn create(
        &self,
        user_id: UserId,
        now: OffsetDateTime,
    ) -> Result<SessionToken, SessionError> {
        let token = SessionToken::generate()?;
        let id = token.storage_id();
        let session_key = session_key(&id);
        let user_sessions_key = user_sessions_key(user_id);
        let timestamp = now.unix_timestamp();
        let expires_at = (now + SESSION_LIFETIME).unix_timestamp();
        redis::pipe()
            .atomic()
            .cmd("HSET")
            .arg(&session_key)
            .arg(USER_ID_FIELD)
            .arg(user_id.to_string())
            .arg(CREATED_AT_FIELD)
            .arg(timestamp)
            .arg(LAST_SEEN_AT_FIELD)
            .arg(timestamp)
            .arg(LAST_AUTH_AT_FIELD)
            .arg(timestamp)
            .ignore()
            .cmd("EXPIREAT")
            .arg(&session_key)
            .arg(expires_at)
            .ignore()
            .cmd("SADD")
            .arg(&user_sessions_key)
            .arg(id.as_str())
            .ignore()
            .cmd("EXPIREAT")
            .arg(&user_sessions_key)
            .arg(expires_at)
            .ignore()
            .query_async::<()>(&mut self.connection.clone())
            .await?;
        Ok(token)
    }

    /// Returns the live session for `token`, recording `last_seen_at` at most once an hour.
    ///
    /// # Errors
    /// Fails if Valkey is unavailable or the stored record is malformed.
    #[tracing::instrument(skip_all)]
    pub async fn load(
        &self,
        token: &SessionToken,
        now: OffsetDateTime,
    ) -> Result<Option<Session>, SessionError> {
        let session_key = session_key(&token.storage_id());
        let mut connection = self.connection.clone();
        let fields: HashMap<String, String> = redis::cmd("HGETALL")
            .arg(&session_key)
            .query_async(&mut connection)
            .await?;
        if fields.is_empty() {
            return Ok(None);
        }
        let session = parse_session(&fields)?;
        if is_expired(session.created_at, now) || is_idle(session.last_seen_at, now) {
            self.revoke(token).await?;
            return Ok(None);
        }
        if !needs_last_seen_update(session.last_seen_at, now) {
            return Ok(Some(session));
        }
        TOUCH_SCRIPT
            .key(&session_key)
            .arg(LAST_SEEN_AT_FIELD)
            .arg(now.unix_timestamp())
            .invoke_async::<()>(&mut connection)
            .await?;
        Ok(Some(Session {
            last_seen_at: now,
            ..session
        }))
    }

    /// # Errors
    /// Fails if Valkey is unavailable.
    #[tracing::instrument(skip_all)]
    pub async fn revoke(&self, token: &SessionToken) -> Result<(), SessionError> {
        let id = token.storage_id();
        REVOKE_SCRIPT
            .key(session_key(&id))
            .arg(USER_ID_FIELD)
            .arg(USER_SESSIONS_KEY_PREFIX)
            .arg(id.as_str())
            .invoke_async::<()>(&mut self.connection.clone())
            .await?;
        Ok(())
    }

    /// # Errors
    /// Fails if Valkey is unavailable.
    #[tracing::instrument(skip_all, fields(user_id = %user_id))]
    pub async fn revoke_all(&self, user_id: UserId) -> Result<(), SessionError> {
        REVOKE_ALL_SCRIPT
            .key(user_sessions_key(user_id))
            .arg(SESSION_KEY_PREFIX)
            .invoke_async::<()>(&mut self.connection.clone())
            .await?;
        Ok(())
    }
}

pub fn session_key(id: &StorageId) -> String {
    format!("{SESSION_KEY_PREFIX}{}", id.as_str())
}

pub fn user_sessions_key(user_id: UserId) -> String {
    format!("{USER_SESSIONS_KEY_PREFIX}{user_id}")
}

fn parse_session(fields: &HashMap<String, String>) -> Result<Session, SessionError> {
    let user_id = fields
        .get(USER_ID_FIELD)
        .and_then(|id| id.parse::<uuid::Uuid>().ok())
        .ok_or(SessionError::MalformedRecord)?;
    Ok(Session {
        user_id: UserId::from(user_id),
        created_at: parse_timestamp(fields, CREATED_AT_FIELD)?,
        last_seen_at: parse_timestamp(fields, LAST_SEEN_AT_FIELD)?,
        last_auth_at: parse_timestamp(fields, LAST_AUTH_AT_FIELD)?,
    })
}

fn parse_timestamp(
    fields: &HashMap<String, String>,
    field: &str,
) -> Result<OffsetDateTime, SessionError> {
    fields
        .get(field)
        .and_then(|seconds| seconds.parse().ok())
        .and_then(|seconds| OffsetDateTime::from_unix_timestamp(seconds).ok())
        .ok_or(SessionError::MalformedRecord)
}

fn is_expired(created_at: OffsetDateTime, now: OffsetDateTime) -> bool {
    now - created_at >= SESSION_LIFETIME
}

fn is_idle(last_seen_at: OffsetDateTime, now: OffsetDateTime) -> bool {
    now - last_seen_at >= INACTIVITY_TIMEOUT
}

fn needs_last_seen_update(last_seen_at: OffsetDateTime, now: OffsetDateTime) -> bool {
    now - last_seen_at >= LAST_SEEN_REFRESH_INTERVAL
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: OffsetDateTime = OffsetDateTime::UNIX_EPOCH;

    #[test]
    fn sessions_expire_thirty_days_after_login() {
        assert!(!is_expired(START, START + Duration::days(29)));
        assert!(is_expired(START, START + SESSION_LIFETIME));
    }

    #[test]
    fn sessions_end_after_seven_idle_days() {
        assert!(!is_idle(
            START,
            START + INACTIVITY_TIMEOUT - Duration::minutes(1)
        ));
        assert!(is_idle(START, START + INACTIVITY_TIMEOUT));
    }

    #[test]
    fn last_seen_is_written_at_most_hourly() {
        assert!(!needs_last_seen_update(
            START,
            START + Duration::minutes(59)
        ));
        assert!(needs_last_seen_update(START, START + Duration::hours(1)));
    }
}
