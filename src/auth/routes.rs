//! # Auth Routes
//!
//! Route definitions for authentication and token lifecycle endpoints.

use crate::{
    auth::handlers::{login_handler, logout_handler, refresh_handler, register_handler},
    AppState,
};
use axum::{routing::post, Router};
use std::sync::Arc;

/// Builds and returns the Axum router for the `/auth` domain.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register_handler))
        .route("/login", post(login_handler))
        .route("/refresh", post(refresh_handler))
        .route("/logout", post(logout_handler))
}
