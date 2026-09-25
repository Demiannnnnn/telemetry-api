//! # Configuration Module
//!
//! Loads and validates environment variables into a typed, immutable [`Config`] struct.
//! Guarantees fail-fast startup if critical environment variables are missing or invalid.

use serde::Deserialize;
use std::fmt;
use thiserror::Error;

/// Application environment.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// Development environment with verbose logging and local defaults.
    #[default]
    Development,
    /// Automated testing environment.
    Test,
    /// Production environment with strict security requirements.
    Production,
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Test => write!(f, "test"),
            Self::Production => write!(f, "production"),
        }
    }
}

/// Errors that can occur during configuration loading.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    /// A mandatory environment variable was not found.
    #[error("Missing required environment variable: {0}")]
    MissingVariable(&'static str),

    /// The port number was not a valid 16-bit unsigned integer.
    #[error("Invalid server port specified")]
    InvalidPort,

    /// The JWT secret does not meet the minimum length or complexity requirement.
    #[error("Weak or invalid secret: {0}")]
    WeakSecret(&'static str),

    /// An environment variable contained an invalid or unparseable value.
    #[error("Invalid configuration value: {0}")]
    InvalidValue(String),
}

/// Typed, immutable configuration for the Telemetry API.
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    /// Host interface to bind to (e.g., "0.0.0.0" or "127.0.0.1").
    pub server_host: String,
    /// Port number to listen on (e.g., 8080).
    pub server_port: u16,
    /// PostgreSQL connection URL.
    pub database_url: String,
    /// Maximum connections in the database pool.
    pub database_max_connections: u32,
    /// Minimum idle connections in the database pool.
    pub database_min_connections: u32,
    /// Secret key used for signing and verifying JWT tokens (min 32 chars).
    pub jwt_secret: String,
    /// Expiration time for access tokens in hours.
    pub jwt_access_expiration_hours: i64,
    /// Expiration time for refresh tokens in days.
    pub jwt_refresh_expiration_days: i64,
    /// List of allowed CORS origins.
    pub cors_allowed_origins: Vec<String>,
    /// Maximum allowed requests per second per user.
    pub rate_limit_requests_per_second: u64,
    /// Maximum burst size for rate limiting.
    pub rate_limit_burst_size: u32,
    /// Running environment mode.
    pub environment: Environment,
    /// Maximum allowed request payload in bytes.
    pub max_payload_bytes: usize,
}

