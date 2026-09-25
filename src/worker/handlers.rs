//! # Worker Handlers
//!
//! Request handlers for worker CRUD operations, listing, and consent matrix updates.

use crate::{
    errors::AppError,
    middleware::{verify_admin_owns_worker, AuthUser, RequireAdmin, RequireWorker},
    models::{CollectionResponse, PaginationMeta, PaginationParams, SingleResponse},
    worker::models::{
        ConsentFlags, CreateWorkerRequest, DataDeletionRequest, DataDeletionResponse,
        DataExportRequest, DataRequestResponse, UpdateConsentRequest, UpdateWorkerRequest,
        WorkerQueryParams, WorkerRecord, WorkerResponse, WorkerRoleType,
    },
    AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

/// Creates a new worker account assigned to the authenticated administrator.
///
/// Consent flags are initialized to all-false by default.
pub async fn create_worker(
    RequireAdmin(admin): RequireAdmin,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateWorkerRequest>,
) -> Result<(StatusCode, Json<SingleResponse<WorkerResponse>>), AppError> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    if let (Some(start), Some(end)) = (payload.work_hours_start, payload.work_hours_end) {
        if start >= end {
            return Err(AppError::Validation(
                "work_hours_start must precede work_hours_end".to_string(),
            ));
        }
    }

    let default_consent = serde_json::to_value(ConsentFlags::default())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Serialization error: {}", e)))?;
    let timezone = payload
        .timezone
        .unwrap_or_else(|| "America/Santiago".to_string());

    let record = sqlx::query!(
        r#"
        INSERT INTO workers (
            admin_id, email, name, device_identifier, role_type, 
            consent_flags, work_hours_start, work_hours_end, timezone
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, admin_id, email, name, device_identifier, 
                  role_type as "role_type: WorkerRoleType", is_active, 
                  consent_flags, work_hours_start, work_hours_end, 
                  timezone, created_at, updated_at
        "#,
        admin.id,
        payload.email,
        payload.name,
        payload.device_identifier,
        payload.role_type as WorkerRoleType,
        default_consent,
        payload.work_hours_start,
        payload.work_hours_end,
        timezone
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
            AppError::Conflict("A worker with this email already exists".to_string())
        }
        other => AppError::Database(other),
    })?;

    tracing::info!(
        worker_id = %record.id,
        admin_id = %admin.id,
        "Worker registered successfully"
    );

    let worker_record = WorkerRecord {
        id: record.id,
        admin_id: record.admin_id,
        email: record.email,
        name: record.name,
        device_identifier: record.device_identifier,
        role_type: record.role_type,
        is_active: record.is_active,
        consent_flags: record.consent_flags,
        work_hours_start: record.work_hours_start,
        work_hours_end: record.work_hours_end,
        timezone: record.timezone,
        created_at: record.created_at,
        updated_at: record.updated_at,
    };

    Ok((
        StatusCode::CREATED,
        Json(SingleResponse::new(worker_record.into_response())),
    ))
}

