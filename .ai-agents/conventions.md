# Code Conventions & Best Practices

## Rust Code Style

### Formatting
- **Formatter:** Always use `rustfmt` (`cargo fmt`) before committing.
- **Line width:** 100 characters maximum (configure in `rustfmt.toml` if needed).
- **Indentation:** 4 spaces (Rust default).
- **Trailing commas:** Always use trailing commas in multi-line constructs.

### Naming Conventions

| Item | Convention | Example |
|------|-----------|---------|
| Modules | `snake_case` | `system_activity` |
| Files | `snake_case.rs` | `network_activity.rs` |
| Structs | `PascalCase` | `WorkerProfile` |
| Enums | `PascalCase` | `TelemetryCategory` |
| Enum Variants | `PascalCase` | `SystemActivity` |
| Functions | `snake_case` | `get_worker_by_id` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_BATCH_SIZE` |
| Type aliases | `PascalCase` | `DbPool` |
| Traits | `PascalCase` | `Validatable` |
| Macros | `snake_case!` | `query!` |

### Handler Function Naming

Handler functions follow the pattern: `{action}_{entity}` or `{action}_{entity}_{qualifier}`

```
create_worker       # POST /workers
get_worker          # GET /workers/:id
list_workers        # GET /workers
update_worker       # PUT /workers/:id
delete_worker       # DELETE /workers/:id
get_worker_telemetry # GET /workers/:id/telemetry
```

### Model Naming

| Type | Suffix | Example | Purpose |
|------|--------|---------|---------|
| Database model | (none) | `Worker` | Direct DB row mapping |
| Create request | `CreateRequest` | `CreateWorkerRequest` | POST body |
| Update request | `UpdateRequest` | `UpdateWorkerRequest` | PUT/PATCH body |
| Response | `Response` | `WorkerResponse` | API response |
| List response | `ListResponse` | `WorkerListResponse` | Paginated list |
| Query params | `QueryParams` | `TelemetryQueryParams` | GET query parameters |

## Error Handling

### Rules

1. **Never use `.unwrap()` or `.expect()` in production code.**
2. **Always propagate errors with `?` operator.**
3. **Define specific error types with `thiserror`.**
4. **Map errors to appropriate HTTP status codes.**

### Error Response Format

All API errors MUST return this JSON structure:

```json
{
  "error": {
    "code": "RESOURCE_NOT_FOUND",
    "message": "Worker with ID '...' not found",
    "details": null
  }
}
```

### Standard Error Codes

| Code | HTTP Status | Usage |
|------|-------------|-------|
| `VALIDATION_ERROR` | 400 | Invalid request body/params |
| `INVALID_CREDENTIALS` | 401 | Wrong username/password |
| `TOKEN_EXPIRED` | 401 | JWT token expired |
| `UNAUTHORIZED` | 401 | Missing or invalid token |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `RESOURCE_NOT_FOUND` | 404 | Entity not found |
| `CONFLICT` | 409 | Duplicate resource |
| `PAYLOAD_TOO_LARGE` | 413 | Request body exceeds limit |
| `RATE_LIMITED` | 429 | Too many requests |
| `INTERNAL_ERROR` | 500 | Unexpected server error |

## API Design Conventions

### URL Structure

```
/api/v1/{resource}           # Collection
/api/v1/{resource}/{id}      # Single resource
/api/v1/{resource}/{id}/{sub} # Sub-resource
```

### HTTP Methods

| Method | Action | Idempotent | Request Body |
|--------|--------|------------|--------------|
| GET | Read | Yes | No |
| POST | Create | No | Yes |
| PUT | Full update | Yes | Yes |
| PATCH | Partial update | Yes | Yes |
| DELETE | Delete | Yes | No |

### Pagination

All list endpoints MUST support pagination:

```
GET /api/v1/workers?page=1&per_page=20

Response headers:
X-Total-Count: 150
X-Total-Pages: 8
X-Current-Page: 1
X-Per-Page: 20
```

Response body:
```json
{
  "data": [...],
  "pagination": {
    "total": 150,
    "page": 1,
    "per_page": 20,
    "total_pages": 8
  }
}
```

### Filtering and Sorting

```
GET /api/v1/telemetry/system?worker_id=...&from=2024-01-01&to=2024-01-31&sort=created_at&order=desc
```

### Response Envelope

All successful responses use this structure:

```json
// Single resource
{
  "data": { ... }
}

// Collection
{
  "data": [ ... ],
  "pagination": { ... }
}