impl Config {
    /// Loads configuration from the environment and `.env` file if present.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::MissingVariable`] if required variables are absent.
    /// Returns [`ConfigError::WeakSecret`] if `JWT_SECRET` is less than 32 characters.
    /// Returns [`ConfigError::InvalidPort`] if `SERVER_PORT` is not a valid number.
    pub fn from_env() -> Result<Self, ConfigError> {
        // Attempt to load .env file; ignore if missing
        dotenvy::dotenv().ok();
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Loads configuration using a custom lookup function.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::MissingVariable`] if required variables are absent.
    /// Returns [`ConfigError::WeakSecret`] if `JWT_SECRET` is less than 32 characters.
    /// Returns [`ConfigError::InvalidPort`] if `SERVER_PORT` is not a valid number.
    pub fn from_lookup<F>(mut lookup: F) -> Result<Self, ConfigError>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let server_host = lookup("SERVER_HOST").unwrap_or_else(|| "0.0.0.0".to_string());

        let server_port = match lookup("SERVER_PORT") {
            Some(val) => val.parse::<u16>().map_err(|_| ConfigError::InvalidPort)?,
            None => 8080,
        };

        let database_url =
            lookup("DATABASE_URL").ok_or(ConfigError::MissingVariable("DATABASE_URL"))?;

        let database_max_connections = lookup("DATABASE_MAX_CONNECTIONS")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(10);

        let database_min_connections = lookup("DATABASE_MIN_CONNECTIONS")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(2);

        let jwt_secret = lookup("JWT_SECRET").ok_or(ConfigError::MissingVariable("JWT_SECRET"))?;

        if jwt_secret.len() < 32 {
            return Err(ConfigError::WeakSecret(
                "JWT_SECRET must be at least 32 characters long",
            ));
        }

        // Support both JWT_ACCESS_EXPIRATION_HOURS and JWT_EXPIRATION_HOURS
        let jwt_access_expiration_hours = lookup("JWT_ACCESS_EXPIRATION_HOURS")
            .or_else(|| lookup("JWT_EXPIRATION_HOURS"))
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(24);

        let jwt_refresh_expiration_days = lookup("JWT_REFRESH_EXPIRATION_DAYS")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(7);

        let cors_allowed_origins = lookup("CORS_ALLOWED_ORIGINS")
            .map(|s| {
                s.split(',')
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect()
            })
            .unwrap_or_else(|| {
                vec![
                    "http://localhost:3000".to_string(),
                    "http://localhost:3001".to_string(),
                ]
            });

        let rate_limit_requests_per_second = lookup("RATE_LIMIT_REQUESTS_PER_SECOND")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(100);

        let rate_limit_burst_size = lookup("RATE_LIMIT_BURST_SIZE")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(200);

        let environment = match lookup("ENVIRONMENT")
            .unwrap_or_else(|| "development".to_string())
            .to_lowercase()
            .as_str()
        {
            "production" | "prod" => Environment::Production,
            "test" => Environment::Test,
            _ => Environment::Development,
        };

        let max_payload_bytes = lookup("MAX_PAYLOAD_BYTES")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(10 * 1024 * 1024); // 10 MB default

        Ok(Self {
            server_host,
            server_port,
            database_url,
            database_max_connections,
            database_min_connections,
            jwt_secret,
            jwt_access_expiration_hours,
            jwt_refresh_expiration_days,
            cors_allowed_origins,
            rate_limit_requests_per_second,
            rate_limit_burst_size,
            environment,
            max_payload_bytes,
        })
    }

    /// Returns a masked database URL safe for logging (passwords redacted).
    pub fn masked_database_url(&self) -> String {
        // Redact password if present in standard URI format: postgres://user:pass@host/db
        if let Some(at_idx) = self.database_url.find('@') {
            if let Some(proto_idx) = self.database_url.find("://") {
                let user_pass = &self.database_url[proto_idx + 3..at_idx];
                if let Some(colon_idx) = user_pass.find(':') {
                    let user = &user_pass[..colon_idx];
                    let host_and_beyond = &self.database_url[at_idx..];
                    let proto = &self.database_url[..proto_idx + 3];
                    return format!("{proto}{user}:***{host_and_beyond}");
                }
            }
        }
        "postgres://***".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_config_missing_jwt_secret_fails() {
        let mut map = HashMap::new();
        map.insert(
            "DATABASE_URL".to_string(),
            "postgres://localhost/test".to_string(),
        );

        let result = Config::from_lookup(|k| map.get(k).cloned());
        assert_eq!(
            result.unwrap_err(),
            ConfigError::MissingVariable("JWT_SECRET")
        );
    }

    #[test]
    fn test_config_weak_jwt_secret_fails() {
        let mut map = HashMap::new();
        map.insert(
            "DATABASE_URL".to_string(),
            "postgres://localhost/test".to_string(),
        );
        map.insert("JWT_SECRET".to_string(), "too-short-secret".to_string());

        let result = Config::from_lookup(|k| map.get(k).cloned());
        assert!(matches!(result, Err(ConfigError::WeakSecret(_))));
    }

    #[test]
    fn test_config_invalid_port_fails() {
        let mut map = HashMap::new();
        map.insert(
            "DATABASE_URL".to_string(),
            "postgres://localhost/test".to_string(),
        );
        map.insert(
            "JWT_SECRET".to_string(),
            "this-is-a-valid-32-character-secret-key!!".to_string(),
        );
        map.insert("SERVER_PORT".to_string(), "not-a-port".to_string());

        let result = Config::from_lookup(|k| map.get(k).cloned());
        assert_eq!(result.unwrap_err(), ConfigError::InvalidPort);
    }

    #[test]
    fn test_config_valid_loading() {
        let mut map = HashMap::new();
        map.insert(
            "DATABASE_URL".to_string(),
            "postgres://usr:secret@localhost:5432/db".to_string(),
        );
        map.insert(
            "JWT_SECRET".to_string(),
            "this-is-a-valid-32-character-secret-key!!".to_string(),
        );
        map.insert("SERVER_PORT".to_string(), "9090".to_string());

        let config =
            Config::from_lookup(|k| map.get(k).cloned()).expect("valid configuration should load");
        assert_eq!(config.server_port, 9090);
        assert_eq!(config.server_host, "0.0.0.0");
        assert_eq!(config.environment, Environment::Development);
        assert_eq!(
            config.masked_database_url(),
            "postgres://usr:***@localhost:5432/db"
        );
    }
}
