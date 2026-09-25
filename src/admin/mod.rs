//! # Admin Module
//!
//! Handles administrator profile management and aggregate worker statistics.

pub mod handlers;
pub mod models;
pub mod routes;

pub use models::*;
pub use routes::router;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        admin::models::{AdminProfileResponse, UpdateAdminRequest},
        config::Config,
        crypto::UserRole,
        database::init_pool,
        middleware::{AuthUser, RequireAdmin},
        AppState,
    };
    use axum::extract::State;
    use axum::Json;
    use chrono::Utc;
    use std::sync::Arc;
    use uuid::Uuid;
    use validator::Validate;

    async fn setup_app_state() -> Option<Arc<AppState>> {
        let config = Config::from_env().ok()?;
        let pool = init_pool(&config).await.ok()?;
        Some(Arc::new(AppState::new(pool, config)))
    }

    #[test]
    fn test_admin_update_validation_short_name_fails() {
        let req = UpdateAdminRequest {
            name: Some("A".to_string()),
            organization: Some("Valid Org".to_string()),
        };
        assert!(
            req.validate().is_err(),
            "1 character name must fail validation"
        );

        let valid_req = UpdateAdminRequest {
            name: Some("Valid Name".to_string()),
            organization: Some("Valid Org".to_string()),
        };
        assert!(valid_req.validate().is_ok());
    }

    #[test]
    fn test_admin_profile_response_serialization_excludes_password() {
        let profile = AdminProfileResponse {
            id: Uuid::new_v4(),
            email: "admin@example.com".to_string(),
            name: "Admin User".to_string(),
            organization: "Acme Corp".to_string(),
            is_active: true,
            worker_count: 5,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&profile).expect("serialize");
        assert!(!json.contains("password"));
        assert!(!json.contains("password_hash"));
        assert!(json.contains("worker_count"));
    }

    #[tokio::test]
    async fn test_admin_get_me_returns_profile_and_worker_count() {
        let Some(state) = setup_app_state().await else {
            eprintln!("Skipping DB test: PostgreSQL not reachable");
            return;
        };

        let email = format!("admin_{}@example.com", Uuid::new_v4());
        let admin_row: (Uuid,) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization) VALUES ($1, 'hash', 'Test Admin', 'Acme') RETURNING id"
        )
        .bind(&email)
        .fetch_one(&state.db)
        .await
        .expect("Admin creation");

        let admin_id = admin_row.0;

        // Insert 2 active workers and 1 inactive worker
        for i in 1..=2 {
            sqlx::query(
                "INSERT INTO workers (admin_id, email, name, device_identifier, role_type, is_active)
                 VALUES ($1, $2, 'Active Worker', $3, 'OFFICE', TRUE)",
            )
            .bind(admin_id)
            .bind(format!("worker_{}_{}@example.com", i, Uuid::new_v4()))
            .bind(format!("dev_{}", i))
            .execute(&state.db)
            .await
            .expect("Worker creation");
        }

        sqlx::query(
            "INSERT INTO workers (admin_id, email, name, device_identifier, role_type, is_active)
             VALUES ($1, $2, 'Inactive Worker', 'dev_inactive', 'OFFICE', FALSE)",
        )
        .bind(admin_id)
        .bind(format!("worker_inactive_{}@example.com", Uuid::new_v4()))
        .execute(&state.db)
        .await
        .expect("Inactive worker creation");

        let auth_user = RequireAdmin(AuthUser {
            id: admin_id,
            role: UserRole::Admin,
            email,
        });

        let Json(res) = handlers::get_me(auth_user, State(state))
            .await
            .expect("get_me should succeed");

        assert_eq!(res.data.id, admin_id);
        assert_eq!(
            res.data.worker_count, 2,
            "Only active workers must be counted"
        );
    }

    #[tokio::test]
    async fn test_admin_update_me_modifies_name_and_updates_timestamp() {
        let Some(state) = setup_app_state().await else {
            eprintln!("Skipping DB test: PostgreSQL not reachable");
            return;
        };

        let email = format!("admin_update_{}@example.com", Uuid::new_v4());
        let admin_row: (Uuid, chrono::DateTime<Utc>) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization) VALUES ($1, 'hash', 'Original Name', 'Original Org') RETURNING id, updated_at"
        )
        .bind(&email)
        .fetch_one(&state.db)
        .await
        .expect("Admin creation");

        let admin_id = admin_row.0;
        let original_updated_at = admin_row.1;

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let auth_user = RequireAdmin(AuthUser {
            id: admin_id,
            role: UserRole::Admin,
            email,
        });

        let update_req = UpdateAdminRequest {
            name: Some("New Name".to_string()),
            organization: Some("New Org".to_string()),
        };

        let Json(res) = handlers::update_me(auth_user, State(state), Json(update_req))
            .await
            .expect("update_me should succeed");

        assert_eq!(res.data.name, "New Name");
        assert_eq!(res.data.organization, "New Org");
        assert!(res.data.updated_at > original_updated_at);
    }
}
