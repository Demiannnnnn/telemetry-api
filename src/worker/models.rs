//! # Worker Models
//!
//! Database models, DTOs, and query parameters for worker management and consent flags.

use chrono::{DateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Worker deployment role type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "worker_role_type", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkerRoleType {
    /// Office-bound worker (location tracking strictly forbidden).
    Office,
    /// Field worker (location tracking permitted during work hours with consent).
    Field,
}

/// Granular privacy consent flags controlled exclusively by the worker.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsentFlags {
    /// Consent for system activity monitoring (idle time, applications).
    pub system_activity: bool,
    /// Consent for network domain monitoring.
    pub network_activity: bool,
    /// Consent for basic productivity frequency metrics (clicks, keystroke count).
    pub productivity_basic: bool,
    /// Consent for periodic screenshot capture.
    pub productivity_screenshots: bool,
    /// Consent for corporate directory file access logging.
    pub file_activity: bool,
    /// Consent for GPS geolocation tracking (FIELD role only).
    pub location: bool,
}

/// Request DTO for creating a new worker account.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateWorkerRequest {
    /// Worker email address.
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    /// Worker full name.
    #[validate(length(
        min = 2,
        max = 255,
        message = "Name must be between 2 and 255 characters"
    ))]
    pub name: String,

    /// Device serial or identifier string.
    #[validate(length(
        min = 2,
        max = 512,
        message = "Device identifier must be between 2 and 512 characters"
    ))]
    pub device_identifier: String,

    /// Worker role type (OFFICE or FIELD).
    pub role_type: WorkerRoleType,

    /// Optional start of work hours (e.g. 09:00:00).
    pub work_hours_start: Option<NaiveTime>,

    /// Optional end of work hours (e.g. 18:00:00).
    pub work_hours_end: Option<NaiveTime>,

    /// Local IANA timezone (defaults to America/Santiago).
    pub timezone: Option<String>,
}

/// Request DTO for updating existing worker profile details.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateWorkerRequest {
    /// Updated name.
    #[validate(length(
        min = 2,
        max = 255,
        message = "Name must be between 2 and 255 characters"
    ))]
    pub name: Option<String>,

    /// Updated device identifier.
    #[validate(length(
        min = 2,
        max = 512,
        message = "Device identifier must be between 2 and 512 characters"
    ))]
    pub device_identifier: Option<String>,

    /// Updated worker role type.
    pub role_type: Option<WorkerRoleType>,

    /// Updated active status.
    pub is_active: Option<bool>,

    /// Updated start of work hours.
    pub work_hours_start: Option<NaiveTime>,

    /// Updated end of work hours.
    pub work_hours_end: Option<NaiveTime>,

    /// Updated timezone.
    pub timezone: Option<String>,
}

/// Request DTO for worker consent update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConsentRequest {
    /// Full matrix of consent flags.
    pub consent_flags: ConsentFlags,
}

/// Query parameters for paginated and filtered worker listings.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct WorkerQueryParams {
    /// 1-based page number.
    pub page: Option<u32>,
    /// Items per page (1 to 100).
    pub per_page: Option<u32>,
    /// Filter by active status.
    pub is_active: Option<bool>,
    /// Filter by worker role type.
    pub role_type: Option<WorkerRoleType>,
    /// Substring search for worker name or email.
    pub search: Option<String>,
}

/// Public representation of a worker record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerResponse {
    /// Worker UUID.
    pub id: Uuid,
    /// Managing administrator UUID.
    pub admin_id: Uuid,
    /// Worker email address.
    pub email: String,
    /// Worker full name.
    pub name: String,
    /// Enrolled device identifier.
    pub device_identifier: String,
    /// Worker role type.
    pub role_type: WorkerRoleType,
    /// Account status.
    pub is_active: bool,
    /// Current consent flags.
    pub consent_flags: ConsentFlags,
    /// Optional scheduled start of work hours.
    pub work_hours_start: Option<NaiveTime>,
    /// Optional scheduled end of work hours.
    pub work_hours_end: Option<NaiveTime>,
    /// Configured timezone.
    pub timezone: String,
    /// Record creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Record update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Internal database representation for worker queries.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WorkerRecord {
    /// Worker UUID.
    pub id: Uuid,
    /// Managing administrator UUID.
    pub admin_id: Uuid,
    /// Worker email address.
    pub email: String,
    /// Worker full name.
    pub name: String,
    /// Enrolled device identifier.
    pub device_identifier: String,
    /// Worker role type.
    pub role_type: WorkerRoleType,
    /// Account status.
    pub is_active: bool,
    /// Raw consent flags JSONB value.
    pub consent_flags: serde_json::Value,
    /// Optional scheduled start of work hours.
    pub work_hours_start: Option<NaiveTime>,
    /// Optional scheduled end of work hours.
    pub work_hours_end: Option<NaiveTime>,
    /// Configured timezone.
    pub timezone: String,
    /// Record creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Record update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl WorkerRecord {
    /// Converts database record to public [`WorkerResponse`].
    pub fn into_response(self) -> WorkerResponse {
        let consent_flags: ConsentFlags =
            serde_json::from_value(self.consent_flags).unwrap_or_default();

        WorkerResponse {
            id: self.id,
            admin_id: self.admin_id,
            email: self.email,
            name: self.name,
            device_identifier: self.device_identifier,
            role_type: self.role_type,
            is_active: self.is_active,
            consent_flags,
            work_hours_start: self.work_hours_start,
            work_hours_end: self.work_hours_end,
            timezone: self.timezone,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Request DTO for data portability / export request (GDPR/ARCO).
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DataExportRequest {
    /// Type of request (must be "EXPORT").
    pub r#type: String,
    /// Telemetry categories to export.
    #[validate(length(min = 1, message = "At least one category must be specified"))]
    pub categories: Vec<String>,
    /// Optional lower timestamp bound.
    pub from: Option<DateTime<Utc>>,
    /// Optional upper timestamp bound.
    pub to: Option<DateTime<Utc>>,
}

/// Response DTO for data export acceptance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRequestResponse {
    /// Unique tracking ID for the export request.
    pub request_id: Uuid,
    /// Current request processing status.
    pub status: &'static str,
    /// Estimated completion timestamp.
    pub estimated_completion: DateTime<Utc>,
}

/// Request DTO for data deletion / right to be forgotten (GDPR/ARCO).
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DataDeletionRequest {
    /// Categories of telemetry data to delete (e.g. ["all"] or specific category list).
    #[validate(length(min = 1, message = "At least one category must be specified"))]
    pub categories: Vec<String>,
    /// Stated reason for the deletion request.
    pub reason: Option<String>,
}

/// Response DTO for data deletion acceptance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDeletionResponse {
    /// Unique tracking ID for the deletion request.
    pub request_id: Uuid,
    /// Current request processing status.
    pub status: &'static str,
    /// Confirmation message detailing retention and grace period policy.
    pub message: String,
}
