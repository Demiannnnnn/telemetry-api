//! # Worker Module
//!
//! Handles worker enrollment, device authorization, operational profile management,
//! and worker-controlled privacy consent matrices.

pub mod handlers;
pub mod models;
pub mod routes;

pub use models::*;
pub use routes::router;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        crypto::UserRole,
        database::init_pool,
        middleware::{AuthUser, RequireAdmin, RequireWorker},
        worker::models::{
            ConsentFlags, CreateWorkerRequest, UpdateConsentRequest, WorkerQueryParams,
            WorkerRoleType,
        },
        AppState,
    };
    use axum::extract::{Path, Query, State};
    use axum::Json;
    use chrono::NaiveTime;
    use std::sync::Arc;
    use uuid::Uuid;

    async fn setup_app_state() -> Option<Arc<AppState>> {
        let config = Config::from_env().ok()?;
        let pool = init_pool(&config).await.ok()?;
        Some(Arc::new(AppState::new(pool, config)))
    }

    #[tokio::test]
    async fn test_worker_create_with_invalid_hours_returns_validation_error() {
        let Some(state) = setup_app_state().await else {
            eprintln!("Skipping DB test: PostgreSQL not reachable");
            return;
        };

        let admin_auth = RequireAdmin(AuthUser {
            id: Uuid::new_v4(),
            role: UserRole::Admin,
            email: "admin@example.com".to_string(),
        });

        let req = CreateWorkerRequest {
            email: "invalid_hours@example.com".to_string(),
            name: "Invalid Hours Worker".to_string(),
            device_identifier: "dev123".to_string(),
            role_type: WorkerRoleType::Office,
            work_hours_start: Some(NaiveTime::from_hms_opt(18, 0, 0).unwrap()),
            work_hours_end: Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap()),
            timezone: None,
        };

        let result = handlers::create_worker(admin_auth, State(state), Json(req)).await;
        assert!(matches!(
            result,
            Err(crate::errors::AppError::Validation(_))
        ));
    }

    #[tokio::test]
    async fn test_worker_consent_office_enabling_location_fails() {
        let Some(state) = setup_app_state().await else {
            eprintln!("Skipping DB test: PostgreSQL not reachable");
            return;
        };

        // Create admin first
        let admin_email = format!("admin_{}@example.com", Uuid::new_v4());
        let admin_id: (Uuid,) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization) VALUES ($1, 'hash', 'Admin', 'Org') RETURNING id"
        )
        .bind(&admin_email)
        .fetch_one(&state.db)
        .await
        .expect("Admin creation");

        // Create OFFICE worker
        let worker_email = format!("worker_{}@example.com", Uuid::new_v4());
        let worker_id: (Uuid,) = sqlx::query_as(
            "INSERT INTO workers (admin_id, email, name, device_identifier, role_type)
             VALUES ($1, $2, 'Office Worker', 'dev-office', 'OFFICE')
             RETURNING id",
        )
        .bind(admin_id.0)
        .bind(&worker_email)
        .fetch_one(&state.db)
        .await
        .expect("Worker creation");

        let worker_auth = RequireWorker(AuthUser {
            id: worker_id.0,
            role: UserRole::Worker,
            email: worker_email,
        });

        // Try to enable location consent
        let req = UpdateConsentRequest {
            consent_flags: ConsentFlags {
                location: true,
                ..ConsentFlags::default()
            },
        };

        let result =
            handlers::update_consent(worker_auth, Path(worker_id.0), State(state), Json(req)).await;

        assert!(
            matches!(result, Err(crate::errors::AppError::Validation(msg)) if msg.contains("OFFICE role cannot consent to location")),
            "Office worker consenting to location must be rejected with 400"
        );
    }

    #[tokio::test]
    async fn test_worker_admin_cannot_access_worker_of_another_admin() {
        let Some(state) = setup_app_state().await else {
            eprintln!("Skipping DB test: PostgreSQL not reachable");
            return;
        };

        // Create Admin A and Admin B
        let admin_a: (Uuid,) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization) VALUES ($1, 'hash', 'Admin A', 'Org A') RETURNING id"
        )
        .bind(format!("admin_a_{}@example.com", Uuid::new_v4()))
        .fetch_one(&state.db)
        .await
        .expect("Admin A");

        let admin_b: (Uuid,) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization) VALUES ($1, 'hash', 'Admin B', 'Org B') RETURNING id"
        )
        .bind(format!("admin_b_{}@example.com", Uuid::new_v4()))
        .fetch_one(&state.db)
        .await
        .expect("Admin B");

        // Worker belonging to Admin A
        let worker_a: (Uuid,) = sqlx::query_as(
            "INSERT INTO workers (admin_id, email, name, device_identifier, role_type)
             VALUES ($1, $2, 'Worker A', 'dev-a', 'OFFICE')
             RETURNING id",
        )
        .bind(admin_a.0)
        .bind(format!("worker_a_{}@example.com", Uuid::new_v4()))
        .fetch_one(&state.db)
        .await
        .expect("Worker A");

        let admin_b_auth = AuthUser {
            id: admin_b.0,
            role: UserRole::Admin,
            email: "admin_b@example.com".to_string(),
        };

        // Admin B trying to read Worker A
        let result = handlers::get_worker(admin_b_auth, Path(worker_a.0), State(state)).await;
        assert!(
            matches!(result, Err(crate::errors::AppError::Forbidden(_))),
            "Admin B must be forbidden from accessing Worker A"
        );
    }

    #[tokio::test]
    async fn test_worker_soft_delete_excludes_from_active_list() {
        let Some(state) = setup_app_state().await else {
            eprintln!("Skipping DB test: PostgreSQL not reachable");
            return;
        };

        let admin_row: (Uuid,) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization) VALUES ($1, 'hash', 'Admin', 'Org') RETURNING id"
        )
        .bind(format!("admin_del_{}@example.com", Uuid::new_v4()))
        .fetch_one(&state.db)
        .await
        .expect("Admin");

        let admin_auth = RequireAdmin(AuthUser {
            id: admin_row.0,
            role: UserRole::Admin,
            email: "admin@example.com".to_string(),
        });

        // Create Worker
        let req = CreateWorkerRequest {
            email: format!("worker_to_delete_{}@example.com", Uuid::new_v4()),
            name: "Delete Me".to_string(),
            device_identifier: "dev_del".to_string(),
            role_type: WorkerRoleType::Field,
            work_hours_start: None,
            work_hours_end: None,
            timezone: None,
        };

        let (_, Json(created)) =
            handlers::create_worker(admin_auth.clone(), State(state.clone()), Json(req))
                .await
                .expect("Worker create");

        let worker_id = created.data.id;

        // Verify present in list
        let Json(list_before) = handlers::list_workers(
            admin_auth.clone(),
            State(state.clone()),
            Query(WorkerQueryParams::default()),
        )
        .await
        .expect("List before");

        assert!(list_before.data.iter().any(|w| w.id == worker_id));

        // Soft delete
        let status =
            handlers::delete_worker(admin_auth.clone(), Path(worker_id), State(state.clone()))
                .await
                .expect("Delete worker");
        assert_eq!(status, axum::http::StatusCode::NO_CONTENT);

        // Verify absent from active list
        let Json(list_after) = handlers::list_workers(
            admin_auth,
            State(state),
            Query(WorkerQueryParams::default()),
        )
        .await
        .expect("List after");

        assert!(!list_after.data.iter().any(|w| w.id == worker_id));
    }
}
