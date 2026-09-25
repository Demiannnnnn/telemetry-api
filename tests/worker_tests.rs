//! # End-to-End Worker Management & Consent Tests
//!
//! Validates worker provisioning by administrator, worker authentication,
//! consent flag updates, and enforcement of the office worker location constraint.

mod common;

use axum::http::{Method, StatusCode};
use common::{send_request, setup_test_app};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn test_worker_lifecycle_and_consent_restrictions() {
    let (app, _state) = setup_test_app().await;

    // 1. Setup Admin
    let admin_email = format!("admin_worker_test_{}@example.com", Uuid::new_v4());
    let password = "AdminPassword123!";

    let (status, reg_body) = send_request(
        &app,
        Method::POST,
        "/api/v1/auth/register",
        None,
        Some(json!({
            "email": admin_email,
            "password": password,
            "name": "Admin Tester",
            "organization": "Tester Corp"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let admin_token = reg_body["data"]["access_token"]
        .as_str()
        .expect("Admin access token")
        .to_string();

    // 2. Admin creates an OFFICE worker
    let office_worker_email = format!("office_worker_{}@example.com", Uuid::new_v4());
    let device_id = format!("device_{}", Uuid::new_v4());

    let (status, create_body) = send_request(
        &app,
        Method::POST,
        "/api/v1/workers",
        Some(&admin_token),
        Some(json!({
            "email": office_worker_email,
            "name": "Office Employee",
            "device_identifier": device_id,
            "role_type": "OFFICE",
            "work_hours_start": "09:00:00",
            "work_hours_end": "17:00:00",
            "timezone": "America/Santiago"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let office_worker_id = create_body["data"]["id"]
        .as_str()
        .expect("Worker ID")
        .to_string();

    // 3. Worker logs in
    let (status, worker_login_body) = send_request(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        None,
        Some(json!({
            "email": office_worker_email,
            "device_identifier": device_id,
            "role": "WORKER"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let worker_token = worker_login_body["data"]["access_token"]
        .as_str()
        .expect("Worker access token")
        .to_string();

    // 4. OFFICE Worker attempts to enable location consent -> MUST FAIL (400)
    let consent_uri = format!("/api/v1/workers/{}/consent", office_worker_id);
    let (status, fail_body) = send_request(
        &app,
        Method::PATCH,
        &consent_uri,
        Some(&worker_token),
        Some(json!({
            "consent_flags": {
                "system_activity": true,
                "network_activity": false,
                "productivity_basic": true,
                "productivity_screenshots": false,
                "file_activity": false,
                "location": true
            }
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(fail_body["error"]["code"], "VALIDATION_ERROR");
    assert!(fail_body["error"]["message"]
        .as_str()
        .unwrap_or("")
        .contains("OFFICE role cannot consent to location tracking"));

    // 5. Worker updates consent with valid flags (location: false) -> MUST SUCCEED (200)
    let (status, success_body) = send_request(
        &app,
        Method::PATCH,
        &consent_uri,
        Some(&worker_token),
        Some(json!({
            "consent_flags": {
                "system_activity": true,
                "network_activity": true,
                "productivity_basic": true,
                "productivity_screenshots": false,
                "file_activity": true,
                "location": false
            }
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        success_body["data"]["system_activity"].as_bool(),
        Some(true)
    );
    assert_eq!(
        success_body["data"]["network_activity"].as_bool(),
        Some(true)
    );
    assert_eq!(success_body["data"]["location"].as_bool(), Some(false));

    // 6. Admin creates a FIELD worker and FIELD worker successfully enables location
    let field_worker_email = format!("field_worker_{}@example.com", Uuid::new_v4());
    let field_device_id = format!("field_device_{}", Uuid::new_v4());

    let (status, create_field_body) = send_request(
        &app,
        Method::POST,
        "/api/v1/workers",
        Some(&admin_token),
        Some(json!({
            "email": field_worker_email,
            "name": "Field Technician",
            "device_identifier": field_device_id,
            "role_type": "FIELD",
            "work_hours_start": "08:00:00",
            "work_hours_end": "16:00:00",
            "timezone": "America/Santiago"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let field_worker_id = create_field_body["data"]["id"]
        .as_str()
        .expect("Field worker ID")
        .to_string();

    let (status, field_login_body) = send_request(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        None,
        Some(json!({
            "email": field_worker_email,
            "device_identifier": field_device_id,
            "role": "WORKER"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let field_token = field_login_body["data"]["access_token"]
        .as_str()
        .expect("Field worker access token")
        .to_string();

    let field_consent_uri = format!("/api/v1/workers/{}/consent", field_worker_id);
    let (status, field_consent_body) = send_request(
        &app,
        Method::PATCH,
        &field_consent_uri,
        Some(&field_token),
        Some(json!({
            "consent_flags": {
                "system_activity": true,
                "network_activity": false,
                "productivity_basic": false,
                "productivity_screenshots": false,
                "file_activity": false,
                "location": true
            }
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field_consent_body["data"]["location"].as_bool(), Some(true));
}
