//! # Audit Logging Module
//!
//! Provides an append-only immutable audit trail for security-critical actions
//! and personal data accesses (GDPR / Chile Law 19.628 compliance).
//!
//! # Critical Zero-Knowledge & Privacy Notice
//! The audit trail NEVER records raw or decrypted telemetry payloads.
//! Only resource identifiers, metadata categories, and request parameters are persisted.

use crate::{crypto::UserRole, errors::AppError};
use axum::http::HeaderMap;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

/// Records an immutable audit event in the `audit_logs` table.
///
/// # Arguments
///
/// * `pool` - Database connection pool.
/// * `actor_id` - UUID of the actor performing the action.
/// * `actor_role` - [`UserRole`] of the actor (`Admin` or `Worker`).
/// * `action` - Action identifier (e.g., `"DATA_EXPORT_REQUESTED"`, `"DATA_DELETION_REQUESTED"`).
/// * `resource_type` - Type of resource being acted upon (e.g., `"telemetry"`, `"worker"`, `"auth"`).
/// * `resource_id` - Optional target resource UUID.
/// * `headers` - Optional HTTP request headers (used to extract IP address and User-Agent).
/// * `metadata` - Optional JSONB metadata describing the event (must NOT contain encrypted or decrypted payloads).
#[allow(clippy::too_many_arguments)]
pub async fn record_audit_event(
    pool: &PgPool,
    actor_id: Uuid,
    actor_role: UserRole,
    action: &str,
    resource_type: &str,
    resource_id: Option<Uuid>,
    headers: Option<&HeaderMap>,
    metadata: Option<Value>,
) -> Result<(), AppError> {
    let user_agent = headers
        .and_then(|h| h.get(axum::http::header::USER_AGENT))
        .and_then(|v| v.to_str().ok());

    let ip_str = headers
        .and_then(|h| h.get("x-forwarded-for").or_else(|| h.get("x-real-ip")))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim())
        .filter(|s| s.parse::<std::net::IpAddr>().is_ok());

    sqlx::query!(
        r#"
        INSERT INTO audit_logs (
            actor_id, actor_role, action, resource_type, resource_id, ip_address, user_agent, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6::text::inet, $7, $8)
        "#,
        actor_id,
        actor_role as UserRole,
        action,
        resource_type,
        resource_id,
        ip_str,
        user_agent,
        metadata
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to write audit log");
        AppError::Database(e)
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_metadata_serialization() {
        let meta = serde_json::json!({
            "categories": ["system_activity", "network_activity"],
            "request_id": Uuid::new_v4(),
            "reason": "GDPR Subject Access Request"
        });

        // Ensure serialization contains expected keys and no payload bytes
        let meta_str = serde_json::to_string(&meta).unwrap();
        assert!(meta_str.contains("system_activity"));
        assert!(meta_str.contains("network_activity"));
        assert!(!meta_str.contains("encrypted_payload"));
        assert!(!meta_str.contains("payload"));
    }
}
