//! # Telemetry API Library
//!
//! Core library for the Telemetry API. Organizes the application
//! into domain modules with shared infrastructure.

// -- Shared Infrastructure Modules --
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
