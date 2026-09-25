//! # Database Module
//!
//! Provides PostgreSQL connection pooling and migration management via SQLx.

use crate::{config::Config, errors::AppError};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

/// Database connection pool type alias.
pub type DbPool = PgPool;

/// Initializes the PostgreSQL connection pool and runs pending database migrations.
///
/// # Arguments
///
/// * `config` - Application configuration containing the database URL and connection limits
///
/// # Returns
///
/// Returns an initialized and migrated [`DbPool`].
///
/// # Errors
///
/// Returns [`AppError::Database`] if connecting to PostgreSQL fails.
/// Returns [`AppError::Internal`] if migration execution fails.
pub async fn init_pool(config: &Config) -> Result<DbPool, AppError> {
    tracing::info!(
        max_connections = config.database_max_connections,
        min_connections = config.database_min_connections,
        database_url = %config.masked_database_url(),
        "Connecting to PostgreSQL database..."
    );

    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .min_connections(config.database_min_connections)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .connect(&config.database_url)
        .await
        .map_err(AppError::Database)?;

    tracing::info!("Running pending database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Migration execution failed");
            AppError::Internal(anyhow::anyhow!("Migration failed: {}", e))
        })?;

    tracing::info!("Database migrations applied successfully.");
    Ok(pool)
}

/// Executes a health check query (`SELECT 1`) to verify the database connection is alive.
///
/// # Arguments
///
/// * `pool` - Database connection pool reference
///
/// # Errors
///
/// Returns [`AppError::Database`] if the health check query fails.
pub async fn check_health(pool: &DbPool) -> Result<(), AppError> {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_pool() -> Option<DbPool> {
        let config = Config::from_env().ok()?;
        init_pool(&config).await.ok()
    }

    #[tokio::test]
    async fn test_db_health_check_succeeds() {
        let Some(pool) = setup_test_pool().await else {
            eprintln!("Skipping database test: PostgreSQL not reachable");
            return;
        };

        let result = check_health(&pool).await;
        assert!(result.is_ok(), "Health check should succeed");
    }

    #[tokio::test]
    async fn test_db_migration_execution_succeeds() {
        let Some(pool) = setup_test_pool().await else {
            eprintln!("Skipping database test: PostgreSQL not reachable");
            return;
        };

        let required_tables = vec![
            "admins",
            "workers",
            "telemetry_system_activities",
            "telemetry_network_activities",
            "telemetry_productivity_metrics",
            "telemetry_file_activities",
            "telemetry_location_data",
            "refresh_tokens",
            "audit_logs",
        ];

        for table in required_tables {
            let row: (bool,) = sqlx::query_as(
                "SELECT EXISTS (
                    SELECT 1 FROM information_schema.tables 
                    WHERE table_schema = 'public' AND table_name = $1
                )",
            )
            .bind(table)
            .fetch_one(&pool)
            .await
            .unwrap_or_else(|_| panic!("Failed to query existence of table {table}"));

            assert!(row.0, "Table {table} must exist after migrations");
        }
    }

    #[tokio::test]
    async fn test_db_worker_invalid_work_hours_fails() {
        let Some(pool) = setup_test_pool().await else {
            eprintln!("Skipping database test: PostgreSQL not reachable");
            return;
        };

        // Create admin first
        let admin_id: (uuid::Uuid,) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization)
             VALUES ($1, 'hash', 'Test Admin', 'Org')
             RETURNING id",
        )
        .bind(format!("admin_{}@test.com", uuid::Uuid::new_v4()))
        .fetch_one(&pool)
        .await
        .expect("Admin creation should succeed");

        // Try invalid work hours (18:00 to 09:00)
        let result = sqlx::query(
            "INSERT INTO workers (
                admin_id, email, name, device_identifier, role_type,
                work_hours_start, work_hours_end
            ) VALUES ($1, $2, 'Worker', 'dev1', 'OFFICE', '18:00:00', '09:00:00')",
        )
        .bind(admin_id.0)
        .bind(format!("worker_{}@test.com", uuid::Uuid::new_v4()))
        .execute(&pool)
        .await;

        assert!(
            result.is_err(),
            "Inserting worker with work_hours_start > work_hours_end must fail constraint"
        );
    }

    #[tokio::test]
    async fn test_db_worker_office_location_consent_fails() {
        let Some(pool) = setup_test_pool().await else {
            eprintln!("Skipping database test: PostgreSQL not reachable");
            return;
        };

        let admin_id: (uuid::Uuid,) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization)
             VALUES ($1, 'hash', 'Test Admin', 'Org')
             RETURNING id",
        )
        .bind(format!("admin_{}@test.com", uuid::Uuid::new_v4()))
        .fetch_one(&pool)
        .await
        .expect("Admin creation should succeed");

        // Try location consent with OFFICE role
        let result = sqlx::query(
            "INSERT INTO workers (
                admin_id, email, name, device_identifier, role_type, consent_flags
            ) VALUES ($1, $2, 'Worker', 'dev2', 'OFFICE', '{\"location\": true}')",
        )
        .bind(admin_id.0)
        .bind(format!("worker_{}@test.com", uuid::Uuid::new_v4()))
        .execute(&pool)
        .await;

        assert!(
            result.is_err(),
            "Worker with OFFICE role and location=true must violate chk_workers_location_consent"
        );
    }

    #[tokio::test]
    async fn test_db_updated_at_trigger_updates_timestamp() {
        let Some(pool) = setup_test_pool().await else {
            eprintln!("Skipping database test: PostgreSQL not reachable");
            return;
        };

        let (admin_id, created_at, updated_at): (
            uuid::Uuid,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        ) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization)
             VALUES ($1, 'hash', 'Initial Name', 'Org')
             RETURNING id, created_at, updated_at",
        )
        .bind(format!("admin_{}@test.com", uuid::Uuid::new_v4()))
        .fetch_one(&pool)
        .await
        .expect("Admin insert");

        assert_eq!(created_at, updated_at);

        tokio::time::sleep(Duration::from_millis(50)).await;

        let updated_row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
            "UPDATE admins SET name = 'Updated Name' WHERE id = $1 RETURNING updated_at",
        )
        .bind(admin_id)
        .fetch_one(&pool)
        .await
        .expect("Admin update");

        assert!(
            updated_row.0 > created_at,
            "updated_at timestamp ({:?}) must be greater than created_at ({:?})",
            updated_row.0,
            created_at
        );
    }
}
