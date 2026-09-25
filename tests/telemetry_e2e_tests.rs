//! # End-to-End Telemetry Ingestion and Retrieval Tests
//!
//! Validates worker batch submission across telemetry categories, admin retrieval,
//! and end-to-end zero-knowledge ciphertext payload integrity preservation.

mod common;

use axum::http::{Method, StatusCode};
use base64::Engine;
use chrono::Utc;
use common::{send_request, setup_test_app};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn test_telemetry_batch_submission_and_payload_integrity() {
    let (app, _state) = setup_test_app().await;

    // 1. Setup Admin
    let admin_email = format!("telemetry_admin_{}@example.com", Uuid::new_v4());
    let password = "AdminPassword123!";

    let (status, reg_body) = send_request(
        &app,
        Method::POST,
        "/api/v1/auth/register",
        None,
        Some(json!({
            "email": admin_email,
            "password": password,
            "name": "Telemetry Admin",
            "organization": "Telemetry Enterprise"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let admin_token = reg_body["data"]["access_token"]
        .as_str()
        .expect("Admin access token")
        .to_string();

    // 2. Admin creates Worker
    let worker_email = format!("telemetry_worker_{}@example.com", Uuid::new_v4());
    let device_id = format!("device_{}", Uuid::new_v4());

    let (status, create_body) = send_request(
        &app,
        Method::POST,
        "/api/v1/workers",
        Some(&admin_token),
        Some(json!({
            "email": worker_email,
            "name": "Telemetry Worker",
            "device_identifier": device_id,
            "role_type": "FIELD",
            "work_hours_start": "08:00:00",
            "work_hours_end": "18:00:00",
            "timezone": "America/Santiago"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let worker_id = create_body["data"]["id"]
        .as_str()
        .expect("Worker ID")
        .to_string();

    // 3. Worker logs in
    let (status, login_body) = send_request(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        None,
        Some(json!({
            "email": worker_email,
            "device_identifier": device_id,
            "role": "WORKER"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let worker_token = login_body["data"]["access_token"]
        .as_str()
        .expect("Worker access token")
        .to_string();

    // 4. Worker grants consent for system, network, and file activity
    let consent_uri = format!("/api/v1/workers/{}/consent", worker_id);
    let (status, _) = send_request(
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

    // 5. Worker submits a batch of encrypted system activity
    // Note: The encrypted payload is an opaque AES-GCM ciphertext blob represented as base64
    let raw_ciphertext_bytes = b"EXACT_ENCRYPTED_CLIENT_SIDE_CIPHERTEXT_BLOB_ZERO_KNOWLEDGE";
    let base64_payload = base64::engine::general_purpose::STANDARD.encode(raw_ciphertext_bytes);
    let recorded_at = Utc::now() - chrono::Duration::minutes(2);

    let (status, submit_res) = send_request(
        &app,
        Method::POST,
        "/api/v1/telemetry/system",
        Some(&worker_token),
        Some(json!({
            "records": [
                {
                    "client_timestamp": recorded_at,
                    "encrypted_payload": base64_payload
                }
            ]
        })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(submit_res["data"]["records_created"].as_u64(), Some(1));
    assert_eq!(
        submit_res["data"]["category"].as_str(),
        Some("system_activity")
    );
    let batch_id = submit_res["data"]["batch_id"]
        .as_str()
        .expect("Batch ID")
        .to_string();

    // 6. Admin queries the worker's system activity
    let query_uri = format!("/api/v1/telemetry/system?worker_id={}", worker_id);
    let (status, query_res) =
        send_request(&app, Method::GET, &query_uri, Some(&admin_token), None).await;

    assert_eq!(status, StatusCode::OK);
    let records = query_res["data"]
        .as_array()
        .expect("Data array in response");
    assert!(!records.is_empty());

    let found_record = records
        .iter()
        .find(|r| r["batch_id"].as_str() == Some(&batch_id))
        .expect("Ingested record found in query");

    let ingested_id = found_record["id"]
        .as_str()
        .expect("Ingested record ID")
        .to_string();

    // Zero-Knowledge payload integrity assertion:
    // The exact base64 encrypted payload retrieved by the admin must match what the worker uploaded.
    let retrieved_base64 = found_record["encrypted_payload"]
        .as_str()
        .expect("Base64 payload string");
    assert_eq!(retrieved_base64, base64_payload);

    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(retrieved_base64)
        .expect("Decode retrieved base64");
    assert_eq!(decoded_bytes, raw_ciphertext_bytes);

    // 7. Verify single record retrieval by ID
    let single_uri = format!("/api/v1/telemetry/system/{}", ingested_id);
    let (status, single_res) =
        send_request(&app, Method::GET, &single_uri, Some(&admin_token), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        single_res["data"]["encrypted_payload"].as_str(),
        Some(base64_payload.as_str())
    );

    // 8. Delete record by ID
    let (status, _) =
        send_request(&app, Method::DELETE, &single_uri, Some(&admin_token), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // 9. Verify record no longer exists
    let (status, not_found_res) =
        send_request(&app, Method::GET, &single_uri, Some(&admin_token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(not_found_res["error"]["code"], "RESOURCE_NOT_FOUND");
}
