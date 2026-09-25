//! # Telemetry Routes
//!
//! Route definitions for all telemetry ingestion and query endpoints.

use crate::{
    telemetry::{file_activity, location, network_activity, productivity, system_activity},
    AppState,
};
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

/// Builds and returns the router for all telemetry endpoints.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // System activity
        .route(
            "/system",
            post(system_activity::ingest_system_activity)
                .get(system_activity::query_system_activity),
        )
        .route(
            "/system/:id",
            get(system_activity::get_system_activity)
                .delete(system_activity::delete_system_activity),
        )
        // Network activity
        .route(
            "/network",
            post(network_activity::ingest_network_activity)
                .get(network_activity::query_network_activity),
        )
        .route(
            "/network/:id",
            get(network_activity::get_network_activity)
                .delete(network_activity::delete_network_activity),
        )
        // Productivity metrics
        .route(
            "/productivity",
            post(productivity::ingest_productivity).get(productivity::query_productivity),
        )
        .route(
            "/productivity/:id",
            get(productivity::get_productivity).delete(productivity::delete_productivity),
        )
        // File activity
        .route(
            "/files",
            post(file_activity::ingest_file_activity).get(file_activity::query_file_activity),
        )
        .route(
            "/files/:id",
            get(file_activity::get_file_activity).delete(file_activity::delete_file_activity),
        )
        // Location data
        .route(
            "/location",
            post(location::ingest_location).get(location::query_location),
        )
        .route(
            "/location/:id",
            get(location::get_location).delete(location::delete_location),
        )
}
