//! # Telemetry Models
//!
//! Models and DTOs for encrypted telemetry data ingestion and administrative queries.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Subcategory for productivity metrics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(
    type_name = "productivity_subcategory",
    rename_all = "SCREAMING_SNAKE_CASE"
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProductivitySubcategory {
    /// Basic metrics (keystrokes and click frequencies).
    #[default]
    Basic,
    /// Periodic screenshot capture.
    Screenshot,
}

/// Generic telemetry record payload item submitted in batches.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct TelemetryRecordDto {
    /// Base64-encoded client-encrypted binary payload.
    pub encrypted_payload: String,
    /// UTC timestamp when data was recorded on the client device.
    pub client_timestamp: DateTime<Utc>,
}

/// Batch submission wrapper for generic telemetry categories.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct BatchTelemetrySubmission {
    /// List of telemetry records (1 to 100 items).
    #[validate(length(
        min = 1,
        max = 100,
        message = "Batch must contain between 1 and 100 records"
    ))]
    pub records: Vec<TelemetryRecordDto>,
}

/// Record payload item for productivity metrics.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ProductivityRecordDto {
    /// Base64-encoded client-encrypted binary payload.
    pub encrypted_payload: String,
    /// UTC timestamp when data was recorded on the client device.
    pub client_timestamp: DateTime<Utc>,
    /// Productivity subcategory (BASIC or SCREENSHOT).
    #[serde(default)]
    pub subcategory: ProductivitySubcategory,
}

/// Batch submission wrapper for productivity metrics.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct BatchProductivitySubmission {
    /// List of productivity records (1 to 100 items).
    #[validate(length(
        min = 1,
        max = 100,
        message = "Batch must contain between 1 and 100 records"
    ))]
    pub records: Vec<ProductivityRecordDto>,
}

/// Response returned to worker upon successful batch ingestion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatchIngestionResponse {
    /// Unique batch identifier.
    pub batch_id: Uuid,
    /// Number of records successfully inserted.
    pub records_created: usize,
    /// Telemetry category ingested.
    pub category: &'static str,
    /// Server ingestion timestamp.
    pub server_timestamp: DateTime<Utc>,
}

/// Response returned to administrator querying telemetry data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelemetryRecordResponse {
    /// Telemetry record UUID.
    pub id: Uuid,
    /// Worker UUID.
    pub worker_id: Uuid,
    /// Base64-encoded encrypted payload (server never decrypts).
    pub encrypted_payload: String,
    /// Size of binary payload in bytes.
    pub payload_size: i32,
    /// Client-side timestamp.
    pub client_timestamp: DateTime<Utc>,
    /// Server-side insertion timestamp.
    pub server_timestamp: DateTime<Utc>,
    /// Batch UUID if submitted in a batch.
    pub batch_id: Option<Uuid>,
    /// Optional productivity subcategory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<ProductivitySubcategory>,
}

/// Query parameters for administrative telemetry queries.
#[derive(Debug, Clone, Deserialize)]
pub struct TelemetryQueryParams {
    /// Target worker UUID.
    pub worker_id: Uuid,
    /// Filter records on or after this timestamp.
    pub from: Option<DateTime<Utc>>,
    /// Filter records on or before this timestamp.
    pub to: Option<DateTime<Utc>>,
    /// Page number (1-based index).
    pub page: Option<u32>,
    /// Items per page (1 to 100).
    pub per_page: Option<u32>,
    /// Optional subcategory filter for productivity queries.
    pub subcategory: Option<ProductivitySubcategory>,
}
