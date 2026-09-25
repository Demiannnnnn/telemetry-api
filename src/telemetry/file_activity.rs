//! # Telemetry — File Activity
//!
//! Handlers for corporate file access and modification event telemetry.

use crate::{
    errors::AppError,
    middleware::{verify_admin_owns_worker, RequireAdmin, RequireWorker},
    models::{CollectionResponse, PaginationMeta, PaginationParams, SingleResponse},
    telemetry::models::{
        BatchIngestionResponse, BatchTelemetrySubmission, TelemetryQueryParams,
        TelemetryRecordResponse,
    },
    AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use base64::prelude::*;
use chrono::{Duration, Utc};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

/// Ingests a batch of corporate file activity telemetry records.
pub async fn ingest_file_activity(
    RequireWorker(worker_auth): RequireWorker,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BatchTelemetrySubmission>,
) -> Result<(StatusCode, Json<SingleResponse<BatchIngestionResponse>>), AppError> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let worker_id = worker_auth.id;

    // Check worker active status and consent
    let consent_check = sqlx::query!(
        r#"
        SELECT is_active, consent_flags->>'file_activity' as "has_consent"
        FROM workers 
        WHERE id = $1 AND deleted_at IS NULL
        "#,
        worker_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::NotFound("Worker record not found".to_string()))?;

    if !consent_check.is_active {
        return Err(AppError::Forbidden(
            "Worker account is inactive".to_string(),
        ));
    }

    if consent_check.has_consent.as_deref() != Some("true") {
        return Err(AppError::Forbidden(
            "Worker has not given consent for file_activity telemetry".to_string(),
        ));
    }

    let now = Utc::now();
    let max_future = now + Duration::minutes(5);

    for record in &payload.records {
        if record.client_timestamp > max_future {
            return Err(AppError::Validation(
                "client_timestamp cannot be in the future (skew limit: 5m)".to_string(),
            ));
        }
    }

    let batch_id = Uuid::now_v7();
    let count = payload.records.len();
    let mut tx = state.db.begin().await.map_err(AppError::Database)?;

    for record in payload.records {
        let binary_payload = BASE64_STANDARD
            .decode(&record.encrypted_payload)
            .map_err(|_| {
                AppError::Validation("encrypted_payload must be valid base64".to_string())
            })?;
        let payload_size = binary_payload.len() as i32;

        sqlx::query!(
            r#"
            INSERT INTO telemetry_file_activities (
                worker_id, encrypted_payload, payload_size, client_timestamp, server_timestamp, batch_id
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            worker_id,
            binary_payload,
            payload_size,
            record.client_timestamp,
            now,
            batch_id
        )
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;
    }

    tx.commit().await.map_err(AppError::Database)?;

    tracing::info!(
        worker_id = %worker_id,
        category = "file_activity",
        records_count = count,
        batch_id = %batch_id,
        "File activity telemetry batch ingested"
    );

    Ok((
        StatusCode::CREATED,
        Json(SingleResponse::new(BatchIngestionResponse {
            batch_id,
            records_created: count,
            category: "file_activity",
            server_timestamp: now,
        })),
    ))
}

/// Queries encrypted file activity telemetry for a worker (administrator only).
pub async fn query_file_activity(
    RequireAdmin(admin): RequireAdmin,
    State(state): State<Arc<AppState>>,
    Query(params): Query<TelemetryQueryParams>,
) -> Result<Json<CollectionResponse<TelemetryRecordResponse>>, AppError> {
    verify_admin_owns_worker(admin.id, params.worker_id, &state.db).await?;

    let pagination = PaginationParams {
        page: params.page,
        per_page: params.per_page,
    };
    let limit = pagination.limit();
    let offset = pagination.offset();

    let total = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*)
        FROM telemetry_file_activities
        WHERE worker_id = $1
          AND ($2::TIMESTAMPTZ IS NULL OR client_timestamp >= $2)
          AND ($3::TIMESTAMPTZ IS NULL OR client_timestamp <= $3)
        "#,
        params.worker_id,
        params.from,
        params.to
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::Database)?
    .unwrap_or(0);

    let rows = sqlx::query!(
        r#"
        SELECT id, worker_id, encrypted_payload, payload_size, client_timestamp, server_timestamp, batch_id
        FROM telemetry_file_activities
        WHERE worker_id = $1
          AND ($2::TIMESTAMPTZ IS NULL OR client_timestamp >= $2)
          AND ($3::TIMESTAMPTZ IS NULL OR client_timestamp <= $3)
        ORDER BY client_timestamp DESC
        LIMIT $4 OFFSET $5
        "#,
        params.worker_id,
        params.from,
        params.to,
        limit,
        offset
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::Database)?;

    let records = rows
        .into_iter()
        .map(|r| TelemetryRecordResponse {
            id: r.id,
            worker_id: r.worker_id,
            encrypted_payload: BASE64_STANDARD.encode(&r.encrypted_payload),
            payload_size: r.payload_size,
            client_timestamp: r.client_timestamp,
            server_timestamp: r.server_timestamp,
            batch_id: r.batch_id,
            subcategory: None,
        })
        .collect();

    let meta = PaginationMeta::new(total, pagination.page(), pagination.per_page());
    Ok(Json(CollectionResponse::new(records, meta)))
}

/// Retrieves a specific file activity record by ID (administrator only).
pub async fn get_file_activity(
    RequireAdmin(admin): RequireAdmin,
    Path(record_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<SingleResponse<TelemetryRecordResponse>>, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT id, worker_id, encrypted_payload, payload_size, client_timestamp, server_timestamp, batch_id
        FROM telemetry_file_activities
        WHERE id = $1
        "#,
        record_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::NotFound("Telemetry record not found".to_string()))?;

    verify_admin_owns_worker(admin.id, row.worker_id, &state.db).await?;

    let record = TelemetryRecordResponse {
        id: row.id,
        worker_id: row.worker_id,
        encrypted_payload: BASE64_STANDARD.encode(&row.encrypted_payload),
        payload_size: row.payload_size,
        client_timestamp: row.client_timestamp,
        server_timestamp: row.server_timestamp,
        batch_id: row.batch_id,
        subcategory: None,
    };

    Ok(Json(SingleResponse::new(record)))
}

/// Deletes a specific file activity record (administrator only).
pub async fn delete_file_activity(
    RequireAdmin(admin): RequireAdmin,
    Path(record_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, AppError> {
    let row = sqlx::query!(
        "SELECT worker_id FROM telemetry_file_activities WHERE id = $1",
        record_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::NotFound("Telemetry record not found".to_string()))?;

    verify_admin_owns_worker(admin.id, row.worker_id, &state.db).await?;

    sqlx::query!(
        "DELETE FROM telemetry_file_activities WHERE id = $1",
        record_id
    )
    .execute(&state.db)
    .await
    .map_err(AppError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}