/// Lists all workers assigned to the authenticated administrator with filtering and pagination.
pub async fn list_workers(
    RequireAdmin(admin): RequireAdmin,
    State(state): State<Arc<AppState>>,
    Query(params): Query<WorkerQueryParams>,
) -> Result<Json<CollectionResponse<WorkerResponse>>, AppError> {
    let pagination = PaginationParams {
        page: params.page,
        per_page: params.per_page,
    };
    let limit = pagination.limit();
    let offset = pagination.offset();

    let search_pattern = params
        .search
        .as_ref()
        .map(|s| format!("%{}%", s.trim().to_lowercase()));

    let total = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*)
        FROM workers
        WHERE admin_id = $1
          AND deleted_at IS NULL
          AND ($2::BOOLEAN IS NULL OR is_active = $2)
          AND ($3::worker_role_type IS NULL OR role_type = $3)
          AND ($4::TEXT IS NULL OR LOWER(name) LIKE $4 OR LOWER(email) LIKE $4)
        "#,
        admin.id,
        params.is_active,
        params.role_type as Option<WorkerRoleType>,
        search_pattern.clone()
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::Database)?
    .unwrap_or(0);

    let records = sqlx::query!(
        r#"
        SELECT id, admin_id, email, name, device_identifier,
               role_type as "role_type: WorkerRoleType", is_active,
               consent_flags, work_hours_start, work_hours_end,
               timezone, created_at, updated_at
        FROM workers
        WHERE admin_id = $1
          AND deleted_at IS NULL
          AND ($2::BOOLEAN IS NULL OR is_active = $2)
          AND ($3::worker_role_type IS NULL OR role_type = $3)
          AND ($4::TEXT IS NULL OR LOWER(name) LIKE $4 OR LOWER(email) LIKE $4)
        ORDER BY created_at DESC
        LIMIT $5 OFFSET $6
        "#,
        admin.id,
        params.is_active,
        params.role_type as Option<WorkerRoleType>,
        search_pattern,
        limit,
        offset
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::Database)?;

    let data = records
        .into_iter()
        .map(|r| {
            WorkerRecord {
                id: r.id,
                admin_id: r.admin_id,
                email: r.email,
                name: r.name,
                device_identifier: r.device_identifier,
                role_type: r.role_type,
                is_active: r.is_active,
                consent_flags: r.consent_flags,
                work_hours_start: r.work_hours_start,
                work_hours_end: r.work_hours_end,
                timezone: r.timezone,
                created_at: r.created_at,
                updated_at: r.updated_at,
            }
            .into_response()
        })
        .collect();

    let meta = PaginationMeta::new(total, pagination.page(), pagination.per_page());
    Ok(Json(CollectionResponse::new(data, meta)))
}

