//! # Auth Handlers
//!
//! Handlers for administrator registration, user login, token refresh rotation, and logout.

use crate::{
    auth::models::{
        AuthResponse, LoginRequest, LogoutRequest, RefreshTokenRequest, RegisterRequest, UserInfo,
    },
    crypto::{self, UserRole},
    errors::AppError,
    models::SingleResponse,
    AppState,
};
use axum::{extract::State, http::StatusCode, Json};
use chrono::{Duration, Utc};
use std::sync::Arc;
use uuid::Uuid;

/// Helper function to issue an access and refresh token pair for a user and record the refresh token.
async fn issue_tokens(
    state: &AppState,
    user_id: Uuid,
    role: UserRole,
    email: &str,
    name: &str,
) -> Result<AuthResponse, AppError> {
    let access_token = crypto::generate_access_token(
        user_id,
        role,
        email,
        &state.config.jwt_secret,
        state.config.jwt_access_expiration_hours,
    )?;

    let refresh_token = crypto::generate_secure_random_token();
    let token_hash = crypto::hash_token(&refresh_token);
    let expires_at = Utc::now() + Duration::days(state.config.jwt_refresh_expiration_days);

    sqlx::query!(
        r#"
        INSERT INTO refresh_tokens (user_id, user_role, token_hash, expires_at)
        VALUES ($1, $2, $3, $4)
        "#,
        user_id,
        role as UserRole,
        token_hash,
        expires_at
    )
    .execute(&state.db)
    .await
    .map_err(AppError::Database)?;

    Ok(AuthResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.jwt_access_expiration_hours * 3600,
        user: UserInfo {
            id: user_id,
            email: email.to_string(),
            name: name.to_string(),
            role,
        },
    })
}

