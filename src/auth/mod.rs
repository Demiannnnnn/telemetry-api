//! # Auth Module
//!
//! Handles administrator registration, user authentication, token generation,
//! refresh token rotation, and credential verification.

pub mod handlers;
pub mod models;
pub mod routes;

pub use models::*;
pub use routes::router;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        auth::models::{LoginRequest, RefreshTokenRequest, RegisterRequest},
        config::Config,
        crypto::UserRole,
        database::init_pool,
        AppState,
    };
    use axum::extract::State;
    use axum::Json;
    use std::sync::Arc;

    async fn setup_app_state() -> Option<Arc<AppState>> {
        let config = Config::from_env().ok()?;
        let pool = init_pool(&config).await.ok()?;
        Some(Arc::new(AppState::new(pool, config)))
    }

    #[tokio::test]
    async fn test_auth_login_with_valid_credentials_returns_tokens() {
        let Some(state) = setup_app_state().await else {
            eprintln!("Skipping database test: PostgreSQL not reachable");
            return;
        };

        let unique_email = format!("admin_{}@example.com", uuid::Uuid::new_v4());
        let reg_req = RegisterRequest {
            email: unique_email.clone(),
            password: "StrongPassword123!".to_string(),
            name: "John Doe".to_string(),
            organization: "Acme Corp".to_string(),
        };

        // 1. Register
        let (status, Json(res)) = handlers::register_handler(State(state.clone()), Json(reg_req))
            .await
            .expect("Registration should succeed");

        assert_eq!(status, axum::http::StatusCode::CREATED);
        assert!(!res.data.access_token.is_empty());
        assert!(!res.data.refresh_token.is_empty());
        assert_eq!(res.data.user.email, unique_email);

        // 2. Login
        let login_req = LoginRequest {
            email: unique_email,
            password: Some("StrongPassword123!".to_string()),
            device_identifier: None,
            role: UserRole::Admin,
        };

        let Json(login_res) = handlers::login_handler(State(state), Json(login_req))
            .await
            .expect("Login should succeed");

        assert!(!login_res.data.access_token.is_empty());
        assert!(!login_res.data.refresh_token.is_empty());
    }

    #[tokio::test]
    async fn test_auth_refresh_token_rotation_and_invalidation() {
        let Some(state) = setup_app_state().await else {
            eprintln!("Skipping database test: PostgreSQL not reachable");
            return;
        };

        let unique_email = format!("admin_refresh_{}@example.com", uuid::Uuid::new_v4());
        let reg_req = RegisterRequest {
            email: unique_email,
            password: "StrongPassword123!".to_string(),
            name: "Jane Doe".to_string(),
            organization: "Acme Corp".to_string(),
        };

        let (_, Json(reg_res)) = handlers::register_handler(State(state.clone()), Json(reg_req))
            .await
            .expect("Registration should succeed");

        let first_refresh_token = reg_res.data.refresh_token;

        // 1. First refresh -> Should succeed and return new tokens
        let Json(refresh_res_1) = handlers::refresh_handler(
            State(state.clone()),
            Json(RefreshTokenRequest {
                refresh_token: first_refresh_token.clone(),
            }),
        )
        .await
        .expect("First refresh must succeed");

        let second_refresh_token = refresh_res_1.data.refresh_token;
        assert_ne!(first_refresh_token, second_refresh_token);

        // 2. Reuse of first refresh token -> Must trigger security alert and fail
        let reuse_err = handlers::refresh_handler(
            State(state.clone()),
            Json(RefreshTokenRequest {
                refresh_token: first_refresh_token,
            }),
        )
        .await;

        assert!(reuse_err.is_err(), "Reusing revoked token must fail");

        // 3. Second token should also have been revoked due to family reuse invalidation
        let second_use_err = handlers::refresh_handler(
            State(state),
            Json(RefreshTokenRequest {
                refresh_token: second_refresh_token,
            }),
        )
        .await;

        assert!(
            second_use_err.is_err(),
            "All tokens in family should be invalidated after reuse attack"
        );
    }
}
