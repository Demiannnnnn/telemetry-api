# Architecture Decisions & Patterns

## Architectural Style

The Telemetry API follows a **Modular Monolith** architecture with clear module boundaries. This approach was chosen over microservices for the initial version because:

1. **Simplicity** — Single deployment unit, simpler operations
2. **Performance** — No inter-service network overhead
3. **Consistency** — Single database transaction guarantees
4. **Evolvability** — Well-defined module boundaries allow future extraction into microservices

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Telemetry API                            │
│                                                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │   Auth   │  │  Admin   │  │  Worker  │  │Telemetry │       │
│  │  Module  │  │  Module  │  │  Module  │  │  Module  │       │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘       │
│       │              │              │              │             │
│  ┌────┴──────────────┴──────────────┴──────────────┴─────┐      │
│  │              Shared Infrastructure                     │      │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐         │      │
│  │  │ Config │ │Database│ │Middleware│ │ Errors │         │      │
│  │  └────────┘ └────────┘ └────────┘ └────────┘         │      │
│  │  ┌────────┐ ┌────────┐ ┌────────┐                     │      │
│  │  │ Crypto │ │ Models │ │ Routes │                     │      │
│  │  └────────┘ └────────┘ └────────┘                     │      │
│  └───────────────────────────────────────────────────────┘      │
│                              │                                   │
│                     ┌────────┴────────┐                          │
│                     │   PostgreSQL    │                          │
│                     │  (Encrypted     │                          │
│                     │   Storage)      │                          │
│                     └─────────────────┘                          │
└─────────────────────────────────────────────────────────────────┘
```

## Design Patterns

### 1. Repository Pattern (Implicit via SQLx)

While we don't use a traditional Repository trait abstraction, each module's `handlers.rs` interacts with the database through SQLx query macros. This provides:

- Compile-time SQL verification
- Type-safe database interactions
- Direct mapping to domain models

**Rationale:** Adding a Repository trait layer would introduce unnecessary indirection for a project of this scale. SQLx's compile-time verification provides sufficient safety guarantees.

### 2. Handler Pattern (Axum Extractors)

Each module exposes handler functions that use Axum's extractor pattern:

```
Request → Extractors (Auth, Path, Query, Body) → Handler Logic → Response
```

Handlers are thin functions that:
1. Extract and validate input via Axum extractors
2. Execute business logic (database queries, transformations)
3. Return typed responses

### 3. Middleware Chain

```
Request
  │
  ▼
┌──────────────┐
│  CORS Layer  │
├──────────────┤
│  Tracing     │
├──────────────┤
│  Rate Limit  │
├──────────────┤
│  Auth Guard  │  ◄── JWT validation, role extraction
├──────────────┤
│  Handler     │  ◄── Business logic
└──────────────┘
  │
  ▼
Response
```

### 4. Error Handling Pattern

Centralized error handling using a custom `AppError` enum that implements `IntoResponse`:

- Each module can define module-specific error variants
- All errors map to appropriate HTTP status codes
- Error responses follow a consistent JSON format
- Sensitive information is never leaked in error messages

### 5. DTO Pattern (Data Transfer Objects)

Separation between:
- **Database Models** — Direct mapping to database rows (internal)
- **Request DTOs** — Validated input structures
- **Response DTOs** — Sanitized output structures

This ensures that internal database structure changes don't affect the API contract.

## Module Design

### Module Internal Structure

Each domain module follows this internal structure:

```
module/
├── mod.rs        # Module root — re-exports and sub-module declarations
├── models.rs     # Database models, request/response DTOs
├── handlers.rs   # Axum handler functions
└── routes.rs     # Router definition for this module
```

### Module Communication Rules

1. **Modules communicate only through their public API** (exported types and functions)
2. **No circular dependencies** between modules
3. **Shared types** live in the top-level `models/` module
4. **Database access** is encapsulated within each module's handlers
5. **Cross-module queries** should go through handler functions, not direct SQL

### Dependency Graph

```
auth ◄──── admin
  ▲          │
  │          ▼
  └──── worker ────► telemetry
                         │
                         ├── system_activity
                         ├── network_activity
                         ├── productivity
                         ├── file_activity
                         └── location
```

## Data Flow Architecture

### Worker → Server (Data Ingestion)

```
1. Worker app collects telemetry data locally
2. Worker app encrypts data with shared E2E key
3. Worker app batches encrypted payloads
4. Worker app sends POST request to /api/v1/telemetry/{category}
5. Server validates JWT token and worker identity
6. Server stores encrypted blob in PostgreSQL
7. Server returns 201 Created with metadata (timestamp, record ID)
```

### Server → Admin (Data Retrieval)

```
1. Admin app requests GET /api/v1/telemetry/{category}?worker_id=...
2. Server validates JWT token and admin identity
3. Server verifies admin owns the requested worker
4. Server retrieves encrypted blobs from PostgreSQL
5. Server returns encrypted payloads with metadata
6. Admin app decrypts payloads with shared E2E key
7. Admin app displays telemetry data
```

## State Management

- **No server-side sessions** — Stateless JWT authentication
- **Database as single source of truth** — No in-memory caching of business data (initially)
- **Connection pooling** — SQLx pool managed at application startup
- **Configuration** — Loaded once at startup from environment variables

## Scalability Considerations (Future)

1. **Read replicas** — For admin queries that don't need write consistency
2. **Partitioning** — Telemetry tables partitioned by `created_at` for time-range queries
3. **Batch processing** — Worker apps should batch telemetry data (configurable interval)
4. **Compression** — Encrypted payloads can be compressed before encryption on client side
5. **Archival** — Automated archival of telemetry data older than retention period
