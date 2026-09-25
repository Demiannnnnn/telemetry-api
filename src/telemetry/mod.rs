//! # Telemetry Module
//!
//! Manages end-to-end encrypted device telemetry across five categories:
//! 1. `system_activity` — OS metrics, active windows, CPU/RAM/network usage.
//! 2. `network_activity` — Browsing domains and URL access.
//! 3. `productivity` — Keystroke/click frequencies and periodic screenshots.
//! 4. `file_activity` — Corporate directory file access logs.
//! 5. `location` — GPS coordinates during active work hours for field personnel.

pub mod file_activity;
pub mod handlers;
pub mod location;
pub mod models;
pub mod network_activity;
pub mod productivity;
pub mod routes;
pub mod system_activity;

pub use handlers::*;
pub use models::*;
pub use routes::router;

#[cfg(test)]
mod tests {
    use super::{location, network_activity, productivity, system_activity};
    use crate::{
        config::Config,
        crypto::UserRole,
        database::init_pool,
        middleware::{AuthUser, RequireAdmin, RequireWorker},
        telemetry::models::{
            BatchProductivitySubmission, BatchTelemetrySubmission, ProductivityRecordDto,
            ProductivitySubcategory, TelemetryQueryParams, TelemetryRecordDto,
        },
        worker::models::ConsentFlags,
        AppState,
    };
    use axum::extract::{Query, State};
    use axum::Json;
    use chrono::{Duration, NaiveTime, Utc};
    use std::sync::Arc;
    use uuid::Uuid;
    use validator::Validate;

    async fn setup_app_state() -> Option<Arc<AppState>> {
        let config = Config::from_env().ok()?;
        let pool = init_pool(&config).await.ok()?;
        Some(Arc::new(AppState::new(pool, config)))
    }

    #[test]
    fn test_telemetry_batch_empty_records_returns_validation_error() {
        let submission = BatchTelemetrySubmission { records: vec![] };
        assert!(submission.validate().is_err());
    }

    #[test]
    fn test_telemetry_batch_exceeding_100_records_fails() {
        let records = (0..101)
            .map(|_| TelemetryRecordDto {
                encrypted_payload: "cGF5bG9hZA==".to_string(),
                client_timestamp: Utc::now(),
            })
            .collect();
        let submission = BatchTelemetrySubmission { records };
        assert!(submission.validate().is_err());
    }

