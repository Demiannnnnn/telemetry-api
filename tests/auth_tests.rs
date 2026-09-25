//! # End-to-End Authentication Tests
//!
//! Validates admin registration, login, token refresh rotation,
//! and token reuse breach detection with family invalidation.

mod common;

use axum::http::{Method, StatusCode};
use common::{send_request, setup_test_app};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn test_auth_full_lifecycle_and_reuse_detection() {
    let (app, state) = setup_test_app().await;

    let email = format!("e2e_admin_{}@example.com", Uuid::new_v4());
    let password = "SuperSecretPassword123!";
    let name = "E2E Administrator";
    let org = "E2E Enterprise";

    // 1. Register new administrator
    let (status, reg_body) = send_request(
        &app,
        Method::POST,
        "/api/v1/auth/register",
        None,
        Some(json!({
            "email": email,
            "password": password,
            "name": name,
            "organization": org
        })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let admin_id = reg_body["data"]["user"]["id"]
        .as_str()
        .expect("Admin ID in register response");
    assert_eq!(reg_body["data"]["user"]["email"], email);

    // 2. Login to acquire fresh Access & Refresh Tokens
    let (status, login_body) = send_request(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        None,
        Some(json!({
            "email": email,
            "password": password,
            "role": "ADMIN"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let access_token_1 = login_body["data"]["access_token"]
        .as_str()
        .expect("Access token 1")
        .to_string();
    let refresh_token_1 = login_body["data"]["refresh_token"]
        .as_str()
        .expect("Refresh token 1")
        .to_string();

    // Verify access token can query admin profile
    let (status, me_body) = send_request(
        &app,
        Method::GET,
        "/api/v1/admins/me",
        Some(&access_token_1),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me_body["data"]["email"], email);

    // 3. Valid Refresh Token Rotation
    let (status, refresh_body) = send_request(
        &app,
        Method::POST,
        "/api/v1/auth/refresh",
        None,
        Some(json!({
            "refresh_token": refresh_token_1
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let _access_token_2 = refresh_body["data"]["access_token"]
        .as_str()
        .expect("Access token 2")
        .to_string();
    let refresh_token_2 = refresh_body["data"]["refresh_token"]
        .as_str()
        .expect("Refresh token 2")
        .to_string();

    assert_ne!(refresh_token_1, refresh_token_2);

    // 4. Reuse Attempt: Trying to use refresh_token_1 again
    // This MUST trigger token theft detection and invalidate the entire token family.
    let (status, reuse_body) = send_request(
        &app,
        Method::POST,
        "/api/v1/auth/refresh",
        None,
        Some(json!({
            "refresh_token": refresh_token_1
        })),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(
        reuse_body["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("compromise")
            || reuse_body["error"]["code"] == "UNAUTHORIZED"
    );

    // 5. Verify refresh_token_2 is now also revoked due to family compromise detection
    let (status, _second_refresh_attempt) = send_request(
        &app,
        Method::POST,
        "/api/v1/auth/refresh",
        None,
        Some(json!({
            "refresh_token": refresh_token_2
        })),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Verify in database that all tokens for this admin are revoked
    let admin_uuid: Uuid = admin_id.parse().expect("Parse UUID");
    let active_tokens_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1 AND is_revoked = FALSE",
        admin_uuid
    )
    .fetch_one(&state.db)
    .await
    .expect("Query tokens")
    .unwrap_or(0);

    assert_eq!(
        active_tokens_count, 0,
        "All tokens in family must be revoked after compromise"
    );
}
