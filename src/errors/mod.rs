//! # Errors Module
//!
//! Centralized error handling for the Telemetry API.
//! Defines [`AppError`] and canonical JSON error response structures.
//! Guarantees that internal database errors and stack traces are never leaked to clients.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Canonical error payload returned in API error responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorResponse {
    /// Inner error information.
    pub error: ErrorDetails,
}

/// Detailed error breakdown.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorDetails {
    /// Machine-readable error code in SCREAMING_SNAKE_CASE.
    pub code: String,
    /// Human-readable explanation suitable for client display.
    pub message: String,
    /// Optional context-specific validation or error details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Application-wide error enumeration.
///
/// Converts into an HTTP response using [`IntoResponse`].
#[derive(Debug, Error)]
pub enum AppError {
    /// Client input failed validation rules.
    #[error("Validation failed: {0}")]
    Validation(String),

    /// Missing, invalid, or expired credentials.
    #[error("Authentication required: {0}")]
    Unauthorized(String),

    /// Authenticated user does not possess required role or ownership.
    #[error("Forbidden access: {0}")]
    Forbidden(String),

    /// Requested resource does not exist.
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Conflict with current state (e.g. duplicate email).
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Request payload exceeds server limits.
    #[error("Payload too large: {0}")]
    PayloadTooLarge(String),

    /// Client exceeded rate limit quotas.
    #[error("Too many requests")]
    RateLimited,

    /// Database query failure.
    #[error("Database error occurred: {0}")]
    Database(#[from] sqlx::Error),

    /// Unhandled internal application error.
    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            Self::Validation(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.clone()),
            Self::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg.clone()),
            Self::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg.clone()),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, "RESOURCE_NOT_FOUND", msg.clone()),
            Self::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg.clone()),
            Self::PayloadTooLarge(msg) => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "PAYLOAD_TOO_LARGE",
                msg.clone(),
            ),
            Self::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                "Rate limit exceeded".to_string(),
            ),
            Self::Database(err) => {
                tracing::error!(error = %err, "Database query failure");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal database error occurred".to_string(),
                )
            }
            Self::Internal(err) => {
                tracing::error!(error = %err, "Unhandled application error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An unexpected server error occurred".to_string(),
                )
            }
        };

        let body = Json(ErrorResponse {
            error: ErrorDetails {
                code: code.to_string(),
                message,
                details: None,
            },
        });

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_app_error_into_response_mapping() {
        let err = AppError::NotFound("Worker with ID '123' not found".to_string());
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body_bytes = to_bytes(response.into_body(), 1024 * 10)
            .await
            .expect("should read body");
        let parsed: ErrorResponse =
            serde_json::from_slice(&body_bytes).expect("should parse JSON response");

        assert_eq!(parsed.error.code, "RESOURCE_NOT_FOUND");
        assert_eq!(parsed.error.message, "Worker with ID '123' not found");
        assert_eq!(parsed.error.details, None);
    }

    #[tokio::test]
    async fn test_validation_error_into_response() {
        let err = AppError::Validation("Invalid email format".to_string());
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body_bytes = to_bytes(response.into_body(), 1024 * 10)
            .await
            .expect("should read body");
        let parsed: ErrorResponse =
            serde_json::from_slice(&body_bytes).expect("should parse JSON response");

        assert_eq!(parsed.error.code, "VALIDATION_ERROR");
        assert_eq!(parsed.error.message, "Invalid email format");
    }

    #[tokio::test]
    async fn test_database_error_masks_details() {
        // sqlx RowNotFound error
        let err = AppError::Database(sqlx::Error::RowNotFound);
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body_bytes = to_bytes(response.into_body(), 1024 * 10)
            .await
            .expect("should read body");
        let parsed: ErrorResponse =
            serde_json::from_slice(&body_bytes).expect("should parse JSON response");

        assert_eq!(parsed.error.code, "INTERNAL_ERROR");
        assert_eq!(parsed.error.message, "An internal database error occurred");
    }
}