// Action result (no content)
// HTTP 204 No Content
```

## Logging Conventions

### Log Levels

| Level | Usage |
|-------|-------|
| `ERROR` | Unexpected failures that need investigation |
| `WARN` | Expected but noteworthy conditions (rate limits hit, invalid tokens) |
| `INFO` | Significant business events (user login, worker registered, telemetry ingested) |
| `DEBUG` | Detailed diagnostic info (query execution, handler entry/exit) |
| `TRACE` | Very detailed diagnostic info (request/response metadata, NOT bodies) |

### Structured Logging

Always use structured fields with `tracing`:

```rust
// Good
tracing::info!(worker_id = %id, category = "system_activity", count = records.len(), "Telemetry batch ingested");

// Bad
tracing::info!("Telemetry batch ingested for worker {} with {} records", id, records.len());
```

### What to Log

- ✅ Authentication events (login, logout, token refresh)
- ✅ Authorization failures (access denied)
- ✅ CRUD operations (entity created/updated/deleted) with entity IDs
- ✅ Telemetry ingestion metadata (worker ID, category, batch size)
- ✅ Error conditions with context

### What NOT to Log

- ❌ Encrypted payload content (even partially)
- ❌ JWT token values
- ❌ Passwords or credentials
- ❌ Full request/response bodies
- ❌ Personal identifying information (names, emails in logs)

## Database Conventions

### Table Naming

- Tables: `snake_case`, plural (`workers`, `telemetry_system_activities`)
- Columns: `snake_case` (`worker_id`, `created_at`)
- Indexes: `idx_{table}_{column(s)}` (`idx_workers_admin_id`)
- Foreign keys: `fk_{table}_{referenced_table}` (`fk_workers_admins`)
- Constraints: `chk_{table}_{column}` (`chk_workers_email_format`)

### Standard Columns

Every table MUST include:

```sql
id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),  -- UUID v7 preferred
created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
```

### Soft Deletes

Tables that support soft deletes add:

```sql
deleted_at  TIMESTAMPTZ  -- NULL means active, timestamp means deleted
```

### Migration Naming

Migrations follow the SQLx naming convention:

```
{timestamp}_{description}.sql

Example:
20240101000000_create_admins_table.sql
20240101000001_create_workers_table.sql
20240101000002_create_telemetry_system_activities_table.sql
```

## Git Conventions

### Commit Messages

Follow Conventional Commits:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat` — New feature
- `fix` — Bug fix
- `docs` — Documentation only
- `refactor` — Code refactoring
- `test` — Adding or updating tests
- `chore` — Build, CI, or tooling changes
- `perf` — Performance improvement
- `security` — Security fix or improvement

Examples:
```
feat(auth): implement JWT token refresh rotation
fix(telemetry): prevent duplicate batch submissions
docs(api): add endpoint documentation for file activity
refactor(worker): extract validation logic to shared module
```

### Branch Naming

```
main                    # Production
develop                 # Integration
dev/architecture        # Architecture & documentation
dev/features            # Feature development
feature/{ticket}-{desc} # Individual features
fix/{ticket}-{desc}     # Bug fixes
```

## Testing Conventions

### Test Organization

```rust
// Unit tests - in the same file
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_creation_with_valid_data_succeeds() { ... }

    #[test]
    fn test_worker_creation_with_missing_email_fails() { ... }
}
```

### Test Naming Pattern

```
test_{module}_{action}_{condition}_{expected_result}

Examples:
test_auth_login_with_valid_credentials_returns_token
test_auth_login_with_invalid_password_returns_401
test_worker_create_with_duplicate_email_returns_conflict
test_telemetry_batch_exceeding_limit_returns_413
```

### Integration Tests

```
tests/
├── common/
│   └── mod.rs          # Shared test utilities
├── auth_tests.rs
├── admin_tests.rs
├── worker_tests.rs
└── telemetry_tests.rs
```

## Documentation Conventions

### Rust Doc Comments

All public items MUST have documentation comments:

```rust
/// Creates a new worker and associates them with the specified admin.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `admin_id` - UUID of the admin who owns this worker
/// * `request` - Worker creation request with profile details
///
/// # Returns
///
/// The created worker's response DTO.
///
/// # Errors
///
/// Returns `AppError::Conflict` if a worker with the same email already exists.
/// Returns `AppError::NotFound` if the admin_id does not exist.
pub async fn create_worker(
    pool: &PgPool,
    admin_id: Uuid,
    request: CreateWorkerRequest,
) -> Result<WorkerResponse, AppError> {
    // ...
}
```

### Module-Level Documentation

Each `mod.rs` MUST have a module-level doc comment:

```rust
//! # Worker Module
//!
//! Handles worker registration, profile management, and admin-worker associations.
//!
//! ## Endpoints
//!
//! - `POST /api/v1/workers` - Register a new worker
//! - `GET /api/v1/workers/:id` - Get worker profile
//! - `GET /api/v1/workers` - List workers (admin only)
//! - `PUT /api/v1/workers/:id` - Update worker profile
//! - `DELETE /api/v1/workers/:id` - Deactivate worker
```
