use axum::{
    Json,
    http::{
        HeaderValue, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE, InvalidHeaderValue},
    },
    response::{IntoResponse, Response},
};
use serde::{Serialize, Serializer};
use utoipa::{
    PartialSchema, ToSchema,
    openapi::{
        RefOr,
        schema::{ObjectBuilder, Schema, Type},
    },
};

use crate::{request_id, sessions::SessionError, users::UserError};

const PROBLEM_TYPE_BASE: &str = "https://vikingthe.dev/problems/";
const PROBLEM_CONTENT_TYPE: &str = "application/problem+json";

/// RFC 9457 problem types (ADR 0006), all of `api-contract.md`; clients switch on the `type` URI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProblemType {
    Unauthenticated,
    ReauthenticationRequired,
    Forbidden,
    NotFound,
    Validation,
    IdempotencyConflict,
    RateLimited,
    Unavailable,
    Internal,
}

#[derive(Serialize, ToSchema)]
pub struct Problem {
    #[serde(rename = "type")]
    kind: ProblemType,
    title: &'static str,
    status: u16,
    /// `urn:uuid:<request-id>`, the same ID as the `X-Request-Id` header.
    #[serde(skip_serializing_if = "Option::is_none")]
    instance: Option<String>,
}

impl ProblemType {
    const ALL: [Self; 9] = [
        Self::Unauthenticated,
        Self::ReauthenticationRequired,
        Self::Forbidden,
        Self::NotFound,
        Self::Validation,
        Self::IdempotencyConflict,
        Self::RateLimited,
        Self::Unavailable,
        Self::Internal,
    ];
    // Fails to compile when a variant is added, as a reminder to list it in `ALL` above.
    const _ALL_IS_COMPLETE: () = match Self::Internal {
        Self::Unauthenticated
        | Self::ReauthenticationRequired
        | Self::Forbidden
        | Self::NotFound
        | Self::Validation
        | Self::IdempotencyConflict
        | Self::RateLimited
        | Self::Unavailable
        | Self::Internal => (),
    };

    fn uri(self) -> String {
        format!("{PROBLEM_TYPE_BASE}{}", self.code())
    }

    fn code(self) -> &'static str {
        match self {
            Self::Unauthenticated => "unauthenticated",
            Self::ReauthenticationRequired => "reauthentication_required",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::Validation => "validation",
            Self::IdempotencyConflict => "idempotency_conflict",
            Self::RateLimited => "rate_limited",
            Self::Unavailable => "unavailable",
            Self::Internal => "internal",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Unauthenticated => "Authentication required",
            Self::ReauthenticationRequired => "Recent login required",
            Self::Forbidden => "Forbidden",
            Self::NotFound => "Not found",
            Self::Validation => "Validation failed",
            Self::IdempotencyConflict => "Idempotency key reused with a different request",
            Self::RateLimited => "Too many requests",
            Self::Unavailable => "Service unavailable",
            Self::Internal => "Internal server error",
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::Unauthenticated | Self::ReauthenticationRequired => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Validation | Self::IdempotencyConflict => StatusCode::UNPROCESSABLE_ENTITY,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl Serialize for ProblemType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.uri())
    }
}

// Publishes `type` as the contract's closed set of URIs; clients still keep a default branch for skew between deploys.
impl PartialSchema for ProblemType {
    fn schema() -> RefOr<Schema> {
        ObjectBuilder::new()
            .schema_type(Type::String)
            .enum_values(Some(Self::ALL.map(Self::uri)))
            .into()
    }
}

impl ToSchema for ProblemType {}

impl IntoResponse for ProblemType {
    fn into_response(self) -> Response {
        let body = Problem {
            kind: self,
            title: self.title(),
            status: self.status().as_u16(),
            instance: request_id::current().map(|id| format!("urn:uuid:{id}")),
        };
        (
            self.status(),
            [
                (CONTENT_TYPE, HeaderValue::from_static(PROBLEM_CONTENT_TYPE)),
                (CACHE_CONTROL, HeaderValue::from_static("no-store")),
            ],
            Json(body),
        )
            .into_response()
    }
}

/// Errors a handler can't recover from; clients only ever see `internal`.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Sessions(#[from] SessionError),
    #[error(transparent)]
    Users(#[from] UserError),
    #[error("could not build a header: {0}")]
    Header(#[from] InvalidHeaderValue),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!(error = %self, "request failed");
        ProblemType::Internal.into_response()
    }
}
