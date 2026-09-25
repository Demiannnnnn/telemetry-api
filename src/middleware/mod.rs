//! # Middleware Module
//!
//! Security middleware pipeline, CORS configuration, response security headers,
//! rate limiting, and Axum extractors for role-based authentication guards.

use crate::{
    config::Config,
    crypto::{self, UserRole},
    database::DbPool,
    errors::AppError,
    AppState,
};
use axum::{
    async_trait,
    body::Body,
    extract::{FromRef, FromRequestParts},
    http::{
        header::{self, HeaderName, HeaderValue, AUTHORIZATION},
        request::Parts,
        Method, Request, Response,
    },
    middleware::Next,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use uuid::Uuid;

/// Extracted authenticated user details from a validated JWT Bearer token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthUser {
    /// Subject user UUID.
    pub id: Uuid,
    /// Authenticated role (Admin or Worker).
    pub role: UserRole,
    /// User email address.
    pub email: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    Arc<AppState>: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = Arc::<AppState>::from_ref(state);

        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|val| val.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            AppError::Unauthorized(
                "Invalid Authorization header format. Expected Bearer <token>".to_string(),
            )
        })?;

        let claims = crypto::verify_jwt(token, &app_state.config.jwt_secret)?;

        Ok(AuthUser {
            id: claims.sub,
            role: claims.role,
            email: claims.email,
        })
    }
}

/// Extractor Guard that requires the user to have administrator role (`UserRole::Admin`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequireAdmin(pub AuthUser);

#[async_trait]
impl<S> FromRequestParts<S> for RequireAdmin
where
    S: Send + Sync,
    Arc<AppState>: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        if auth_user.role != UserRole::Admin {
            return Err(AppError::Forbidden(
                "Administrator privileges required".to_string(),
            ));
        }
        Ok(RequireAdmin(auth_user))
    }
}

/// Extractor Guard that requires the user to have worker role (`UserRole::Worker`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequireWorker(pub AuthUser);

#[async_trait]
impl<S> FromRequestParts<S> for RequireWorker
where
    S: Send + Sync,
    Arc<AppState>: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        if auth_user.role != UserRole::Worker {
            return Err(AppError::Forbidden(
                "Worker privileges required".to_string(),
            ));
        }
        Ok(RequireWorker(auth_user))
    }
}

/// Verifies that a worker exists and is managed by the specified administrator.
///
/// # Arguments
///
/// * `admin_id` - UUID of the authenticated administrator
/// * `worker_id` - UUID of the target worker
/// * `pool` - PostgreSQL connection pool
///
/// # Errors
///
/// Returns [`AppError::NotFound`] if the worker does not exist or has been deleted.
/// Returns [`AppError::Forbidden`] if the worker belongs to another administrator.
pub async fn verify_admin_owns_worker(
    admin_id: Uuid,
    worker_id: Uuid,
    pool: &DbPool,
) -> Result<(), AppError> {
    let record = sqlx::query!(
        "SELECT admin_id FROM workers WHERE id = $1 AND deleted_at IS NULL",
        worker_id
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    match record {
        Some(row) if row.admin_id == admin_id => Ok(()),
        Some(_) => Err(AppError::Forbidden(
            "You do not have permission to access this worker's data".to_string(),
        )),
        None => Err(AppError::NotFound(format!(
            "Worker with ID '{}' not found",
            worker_id
        ))),
    }
}

/// In-memory sliding-window rate limiter.
#[derive(Debug, Default)]
pub struct InMemoryRateLimiter {
    history: Mutex<HashMap<String, Vec<Instant>>>,
}

impl InMemoryRateLimiter {
    /// Creates a new in-memory rate limiter.
    pub fn new() -> Self {
        Self {
            history: Mutex::new(HashMap::new()),
        }
    }

    /// Checks if a request from the given key is allowed under the rate limit.
    ///
    /// # Arguments
    ///
    /// * `key` - Client identifier (e.g. IP or user ID)
    /// * `max_requests` - Maximum number of requests allowed in the time window
    /// * `window` - Time window duration
    pub fn check_allowed(&self, key: &str, max_requests: usize, window: Duration) -> bool {
        let Ok(mut map) = self.history.lock() else {
            return true;
        };

        let now = Instant::now();
        let cutoff = now.checked_sub(window).unwrap_or(now);

        let timestamps = map.entry(key.to_string()).or_default();
        timestamps.retain(|&t| t > cutoff);

        if timestamps.len() >= max_requests {
            false
        } else {
            timestamps.push(now);
            true
        }
    }
}

/// Injects standard HTTP security headers into all responses.
pub async fn security_headers_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        HeaderName::from_static("x-xss-protection"),
        HeaderValue::from_static("0"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate"),
    );
    headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );

    response
}