    #[tokio::test]
    async fn test_telemetry_future_timestamp_rejected() {
        let Some(state) = setup_app_state().await else {
            eprintln!("Skipping DB test: PostgreSQL not reachable");
            return;
        };

        // Create Admin & Worker with consent
        let admin_id: (Uuid,) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization) VALUES ($1, 'hash', 'Admin', 'Org') RETURNING id"
        )
        .bind(format!("admin_{}@example.com", Uuid::new_v4()))
        .fetch_one(&state.db)
        .await
        .expect("Admin");

        let worker_email = format!("worker_{}@example.com", Uuid::new_v4());
        let consent = serde_json::to_value(ConsentFlags {
            system_activity: true,
            ..Default::default()
        })
        .unwrap();

        let worker_id: (Uuid,) = sqlx::query_as(
            "INSERT INTO workers (admin_id, email, name, device_identifier, role_type, consent_flags)
             VALUES ($1, $2, 'Worker', 'dev', 'OFFICE', $3) RETURNING id",
        )
        .bind(admin_id.0)
        .bind(&worker_email)
        .bind(consent)
        .fetch_one(&state.db)
        .await
        .expect("Worker");

        let worker_auth = RequireWorker(AuthUser {
            id: worker_id.0,
            role: UserRole::Worker,
            email: worker_email,
        });

        // 10 minutes in the future
        let submission = BatchTelemetrySubmission {
            records: vec![TelemetryRecordDto {
                encrypted_payload: "cGF5bG9hZA==".to_string(),
                client_timestamp: Utc::now() + Duration::minutes(10),
            }],
        };

        let result =
            system_activity::ingest_system_activity(worker_auth, State(state), Json(submission))
                .await;

        assert!(
            matches!(result, Err(crate::errors::AppError::Validation(msg)) if msg.contains("future"))
        );
    }

    #[tokio::test]
    async fn test_telemetry_ingest_without_consent_returns_403() {
        let Some(state) = setup_app_state().await else {
            eprintln!("Skipping DB test: PostgreSQL not reachable");
            return;
        };

        let admin_id: (Uuid,) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization) VALUES ($1, 'hash', 'Admin', 'Org') RETURNING id"
        )
        .bind(format!("admin_{}@example.com", Uuid::new_v4()))
        .fetch_one(&state.db)
        .await
        .expect("Admin");

        let worker_email = format!("worker_{}@example.com", Uuid::new_v4());
        let worker_id: (Uuid,) = sqlx::query_as(
            "INSERT INTO workers (admin_id, email, name, device_identifier, role_type)
             VALUES ($1, $2, 'Worker', 'dev', 'OFFICE') RETURNING id",
        )
        .bind(admin_id.0)
        .bind(&worker_email)
        .fetch_one(&state.db)
        .await
        .expect("Worker");

        let worker_auth = RequireWorker(AuthUser {
            id: worker_id.0,
            role: UserRole::Worker,
            email: worker_email,
        });

        let submission = BatchTelemetrySubmission {
            records: vec![TelemetryRecordDto {
                encrypted_payload: "cGF5bG9hZA==".to_string(),
                client_timestamp: Utc::now(),
            }],
        };

        let result =
            system_activity::ingest_system_activity(worker_auth, State(state), Json(submission))
                .await;

        assert!(
            matches!(result, Err(crate::errors::AppError::Forbidden(msg)) if msg.contains("consent")),
            "Ingestion without consent must return 403"
        );
    }

    #[tokio::test]
    async fn test_telemetry_admin_retrieval_returns_correct_worker_records() {
        let Some(state) = setup_app_state().await else {
            eprintln!("Skipping DB test: PostgreSQL not reachable");
            return;
        };

        let admin_id: (Uuid,) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization) VALUES ($1, 'hash', 'Admin', 'Org') RETURNING id"
        )
        .bind(format!("admin_{}@example.com", Uuid::new_v4()))
        .fetch_one(&state.db)
        .await
        .expect("Admin");

        let worker_email = format!("worker_{}@example.com", Uuid::new_v4());
        let consent = serde_json::to_value(ConsentFlags {
            network_activity: true,
            ..Default::default()
        })
        .unwrap();

        let worker_id: (Uuid,) = sqlx::query_as(
            "INSERT INTO workers (admin_id, email, name, device_identifier, role_type, consent_flags)
             VALUES ($1, $2, 'Worker', 'dev', 'OFFICE', $3) RETURNING id",
        )
        .bind(admin_id.0)
        .bind(&worker_email)
        .bind(consent)
        .fetch_one(&state.db)
        .await
        .expect("Worker");

        let worker_auth = RequireWorker(AuthUser {
            id: worker_id.0,
            role: UserRole::Worker,
            email: worker_email,
        });

        let original_base64 = "ZW5jcnlwdGVkX25ldHdvcmtfZGF0YQ==";
        let submission = BatchTelemetrySubmission {
            records: vec![TelemetryRecordDto {
                encrypted_payload: original_base64.to_string(),
                client_timestamp: Utc::now(),
            }],
        };

        // Ingest network activity
        let (status, Json(res)) = network_activity::ingest_network_activity(
            worker_auth,
            State(state.clone()),
            Json(submission),
        )
        .await
        .expect("Ingest network");

        assert_eq!(status, axum::http::StatusCode::CREATED);
        assert_eq!(res.data.records_created, 1);

        // Query as Admin
        let admin_auth = RequireAdmin(AuthUser {
            id: admin_id.0,
            role: UserRole::Admin,
            email: "admin@example.com".to_string(),
        });

        let query_params = TelemetryQueryParams {
            worker_id: worker_id.0,
            from: None,
            to: None,
            page: Some(1),
            per_page: Some(10),
            subcategory: None,
        };

        let Json(query_res) =
            network_activity::query_network_activity(admin_auth, State(state), Query(query_params))
                .await
                .expect("Query network");

        assert_eq!(query_res.data.len(), 1);
        assert_eq!(query_res.data[0].encrypted_payload, original_base64);
    }

    #[tokio::test]
    async fn test_productivity_screenshot_without_screenshot_consent_fails() {
        let Some(state) = setup_app_state().await else {
            eprintln!("Skipping DB test: PostgreSQL not reachable");
            return;
        };

        let admin_id: (Uuid,) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization) VALUES ($1, 'hash', 'Admin', 'Org') RETURNING id"
        )
        .bind(format!("admin_{}@example.com", Uuid::new_v4()))
        .fetch_one(&state.db)
        .await
        .expect("Admin");

        // Worker has basic consent ONLY, screenshot is false
        let worker_email = format!("worker_{}@example.com", Uuid::new_v4());
        let consent = serde_json::to_value(ConsentFlags {
            productivity_basic: true,
            productivity_screenshots: false,
            ..Default::default()
        })
        .unwrap();

        let worker_id: (Uuid,) = sqlx::query_as(
            "INSERT INTO workers (admin_id, email, name, device_identifier, role_type, consent_flags)
             VALUES ($1, $2, 'Worker', 'dev', 'OFFICE', $3) RETURNING id",
        )
        .bind(admin_id.0)
        .bind(&worker_email)
        .bind(consent)
        .fetch_one(&state.db)
        .await
        .expect("Worker");

        let worker_auth = RequireWorker(AuthUser {
            id: worker_id.0,
            role: UserRole::Worker,
            email: worker_email,
        });

        let submission = BatchProductivitySubmission {
            records: vec![ProductivityRecordDto {
                encrypted_payload: "c2NyZWVuc2hvdF9kYXRh".to_string(),
                client_timestamp: Utc::now(),
                subcategory: ProductivitySubcategory::Screenshot,
            }],
        };

        let result =
            productivity::ingest_productivity(worker_auth, State(state), Json(submission)).await;

        assert!(
            matches!(result, Err(crate::errors::AppError::Forbidden(msg)) if msg.contains("screenshot")),
            "Screenshot submission without explicit screenshot consent must be forbidden"
        );
    }

    #[tokio::test]
    async fn test_location_office_worker_rejected() {
        let Some(state) = setup_app_state().await else {
            eprintln!("Skipping DB test: PostgreSQL not reachable");
            return;
        };

        let admin_id: (Uuid,) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization) VALUES ($1, 'hash', 'Admin', 'Org') RETURNING id"
        )
        .bind(format!("admin_{}@example.com", Uuid::new_v4()))
        .fetch_one(&state.db)
        .await
        .expect("Admin");

        let worker_email = format!("worker_{}@example.com", Uuid::new_v4());
        let worker_id: (Uuid,) = sqlx::query_as(
            "INSERT INTO workers (admin_id, email, name, device_identifier, role_type)
             VALUES ($1, $2, 'Office Worker', 'dev', 'OFFICE') RETURNING id",
        )
        .bind(admin_id.0)
        .bind(&worker_email)
        .fetch_one(&state.db)
        .await
        .expect("Worker");

        let worker_auth = RequireWorker(AuthUser {
            id: worker_id.0,
            role: UserRole::Worker,
            email: worker_email,
        });

        let submission = BatchTelemetrySubmission {
            records: vec![TelemetryRecordDto {
                encrypted_payload: "Z3BzX2RhdGE=".to_string(),
                client_timestamp: Utc::now(),
            }],
        };

        let result = location::ingest_location(worker_auth, State(state), Json(submission)).await;

        assert!(
            matches!(result, Err(crate::errors::AppError::Forbidden(msg)) if msg.contains("FIELD")),
            "Location submission from non-FIELD worker must be rejected"
        );
    }

    #[tokio::test]
    async fn test_location_ingestion_outside_work_hours_fails() {
        let Some(state) = setup_app_state().await else {
            eprintln!("Skipping DB test: PostgreSQL not reachable");
            return;
        };

        let admin_id: (Uuid,) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization) VALUES ($1, 'hash', 'Admin', 'Org') RETURNING id"
        )
        .bind(format!("admin_{}@example.com", Uuid::new_v4()))
        .fetch_one(&state.db)
        .await
        .expect("Admin");

        let worker_email = format!("worker_field_{}@example.com", Uuid::new_v4());
        let consent = serde_json::to_value(ConsentFlags {
            location: true,
            ..Default::default()
        })
        .unwrap();

        // Work hours: 09:00 to 18:00 UTC
        let worker_id: (Uuid,) = sqlx::query_as(
            "INSERT INTO workers (
                admin_id, email, name, device_identifier, role_type,
                consent_flags, work_hours_start, work_hours_end, timezone
             ) VALUES ($1, $2, 'Field Worker', 'dev', 'FIELD', $3, '09:00:00', '18:00:00', 'UTC')
             RETURNING id",
        )
        .bind(admin_id.0)
        .bind(&worker_email)
        .bind(consent)
        .fetch_one(&state.db)
        .await
        .expect("Worker");

        let worker_auth = RequireWorker(AuthUser {
            id: worker_id.0,
            role: UserRole::Worker,
            email: worker_email,
        });

        // Yesterday at 22:00:00 UTC (outside 09:00 - 18:00, and strictly in the past)
        let evening_ts = (Utc::now() - Duration::days(1))
            .date_naive()
            .and_time(NaiveTime::from_hms_opt(22, 0, 0).unwrap())
            .and_utc();

        let submission = BatchTelemetrySubmission {
            records: vec![TelemetryRecordDto {
                encrypted_payload: "Z3BzX2RhdGE=".to_string(),
                client_timestamp: evening_ts,
            }],
        };

        let result = location::ingest_location(worker_auth, State(state), Json(submission)).await;

        assert!(
            matches!(result, Err(crate::errors::AppError::Forbidden(msg)) if msg.contains("outside configured work hours")),
            "Location submitted outside work hours must be rejected"
        );
    }
}