/// Retrieves a specific worker profile by ID.
///
/// Permitted for the managing administrator or the worker themselves.
pub async fn get_worker(
    auth_user: AuthUser,
    Path(worker_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<SingleResponse<WorkerResponse>>, AppError> {
    let record = sqlx::query!(
        r#"
        SELECT id, admin_id, email, name, device_identifier,
               role_type as "role_type: WorkerRoleType", is_active,
               consent_flags, work_hours_start, work_hours_end,
               timezone, created_at, updated_at
        FROM workers
        WHERE id = $1 AND deleted_at IS NULL
        "#,
        worker_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::NotFound(format!("Worker with ID '{worker_id}' not found")))?;

    // Authorization check
    match auth_user.role {
        crate::crypto::UserRole::Admin => {
            if record.admin_id != auth_user.id {
                return Err(AppError::Forbidden(
                    "You do not manage this worker".to_string(),
                ));
            }
        }
        crate::crypto::UserRole::Worker => {
            if record.id != auth_user.id {
                return Err(AppError::Forbidden(
                    "You cannot access other workers' profiles".to_string(),
                ));
            }
        }
    }

    let worker_record = WorkerRecord {
        id: record.id,
        admin_id: record.admin_id,
        email: record.email,
        name: record.name,
        device_identifier: record.device_identifier,
        role_type: record.role_type,
        is_active: record.is_active,
        consent_flags: record.consent_flags,
        work_hours_start: record.work_hours_start,
        work_hours_end: record.work_hours_end,
        timezone: record.timezone,
        created_at: record.created_at,
        updated_at: record.updated_at,
    };

    Ok(Json(SingleResponse::new(worker_record.into_response())))
}

/// Updates worker operational configuration (admin only).
pub async fn update_worker(
    RequireAdmin(admin): RequireAdmin,
    Path(worker_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateWorkerRequest>,
) -> Result<Json<SingleResponse<WorkerResponse>>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    verify_admin_owns_worker(admin.id, worker_id, &state.db).await?;

    if let (Some(start), Some(end)) = (payload.work_hours_start, payload.work_hours_end) {
        if start >= end {
            return Err(AppError::Validation(
                "work_hours_start must precede work_hours_end".to_string(),
            ));
        }
    }

    let record = sqlx::query!(
        r#"
        UPDATE workers
        SET
            name = COALESCE($1, name),
            device_identifier = COALESCE($2, device_identifier),
            role_type = COALESCE($3, role_type),
            is_active = COALESCE($4, is_active),
            work_hours_start = COALESCE($5, work_hours_start),
            work_hours_end = COALESCE($6, work_hours_end),
            timezone = COALESCE($7, timezone),
            updated_at = NOW()
        WHERE id = $8 AND deleted_at IS NULL
        RETURNING id, admin_id, email, name, device_identifier,
                  role_type as "role_type: WorkerRoleType", is_active,
                  consent_flags, work_hours_start, work_hours_end,
                  timezone, created_at, updated_at
        "#,
        payload.name,
        payload.device_identifier,
        payload.role_type as Option<WorkerRoleType>,
        payload.is_active,
        payload.work_hours_start,
        payload.work_hours_end,
        payload.timezone,
        worker_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::NotFound(format!("Worker with ID '{worker_id}' not found")))?;

    tracing::info!(worker_id = %worker_id, "Worker profile updated by administrator");

    let worker_record = WorkerRecord {
        id: record.id,
        admin_id: record.admin_id,
        email: record.email,
        name: record.name,
        device_identifier: record.device_identifier,
        role_type: record.role_type,
        is_active: record.is_active,
        consent_flags: record.consent_flags,
        work_hours_start: record.work_hours_start,
        work_hours_end: record.work_hours_end,
        timezone: record.timezone,
        created_at: record.created_at,
        updated_at: record.updated_at,
    };

    Ok(Json(SingleResponse::new(worker_record.into_response())))
}

/// Soft-deletes a worker (admin only).
pub async fn delete_worker(
    RequireAdmin(admin): RequireAdmin,
    Path(worker_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, AppError> {
    verify_admin_owns_worker(admin.id, worker_id, &state.db).await?;

    sqlx::query!(
        "UPDATE workers SET deleted_at = NOW(), is_active = FALSE WHERE id = $1",
        worker_id
    )
    .execute(&state.db)
    .await
    .map_err(AppError::Database)?;

    tracing::info!(worker_id = %worker_id, "Worker soft-deleted");

    Ok(StatusCode::NO_CONTENT)
}

/// Updates privacy consent flags (worker only).
///
/// Prevents office workers from consenting to location tracking.
pub async fn update_consent(
    RequireWorker(worker_auth): RequireWorker,
    Path(worker_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateConsentRequest>,
) -> Result<Json<SingleResponse<ConsentFlags>>, AppError> {
    if worker_auth.id != worker_id {
        return Err(AppError::Forbidden(
            "You can only modify your own consent flags".to_string(),
        ));
    }

    let worker = sqlx::query!(
        r#"SELECT role_type as "role_type: WorkerRoleType" FROM workers WHERE id = $1 AND deleted_at IS NULL"#,
        worker_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::NotFound("Worker not found".to_string()))?;

    // Hard privacy constraint: Office workers cannot consent to location tracking
    if worker.role_type == WorkerRoleType::Office && payload.consent_flags.location {
        return Err(AppError::Validation(
            "Workers with OFFICE role cannot consent to location tracking".to_string(),
        ));
    }

    let consent_json = serde_json::to_value(&payload.consent_flags)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Serialization error: {}", e)))?;

    sqlx::query!(
        "UPDATE workers SET consent_flags = $1, updated_at = NOW() WHERE id = $2",
        consent_json,
        worker_id
    )
    .execute(&state.db)
    .await
    .map_err(AppError::Database)?;

    let _ = crate::audit::record_audit_event(
        &state.db,
        worker_auth.id,
        worker_auth.role,
        "CONSENT_UPDATED",
        "worker",
        Some(worker_id),
        None,
        Some(consent_json),
    )
    .await;

    tracing::info!(worker_id = %worker_id, "Worker consent flags updated");

    Ok(Json(SingleResponse::new(payload.consent_flags)))
}

/// Requests a complete export of the worker's historical telemetry data (GDPR/ARCO).
pub async fn request_data_export(
    RequireWorker(worker): RequireWorker,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<DataExportRequest>,
) -> Result<(StatusCode, Json<SingleResponse<DataRequestResponse>>), AppError> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    if payload.r#type != "EXPORT" {
        return Err(AppError::Validation(
            "Request type must be 'EXPORT'".to_string(),
        ));
    }

    if let (Some(from), Some(to)) = (payload.from, payload.to) {
        if from > to {
            return Err(AppError::Validation(
                "'from' timestamp must precede 'to' timestamp".to_string(),
            ));
        }
    }

    // Rate limit: Max 5 DSR requests per 24 hours per worker
    let dsr_count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*)
        FROM audit_logs
        WHERE actor_id = $1
          AND action IN ('DATA_EXPORT_REQUESTED', 'DATA_DELETION_REQUESTED')
          AND created_at >= NOW() - INTERVAL '24 hours'
        "#,
        worker.id
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::Database)?
    .unwrap_or(0);

    if dsr_count >= 5 {
        return Err(AppError::RateLimited);
    }

    let request_id = Uuid::now_v7();
    let estimated_completion = chrono::Utc::now() + chrono::Duration::hours(1);

    let meta = serde_json::json!({
        "categories": payload.categories,
        "from": payload.from,
        "to": payload.to,
        "request_id": request_id,
    });

    crate::audit::record_audit_event(
        &state.db,
        worker.id,
        worker.role,
        "DATA_EXPORT_REQUESTED",
        "telemetry",
        Some(worker.id),
        Some(&headers),
        Some(meta),
    )
    .await?;

    tracing::info!(
        worker_id = %worker.id,
        request_id = %request_id,
        action = "DSR_REQUEST",
        "Data export request registered"
    );

    Ok((
        StatusCode::ACCEPTED,
        Json(SingleResponse::new(DataRequestResponse {
            request_id,
            status: "PENDING",
            estimated_completion,
        })),
    ))
}

/// Requests the purge/erasure of historical telemetry data (GDPR/ARCO Right to be Forgotten).
pub async fn request_data_deletion(
    RequireWorker(worker): RequireWorker,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<DataDeletionRequest>,
) -> Result<(StatusCode, Json<SingleResponse<DataDeletionResponse>>), AppError> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Rate limit: Max 5 DSR requests per 24 hours per worker
    let dsr_count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*)
        FROM audit_logs
        WHERE actor_id = $1
          AND action IN ('DATA_EXPORT_REQUESTED', 'DATA_DELETION_REQUESTED')
          AND created_at >= NOW() - INTERVAL '24 hours'
        "#,
        worker.id
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::Database)?
    .unwrap_or(0);

    if dsr_count >= 5 {
        return Err(AppError::RateLimited);
    }

    let request_id = Uuid::now_v7();

    let meta = serde_json::json!({
        "categories": payload.categories,
        "reason": payload.reason,
        "request_id": request_id,
    });

    crate::audit::record_audit_event(
        &state.db,
        worker.id,
        worker.role,
        "DATA_DELETION_REQUESTED",
        "telemetry",
        Some(worker.id),
        Some(&headers),
        Some(meta),
    )
    .await?;

    tracing::info!(
        worker_id = %worker.id,
        request_id = %request_id,
        action = "DSR_REQUEST",
        "Data deletion request registered"
    );

    Ok((
        StatusCode::ACCEPTED,
        Json(SingleResponse::new(DataDeletionResponse {
            request_id,
            status: "PENDING",
            message:
                "Data deletion request accepted and will be purged according to retention policy"
                    .to_string(),
        })),
    ))
}
