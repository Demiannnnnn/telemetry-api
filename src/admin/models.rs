//! # Admin Models
//!
//! Database models and DTOs for administrator profiles and updates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Internal representation of a row in the admins table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Admin {
    /// Unique administrator UUID.
    pub id: Uuid,
    /// Administrator email address.
    pub email: String,
    /// Secure bcrypt password hash.
    pub password_hash: String,
    /// Administrator display name.
    pub name: String,
    /// Company or organization name.
    pub organization: String,
    /// Account status.
    pub is_active: bool,
    /// Account creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Soft-delete timestamp if deleted.
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Request DTO for updating administrator profile details.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateAdminRequest {
    /// Optional updated name (2 to 255 characters).
    #[validate(length(
        min = 2,
        max = 255,
        message = "Name must be between 2 and 255 characters"
    ))]
    pub name: Option<String>,

    /// Optional updated organization (2 to 255 characters).
    #[validate(length(
        min = 2,
        max = 255,
        message = "Organization must be between 2 and 255 characters"
    ))]
    pub organization: Option<String>,
}

/// Public response DTO for administrator profile inquiries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminProfileResponse {
    /// Administrator UUID.
    pub id: Uuid,
    /// Administrator email address.
    pub email: String,
    /// Administrator full name.
    pub name: String,
    /// Associated organization.
    pub organization: String,
    /// Whether the administrator account is currently active.
    pub is_active: bool,
    /// Count of active workers managed by this administrator.
    pub worker_count: i64,
    /// Account creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Account last update timestamp.
    pub updated_at: DateTime<Utc>,
}
