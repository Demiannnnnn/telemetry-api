# Project Constraints

## Hard Constraints (Non-Negotiable)

### Security Constraints

1. **SC-001: Zero-Knowledge Server**
   - The server MUST NOT have the ability to decrypt telemetry data.
   - Encryption keys MUST be managed exclusively on client devices (Admin and Worker apps).
   - No decryption logic shall exist in the server codebase.

2. **SC-002: Encrypted Storage**
   - ALL telemetry data MUST be stored as encrypted blobs in PostgreSQL.
   - Only metadata (timestamps, record IDs, worker IDs, data category) may be stored in plaintext.
   - Metadata MUST NOT contain information that could reveal the content of telemetry data.

3. **SC-003: No Logging of Sensitive Data**
   - Encrypted payloads MUST NOT be logged, even at debug/trace level.
   - Request/response bodies containing encrypted data MUST be excluded from tracing spans.
   - Only metadata (request path, status code, latency, user ID) may be logged.

4. **SC-004: Authentication Required**
   - All endpoints MUST require JWT authentication except:
     - `POST /api/v1/auth/login`
     - `POST /api/v1/auth/register`
     - `GET /api/v1/health`
   - JWT tokens MUST have a maximum lifetime of 24 hours.
   - Refresh tokens MUST implement rotation (single-use).

5. **SC-005: Authorization Boundaries**
   - Admin users can ONLY access data for workers assigned to them.
   - Worker users can ONLY submit their own telemetry data.
   - Cross-admin data access MUST be impossible at the database query level.

### Data Privacy Constraints

6. **DP-001: Keystroke Data**
   - ONLY keystroke frequency (count per time interval) may be collected.
   - Keystroke CONTENT (actual characters typed) MUST NEVER be collected, transmitted, or stored.
   - Any code that could potentially capture keystroke content MUST NOT exist in the codebase.

7. **DP-002: Network Data**
   - ONLY domain names / URLs may be collected.
   - Page content, form data, or POST bodies MUST NEVER be collected.

8. **DP-003: File Data**
   - ONLY file access/modification metadata (path, timestamp, action type) may be collected.
   - File CONTENT MUST NEVER be collected.
   - Only files in designated corporate directories may be monitored.

9. **DP-004: Screenshot Data**
   - Screenshots MUST require a separate, explicit consent flag per worker.
   - The worker MUST receive a visible notification when a screenshot is captured.
   - Screenshots SHOULD be captured at low frequency (configurable, default: no more than once per 10 minutes).

10. **DP-005: Location Data**
    - GPS data MUST ONLY be collected during configured work hours.
    - GPS data MUST ONLY be collected for workers with field roles and explicit consent.
    - GPS precision SHOULD be limited to what is operationally necessary.
    - Location tracking MUST automatically stop outside work hours.

### Technical Constraints

11. **TC-001: Rust Edition**
    - Minimum Supported Rust Version (MSRV): 1.75
    - Edition: 2021

12. **TC-002: Database**
    - PostgreSQL 16+ is required.
    - SQLx is the ONLY allowed database driver.
    - All queries MUST use `sqlx::query!` or `sqlx::query_as!` macros for compile-time verification.
    - Raw string queries (`sqlx::query("...")`) are NOT allowed except in migrations.

13. **TC-003: UUID Format**
    - All primary keys MUST be UUID v7 (time-sortable).
    - UUIDs are generated server-side at insertion time.

14. **TC-004: Timestamps**
    - All timestamps MUST be stored in UTC.
    - All tables MUST have `created_at` (NOT NULL, default NOW()) and `updated_at` (NOT NULL, auto-updated) columns.
    - Use `TIMESTAMPTZ` PostgreSQL type.

15. **TC-005: API Versioning**
    - All endpoints MUST be prefixed with `/api/v1/`.
    - Breaking changes require a new API version (`/api/v2/`).

16. **TC-006: No Unwrap in Production Code**
    - `.unwrap()` and `.expect()` MUST NOT be used in production code.
    - Use proper error handling with `Result<T, E>` and the `?` operator.
    - `.unwrap()` is allowed ONLY in test code.

## Soft Constraints (Strongly Recommended)

17. **Response Times**
    - Health check: < 50ms
    - Authentication endpoints: < 200ms
    - Single record CRUD: < 100ms
    - Batch telemetry ingestion: < 500ms for up to 100 records
    - List/query endpoints: < 300ms

18. **Payload Sizes**
    - Maximum request body: 10 MB (configurable)
    - Maximum batch size: 100 records per request
    - Maximum screenshot payload: 5 MB per image

19. **Rate Limiting**
    - Default: 100 requests/second per authenticated user
    - Telemetry ingestion: 10 requests/second per worker (batch encouraged)
    - Authentication: 5 attempts/minute per IP

20. **Data Retention**
    - Default retention period: configurable per admin (default 90 days)
    - Archived data: moved to cold storage after retention period
    - Deleted data: hard-deleted after retention period + 30 days grace period

## Dependency Constraints

21. **Allowed Crate Categories**
    - Web framework: `axum`, `tower`, `tower-http` only
    - Database: `sqlx` only (no diesel, no sea-orm)
    - Serialization: `serde`, `serde_json`
    - Auth: `jsonwebtoken`
    - Validation: `validator`
    - Error handling: `thiserror`, `anyhow`
    - Logging: `tracing`, `tracing-subscriber`
    - Utilities: `uuid`, `chrono`, `dotenvy`

22. **Prohibited Dependencies**
    - No ORM frameworks (diesel, sea-orm) — SQLx with raw queries is required
    - No web frameworks other than Axum
    - No `unsafe` code without explicit justification and review
    - No cryptographic libraries on the server side (encryption is client-side only)
