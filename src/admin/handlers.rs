//! # Admin Handlers
//!
//! Request handlers for administrator profile inspection and updates.

use crate::{
    admin::models::{AdminProfileResponse, UpdateAdminRequest},
    errors::AppError,
    middleware::RequireAdmin,
    models::SingleResponse,
    AppState,
};
use axum::{extract::State, Json};
use std::sync::Arc;
use validator::Validate;

/// Retrieves the authenticated administrator's profile and active worker count.
///
/// # Errors
///
/// Returns [`AppError::NotFound`] if the admin record does not exist or has been deleted.
pub async fn get_me(
    RequireAdmin(admin): RequireAdmin,
    State(state): State<Arc<AppState>>,
) -> Result<Json<SingleResponse<AdminProfileResponse>>, AppError> {
    let admin_id = admin.id;

    let record = sqlx::query!(
        r#"
        SELECT 
            a.id, a.email, a.name, a.organization, a.is_active, a.created_at, a.updated_at,
            (SELECT COUNT(*) FROM workers w WHERE w.admin_id = a.id AND w.deleted_at IS NULL AND w.is_active = TRUE) as "worker_count!"
        FROM admins a
        WHERE a.id = $1 AND a.deleted_at IS NULL
        "#,
        admin_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::NotFound("Admin profile not found".to_string()))?;

    let response = AdminProfileResponse {
        id: record.id,
        email: record.email,
        name: record.name,
        organization: record.organization,
        is_active: record.is_active,
        worker_count: record.worker_count,
        created_at: record.created_at,
        updated_at: record.updated_at,
    };

    Ok(Json(SingleResponse::new(response)))
}

/// Updates the authenticated administrator's display name and/or organization.
///
/// Note: Email address cannot be modified via this endpoint.
///
/// # Errors
///
/// Returns [`AppError::Validation`] if fields fail validation rules.
/// Returns [`AppError::NotFound`] if the admin record does not exist or has been deleted.
pub async fn update_me(
    RequireAdmin(admin): RequireAdmin,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateAdminRequest>,
) -> Result<Json<SingleResponse<AdminProfileResponse>>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let admin_id = admin.id;

    let record = sqlx::query!(
        r#"
        UPDATE admins
        SET 
            name = COALESCE($1, name),
            organization = COALESCE($2, organization),
            updated_at = NOW()
        WHERE id = $3 AND deleted_at IS NULL
        RETURNING id, email, name, organization, is_active, created_at, updated_at
        "#,
        payload.name,
        payload.organization,
        admin_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::NotFound("Admin profile not found".to_string()))?;

    let worker_count = sqlx::query_scalar!(
        r#"SELECT COUNT(*) FROM workers WHERE admin_id = $1 AND deleted_at IS NULL AND is_active = TRUE"#,
        admin_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::Database)?
    .unwrap_or(0);

    tracing::info!(admin_id = %admin_id, "Admin profile updated successfully");

    Ok(Json(SingleResponse::new(AdminProfileResponse {
        id: record.id,
        email: record.email,
        name: record.name,
        organization: record.organization,
        is_active: record.is_active,
        worker_count,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })))
}
