# AGENT.md — Telemetry API

## Project Overview

This is the **Telemetry API**, a Rust/Axum REST API that serves as an encrypted middleware between two client applications (Admin and Worker). The API stores and relays end-to-end encrypted telemetry data. The server operates under a **zero-knowledge model** — it never has access to decryption keys or plaintext data.

## Tech Stack

- **Language:** Rust (edition 2021, MSRV 1.75)
- **Framework:** Axum 0.7
- **Database:** PostgreSQL 16 with SQLx
- **Auth:** JWT (jsonwebtoken crate)
- **Async:** Tokio runtime

## Architecture Rules

### Module-Based Organization
- The codebase is organized by **domain modules**, NOT by service layers.
- Each module (e.g., `admin`, `worker`, `telemetry`) contains its own `models.rs`, `handlers.rs`, `routes.rs`.
- Shared code lives in top-level modules: `config/`, `database/`, `errors/`, `middleware/`, `models/`, `crypto/`.

### Entity Relationships
- **Admin (1) → Worker (N):** One admin manages many workers. A worker belongs to exactly one admin.
- **Worker (1) → Telemetry (N):** Each worker generates multiple telemetry records across categories.

### Telemetry Categories
The `telemetry` module has sub-modules for each data category:
1. `system_activity` — Idle time, apps, window titles, CPU/RAM/network, USB events
2. `network_activity` — Domains visited, domain categories
3. `productivity` — Keystroke frequency, click frequency, screenshots
4. `file_activity` — File access/modification logs
5. `location` — GPS coordinates (work hours only)

## Critical Constraints

### Security
- **ALL telemetry data is stored encrypted.** The server stores opaque encrypted blobs.
- **NEVER** implement server-side decryption of telemetry data.
- **NEVER** log or expose encrypted payload contents.
- JWT tokens must have short expiration times with refresh token rotation.
- All endpoints must be authenticated except `/health` and `/auth/login`, `/auth/register`.

### Data Privacy
- Keystroke data is **frequency only**, NEVER content. This is NOT a keylogger.
- Domain data is **URL/domain only**, NEVER page content.
- File data is **access logs only**, NEVER file contents.
- Screenshots require separate explicit consent flags.
- GPS data is restricted to work hours and field roles.

### Database
- Use SQLx with **compile-time query verification** (`sqlx::query!` macros).
- All migrations go in `/migrations` using SQLx migration format.
- Use UUID v7 for primary keys (time-sortable).
- All tables must have `created_at` and `updated_at` timestamps.
- Use soft deletes (`deleted_at` nullable timestamp) where appropriate.

### Code Quality
- Run `cargo fmt` before committing.
- Run `cargo clippy -- -D warnings` — zero warnings policy.
- All public functions must have doc comments.
- Error types use `thiserror` for library errors, `anyhow` only in `main.rs`.
- Use `tracing` for structured logging, never `println!`.

## API Design

- Base path: `/api/v1/`
- RESTful resource naming (plural nouns)
- JSON request/response bodies
- Standard HTTP status codes
- Consistent error response format: `{ "error": { "code": "...", "message": "..." } }`
- Pagination via query params: `?page=1&per_page=20`

## File Naming Conventions

- Snake_case for all Rust files
- Modules: `mod.rs` as the module root
- Models: `models.rs` for structs, enums, DTOs
- Handlers: `handlers.rs` for request handler functions
- Routes: `routes.rs` for route definitions

## Testing

- Unit tests in the same file as the code (`#[cfg(test)]` modules)
- Integration tests in `/tests` directory
- Use `sqlx::test` for database tests with automatic rollback
- Test naming: `test_<module>_<behavior>_<expected_result>`

## Documentation References

- Architecture: `docs/architecture-overview.md`
- Database schema: `docs/database-schema.md`
- API spec: `docs/api-specification.md`
- Security: `docs/security-model.md`
- Legal: `docs/legal-compliance.md`
- AI agent policies: `.ai-agents/`

## Branch Strategy

| Branch | Purpose |
|--------|---------|
| `main` | Production releases |
| `develop` | Integration branch |
| `dev/architecture` | Architecture & docs |
| `dev/features` | Feature development |