/// Constructs a CORS layer based on configuration origins.
pub fn build_cors_layer(config: &Config) -> CorsLayer {
    let allowed_origins: Vec<HeaderValue> = config
        .cors_allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any)
        .expose_headers([
            HeaderName::from_static("x-total-count"),
            HeaderName::from_static("x-total-pages"),
            HeaderName::from_static("x-ratelimit-limit"),
            HeaderName::from_static("x-ratelimit-remaining"),
            HeaderName::from_static("x-ratelimit-reset"),
        ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    #[tokio::test]
    async fn test_middleware_auth_missing_header_returns_401() {
        let config = Config::from_lookup(|k| match k {
            "DATABASE_URL" => Some("postgres://localhost/test".to_string()),
            "JWT_SECRET" => Some("this-is-a-valid-32-character-secret-key!!".to_string()),
            _ => None,
        })
        .expect("config");

        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").expect("pool");
        let state = Arc::new(AppState::new(pool, config));

        let req = Request::builder().body(Body::empty()).unwrap();
        let (mut parts, _) = req.into_parts();

        let result = AuthUser::from_request_parts(&mut parts, &state).await;
        assert!(
            matches!(result, Err(AppError::Unauthorized(msg)) if msg.contains("Missing Authorization"))
        );
    }

    #[tokio::test]
    async fn test_middleware_auth_invalid_bearer_prefix_returns_401() {
        let config = Config::from_lookup(|k| match k {
            "DATABASE_URL" => Some("postgres://localhost/test".to_string()),
            "JWT_SECRET" => Some("this-is-a-valid-32-character-secret-key!!".to_string()),
            _ => None,
        })
        .expect("config");

        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").expect("pool");
        let state = Arc::new(AppState::new(pool, config));

        let req = Request::builder()
            .header(AUTHORIZATION, "Basic 12345")
            .body(Body::empty())
            .unwrap();
        let (mut parts, _) = req.into_parts();

        let result = AuthUser::from_request_parts(&mut parts, &state).await;
        assert!(
            matches!(result, Err(AppError::Unauthorized(msg)) if msg.contains("Expected Bearer"))
        );
    }

    #[tokio::test]
    async fn test_middleware_require_admin_with_worker_token_returns_403() {
        let config = Config::from_lookup(|k| match k {
            "DATABASE_URL" => Some("postgres://localhost/test".to_string()),
            "JWT_SECRET" => Some("this-is-a-valid-32-character-secret-key!!".to_string()),
            _ => None,
        })
        .expect("config");

        let worker_token = crypto::generate_access_token(
            Uuid::new_v4(),
            UserRole::Worker,
            "worker@example.com",
            &config.jwt_secret,
            1,
        )
        .expect("token");

        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").expect("pool");
        let state = Arc::new(AppState::new(pool, config));

        let req = Request::builder()
            .header(AUTHORIZATION, format!("Bearer {worker_token}"))
            .body(Body::empty())
            .unwrap();
        let (mut parts, _) = req.into_parts();

        let result = RequireAdmin::from_request_parts(&mut parts, &state).await;
        assert!(
            matches!(result, Err(AppError::Forbidden(msg)) if msg.contains("Administrator privileges required"))
        );
    }

    #[tokio::test]
    async fn test_ownership_guard_admin_accessing_other_admin_worker_returns_403() {
        let config = Config::from_env().ok();
        let Some(config) = config else {
            eprintln!("Skipping DB test: .env not loaded");
            return;
        };
        let Ok(pool) = crate::database::init_pool(&config).await else {
            eprintln!("Skipping DB test: DB not reachable");
            return;
        };

        // Create Admin A
        let admin_a: (Uuid,) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization) VALUES ($1, 'hash', 'Admin A', 'Org A') RETURNING id",
        )
        .bind(format!("admin_a_{}@example.com", Uuid::new_v4()))
        .fetch_one(&pool)
        .await
        .expect("Admin A insert");

        // Create Admin B
        let admin_b: (Uuid,) = sqlx::query_as(
            "INSERT INTO admins (email, password_hash, name, organization) VALUES ($1, 'hash', 'Admin B', 'Org B') RETURNING id",
        )
        .bind(format!("admin_b_{}@example.com", Uuid::new_v4()))
        .fetch_one(&pool)
        .await
        .expect("Admin B insert");

        // Create Worker under Admin A
        let worker_a: (Uuid,) = sqlx::query_as(
            "INSERT INTO workers (admin_id, email, name, device_identifier, role_type) VALUES ($1, $2, 'Worker A', 'dev-a', 'OFFICE') RETURNING id",
        )
        .bind(admin_a.0)
        .bind(format!("worker_a_{}@example.com", Uuid::new_v4()))
        .fetch_one(&pool)
        .await
        .expect("Worker A insert");

        // Admin A accessing Worker A -> Should succeed
        let own_res = verify_admin_owns_worker(admin_a.0, worker_a.0, &pool).await;
        assert!(own_res.is_ok());

        // Admin B accessing Worker A -> Should return Forbidden (403)
        let other_res = verify_admin_owns_worker(admin_b.0, worker_a.0, &pool).await;
        assert!(matches!(other_res, Err(AppError::Forbidden(_))));

        // Accessing nonexistent worker -> Should return NotFound (404)
        let not_found_res = verify_admin_owns_worker(admin_a.0, Uuid::new_v4(), &pool).await;
        assert!(matches!(not_found_res, Err(AppError::NotFound(_))));
    }

    #[test]
    fn test_in_memory_rate_limiter() {
        let limiter = InMemoryRateLimiter::new();
        let key = "user-123";

        // Allow 3 requests per 10 seconds
        assert!(limiter.check_allowed(key, 3, Duration::from_secs(10)));
        assert!(limiter.check_allowed(key, 3, Duration::from_secs(10)));
        assert!(limiter.check_allowed(key, 3, Duration::from_secs(10)));

        // 4th request must be rejected
        assert!(!limiter.check_allowed(key, 3, Duration::from_secs(10)));
    }
}
