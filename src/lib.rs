//! # Telemetry API Library
//!
//! Core library for the Telemetry API. Organizes the application
//! into domain modules with shared infrastructure.

use std::sync::Arc;

// -- Shared Infrastructure Modules --
pub mod audit;
pub mod config;
pub mod crypto;
pub mod database;
pub mod errors;
pub mod middleware;
pub mod models;
pub mod routes;

// -- Domain Modules --
pub mod admin;
pub mod auth;
pub mod telemetry;
pub mod worker;

/// Shared application state passed to Axum routers and handlers.
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL connection pool.
    pub db: database::DbPool,
    /// Typed configuration.
    pub config: Arc<config::Config>,
}

impl AppState {
    /// Constructs a new [`AppState`] with the given database pool and configuration.
    pub fn new(db: database::DbPool, config: config::Config) -> Self {
        Self {
            db,
            config: Arc::new(config),
        }
    }
}
