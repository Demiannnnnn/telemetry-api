//! # Worker Routes
//!
//! Route definitions for worker CRUD operations and consent management.

use crate::{
    worker::handlers::{
        create_worker, delete_worker, get_worker, list_workers, update_consent, update_worker,
    },
    AppState,
};
use axum::{
    routing::{get, patch, post},
    Router,
};
use std::sync::Arc;

/// Builds and returns the router for worker endpoints.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_worker).get(list_workers))
        .route(
            "/:id",
            get(get_worker).put(update_worker).delete(delete_worker),
        )
        .route("/:id/consent", patch(update_consent))
}