/// Registers a new administrator.
///
/// # Errors
///
/// Returns [`AppError::Validation`] if email or password fails validation.
/// Returns [`AppError::Conflict`] if email is already taken.
pub async fn register_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<SingleResponse<AuthResponse>>), AppError> {
    if payload.email.trim().is_empty() || !payload.email.contains('@') {
        return Err(AppError::Validation("Invalid email address".to_string()));
    }
    if payload.name.trim().is_empty() {
        return Err(AppError::Validation("Name cannot be empty".to_string()));
    }
    if payload.organization.trim().is_empty() {
        return Err(AppError::Validation(
            "Organization cannot be empty".to_string(),
        ));
    }

    crypto::validate_password_complexity(&payload.password)?;

    let existing = sqlx::query!(
        "SELECT id FROM admins WHERE email = $1 AND deleted_at IS NULL",
        payload.email
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?;

    if existing.is_some() {
        return Err(AppError::Conflict(
            "An administrator with this email already exists".to_string(),
        ));
    }

    let password_hash = crypto::hash_password(&payload.password)?;

    let admin = sqlx::query!(
        r#"
        INSERT INTO admins (email, password_hash, name, organization)
        VALUES ($1, $2, $3, $4)
        RETURNING id, email, name
        "#,
        payload.email,
        password_hash,
        payload.name,
        payload.organization
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::Database)?;

    tracing::info!(admin_id = %admin.id, "Admin successfully registered");

    let auth_response =
        issue_tokens(&state, admin.id, UserRole::Admin, &admin.email, &admin.name).await?;

    Ok((
        StatusCode::CREATED,
        Json(SingleResponse::new(auth_response)),
    ))
}

/// Authenticates an administrator or worker.
///
/// # Errors
///
/// Returns [`AppError::Unauthorized`] if credentials are invalid or account is disabled.
pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<SingleResponse<AuthResponse>>, AppError> {
    match payload.role {
        UserRole::Admin => {
            let password = payload.password.ok_or_else(|| {
                AppError::Validation("Password is required for administrator login".to_string())
            })?;

            let admin = sqlx::query!(
                r#"
                SELECT id, email, password_hash, name, is_active
                FROM admins
                WHERE email = $1 AND deleted_at IS NULL
                "#,
                payload.email
            )
            .fetch_optional(&state.db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::Unauthorized("Invalid email or password".to_string()))?;

            if !admin.is_active {
                return Err(AppError::Unauthorized(
                    "Account is inactive or disabled".to_string(),
                ));
            }

            let password_valid = crypto::verify_password(&password, &admin.password_hash)?;
            if !password_valid {
                return Err(AppError::Unauthorized(
                    "Invalid email or password".to_string(),
                ));
            }

            tracing::info!(user_id = %admin.id, role = "ADMIN", "Administrator logged in");

            let auth_response =
                issue_tokens(&state, admin.id, UserRole::Admin, &admin.email, &admin.name).await?;

            Ok(Json(SingleResponse::new(auth_response)))
        }
        UserRole::Worker => {
            let worker = sqlx::query!(
                r#"
                SELECT id, email, name, device_identifier, is_active
                FROM workers
                WHERE email = $1 AND deleted_at IS NULL
                "#,
                payload.email
            )
            .fetch_optional(&state.db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::Unauthorized("Invalid worker credentials".to_string()))?;

            if !worker.is_active {
                return Err(AppError::Unauthorized(
                    "Worker account is inactive or disabled".to_string(),
                ));
            }

            if let Some(device_id) = payload.device_identifier {
                if worker.device_identifier != device_id {
                    return Err(AppError::Unauthorized(
                        "Device identifier mismatch".to_string(),
                    ));
                }
            }

            tracing::info!(user_id = %worker.id, role = "WORKER", "Worker logged in");

            let auth_response = issue_tokens(
                &state,
                worker.id,
                UserRole::Worker,
                &worker.email,
                &worker.name,
            )
            .await?;

            Ok(Json(SingleResponse::new(auth_response)))
        }
    }
}

/// Rotates and refreshes an expired access token using a single-use refresh token.
///
/// Implements Token Family Security: if reuse of an already-revoked token is detected,
/// all tokens in the user's family are immediately revoked as a security measure.
///
/// # Errors
///
/// Returns [`AppError::Unauthorized`] if the token is invalid, expired, or was already revoked.
pub async fn refresh_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<SingleResponse<AuthResponse>>, AppError> {
    let token_hash = crypto::hash_token(&payload.refresh_token);

    let token_record = sqlx::query!(
        r#"
        SELECT id, user_id, user_role as "user_role: UserRole", is_revoked, expires_at
        FROM refresh_tokens
        WHERE token_hash = $1
        "#,
        token_hash
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?;

    let token = match token_record {
        Some(t) => t,
        None => return Err(AppError::Unauthorized("Invalid refresh token".to_string())),
    };

    // Reuse Detection: If token is already revoked, invalidate all tokens for this user
    if token.is_revoked {
        tracing::warn!(
            user_id = %token.user_id,
            "Security Alert: Revoked refresh token reuse detected! Invalidating entire token family."
        );

        sqlx::query!(
            "UPDATE refresh_tokens SET is_revoked = TRUE, revoked_at = NOW() WHERE user_id = $1",
            token.user_id
        )
        .execute(&state.db)
        .await
        .map_err(AppError::Database)?;

        return Err(AppError::Unauthorized(
            "Token compromise detected. Please login again.".to_string(),
        ));
    }

    if token.expires_at < Utc::now() {
        return Err(AppError::Unauthorized(
            "Refresh token has expired".to_string(),
        ));
    }

    // Invalidate the current refresh token (single-use rotation)
    sqlx::query!(
        "UPDATE refresh_tokens SET is_revoked = TRUE, revoked_at = NOW() WHERE id = $1",
        token.id
    )
    .execute(&state.db)
    .await
    .map_err(AppError::Database)?;

    // Retrieve user details to populate new AuthResponse
    let (email, name) = match token.user_role {
        UserRole::Admin => {
            let admin = sqlx::query!(
                "SELECT email, name FROM admins WHERE id = $1 AND deleted_at IS NULL",
                token.user_id
            )
            .fetch_optional(&state.db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::Unauthorized("User account no longer active".to_string()))?;
            (admin.email, admin.name)
        }
        UserRole::Worker => {
            let worker = sqlx::query!(
                "SELECT email, name FROM workers WHERE id = $1 AND deleted_at IS NULL",
                token.user_id
            )
            .fetch_optional(&state.db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::Unauthorized("Worker account no longer active".to_string()))?;
            (worker.email, worker.name)
        }
    };

    let auth_response = issue_tokens(&state, token.user_id, token.user_role, &email, &name).await?;

    Ok(Json(SingleResponse::new(auth_response)))
}

/// Revokes a refresh token on user logout.
pub async fn logout_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LogoutRequest>,
) -> Result<StatusCode, AppError> {
    let token_hash = crypto::hash_token(&payload.refresh_token);

    sqlx::query!(
        "UPDATE refresh_tokens SET is_revoked = TRUE, revoked_at = NOW() WHERE token_hash = $1",
        token_hash
    )
    .execute(&state.db)
    .await
    .map_err(AppError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}
