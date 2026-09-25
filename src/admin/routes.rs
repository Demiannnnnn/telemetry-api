//! # Admin Routes
//!
//! Route definitions for administrator endpoints.

use crate::{
    admin::handlers::{get_me, update_me},
    AppState,
};
use axum::{routing::get, Router};
use std::sync::Arc;

/// Builds and returns the router for administrator endpoints.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/me", get(get_me).put(update_me))
}
