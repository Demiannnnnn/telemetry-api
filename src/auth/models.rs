//! # Auth Models
//!
//! Request and response DTOs for user authentication and token rotation.

use crate::crypto::UserRole;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request payload for administrator registration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegisterRequest {
    /// Administrator email address.
    pub email: String,
    /// Plaintext password (must meet complexity requirements).
    pub password: String,
    /// Administrator full name.
    pub name: String,
    /// Company or organization name.
    pub organization: String,
}

/// Request payload for authentication (both Admin and Worker).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginRequest {
    /// Account email address.
    pub email: String,
    /// Plaintext password (required for Admin, optional for Worker if using device_identifier).
    pub password: Option<String>,
    /// Device identifier (for Worker login).
    pub device_identifier: Option<String>,
    /// Role being logged into.
    pub role: UserRole,
}

/// Request payload to refresh an expired access token.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RefreshTokenRequest {
    /// Current single-use refresh token.
    pub refresh_token: String,
}

/// Request payload to revoke a refresh token on logout.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogoutRequest {
    /// Refresh token to revoke.
    pub refresh_token: String,
}

/// Authenticated user summary included in authentication responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserInfo {
    /// User UUID.
    pub id: Uuid,
    /// User email address.
    pub email: String,
    /// User full name.
    pub name: String,
    /// Role assigned to the user.
    pub role: UserRole,
}

/// Response returned on successful login, registration, or token refresh.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthResponse {
    /// JWT access token (Bearer format).
    pub access_token: String,
    /// Single-use refresh token.
    pub refresh_token: String,
    /// Token type indicator (always "Bearer").
    pub token_type: String,
    /// Lifetime of access token in seconds.
    pub expires_in: i64,
    /// Authenticated user profile information.
    pub user: UserInfo,
}
