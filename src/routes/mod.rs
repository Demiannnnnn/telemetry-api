//! # Routes Module
//!
//! Central routing assembly for the Telemetry API.
//! Mounts `/health`, `/api/v1/` subrouters (`auth`, `admins`, `workers`, `telemetry`),
//! and supplies the fallback 404 JSON handler.

use crate::{admin, auth, database, errors::AppError, telemetry, worker, AppState};
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;

/// Constructs the primary application router with all nested API routes,
/// health check endpoint, fallback handler, and state injection.
pub fn create_router(state: Arc<AppState>) -> Router {
    let api_v1 = Router::new()
        .nest("/auth", auth::routes::router())
        .nest("/admins", admin::routes::router())
        .nest("/workers", worker::routes::router())
        .nest("/telemetry", telemetry::routes::router());

    Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1", api_v1)
        .fallback(fallback_handler)
        .with_state(state)
}

/// Active health check endpoint checking PostgreSQL connectivity.
///
/// Returns 200 OK with server version and UTC timestamp.
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    database::check_health(&state.db).await?;

    Ok(Json(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": Utc::now()
    })))
}

/// Fallback handler for unmatched routes, ensuring canonical JSON error responses.
pub async fn fallback_handler() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": {
                "code": "RESOURCE_NOT_FOUND",
                "message": "The requested API endpoint does not exist",
                "details": null
            }
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::database::init_pool;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn setup_app_state() -> Option<Arc<AppState>> {
        let config = Config::from_env().ok()?;
        let pool = init_pool(&config).await.ok()?;
        Some(Arc::new(AppState::new(pool, config)))
    }

    #[tokio::test]
    async fn test_health_check_endpoint() {
        let Some(state) = setup_app_state().await else {
            eprintln!("Skipping DB test: PostgreSQL not reachable");
            return;
        };

        let app = create_router(state);
        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["status"], "healthy");
        assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
        assert!(body["timestamp"].is_string());
    }

    #[tokio::test]
    async fn test_fallback_handler_returns_canonical_404() {
        let Some(state) = setup_app_state().await else {
            eprintln!("Skipping DB test: PostgreSQL not reachable");
            return;
        };

        let app = create_router(state);
        let request = Request::builder()
            .uri("/api/v1/non_existent_route")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["error"]["code"], "RESOURCE_NOT_FOUND");
        assert_eq!(
            body["error"]["message"],
            "The requested API endpoint does not exist"
        );
    }
}
