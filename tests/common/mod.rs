//! # Test Utilities
//!
//! Shared test helpers, fixtures, and utilities for integration tests.

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use std::sync::Arc;
use telemetry_api::{config::Config, database::init_pool, routes::create_router, AppState};
use tower::ServiceExt;

/// Initializes test configuration, database pool, and application router.
pub async fn setup_test_app() -> (Router, Arc<AppState>) {
    dotenvy::dotenv().ok();
    let config = Config::from_env().expect("Failed to load test config");
    let pool = init_pool(&config)
        .await
        .expect("Failed to initialize test DB pool");
    let state = Arc::new(AppState::new(pool, config));
    let router = create_router(state.clone());
    (router, state)
}

/// Helper function to execute an HTTP request against the test router.
pub async fn send_request(
    app: &Router,
    method: Method,
    uri: &str,
    bearer_token: Option<&str>,
    json_body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder().method(method).uri(uri);

    if let Some(token) = bearer_token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }

    let body = if let Some(json) = json_body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(serde_json::to_vec(&json).expect("Serialize body"))
    } else {
        Body::empty()
    };

    let request = builder.body(body).expect("Build request");
    let response = app.clone().oneshot(request).await.expect("Execute request");
    let status = response.status();

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Read response bytes");

    let val: serde_json::Value = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };

    (status, val)
}
