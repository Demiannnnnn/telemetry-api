# Architecture Overview

## System Context

The Telemetry API is the central component in a three-part system designed for enterprise device telemetry with end-to-end encryption.

```
                    ┌─────────────────────────────────────────────┐
                    │              EXTERNAL SYSTEMS                │
                    │                                             │
                    │  ┌─────────────┐       ┌─────────────┐     │
                    │  │  Worker App │       │  Admin App  │     │
                    │  │  (Mobile/   │       │  (Web/      │     │
                    │  │   Desktop)  │       │   Desktop)  │     │
                    │  └──────┬──────┘       └──────┬──────┘     │
                    │         │                      │            │
                    └─────────┼──────────────────────┼────────────┘
                              │                      │
                              │   HTTPS / TLS 1.3    │
                              │                      │
                    ┌─────────┼──────────────────────┼────────────┐
                    │         ▼                      ▼            │
                    │  ┌─────────────────────────────────────┐    │
                    │  │          TELEMETRY API               │    │
                    │  │       (Rust / Axum)                  │    │
                    │  │                                      │    │
                    │  │  ┌──────┐ ┌──────┐ ┌──────────┐     │    │
                    │  │  │ Auth │ │ CRUD │ │ Telemetry│     │    │
                    │  │  │      │ │      │ │ Ingestion│     │    │
                    │  │  └──────┘ └──────┘ └──────────┘     │    │
                    │  └──────────────┬──────────────────┘    │    │
                    │                 │                        │    │
                    │          ┌──────┴──────┐                │    │
                    │          │ PostgreSQL  │                │    │
                    │          │  Database   │                │    │
                    │          └─────────────┘                │    │
                    │              SERVER                      │    │
                    └─────────────────────────────────────────────┘
```

## Component Architecture

### Layer Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    HTTP Layer (Axum)                         │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                   Middleware Stack                    │    │
│  │  CORS → Tracing → RateLimit → Authentication        │    │
│  └─────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────┤
│                    Routing Layer                             │
│  ┌──────┐  ┌──────┐  ┌──────┐  ┌────────────┐             │
│  │/auth │  │/admin│  │/worker│ │/telemetry/* │             │
│  └──┬───┘  └──┬───┘  └──┬───┘  └──────┬─────┘             │
│     │         │         │              │                     │
├─────┼─────────┼─────────┼──────────────┼─────────────────────┤
│     ▼         ▼         ▼              ▼                     │
│                    Handler Layer                             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Input Validation → Business Logic → Response Build  │   │
│  └──────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│                    Data Access Layer                         │
│  ┌──────────────────────────────────────────────────────┐   │
│  │         SQLx Query Macros (Compile-time verified)     │   │
│  └──────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│                    Database Layer                            │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              PostgreSQL 16 (Encrypted Data)           │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Module Dependency Diagram

```
                        ┌─────────┐
                        │  main   │
                        └────┬────┘
                             │
                    ┌────────┼────────┐
                    │        │        │
                    ▼        ▼        ▼
              ┌─────────┐ ┌─────┐ ┌───────┐
              │ config  │ │routes│ │  db   │
              └─────────┘ └──┬──┘ └───┬───┘
                             │        │
              ┌──────────────┼────────┼──────────────┐
              │              │        │              │
              ▼              ▼        ▼              ▼
         ┌─────────┐   ┌─────────┐  ┌──────┐  ┌──────────┐
         │  auth   │   │  admin  │  │worker│  │telemetry │
         └─────────┘   └─────────┘  └──────┘  └────┬─────┘
                                                     │
                            ┌────────────────────────┼────────────────┐
                            │            │           │        │       │
                            ▼            ▼           ▼        ▼       ▼
                       ┌─────────┐ ┌──────────┐ ┌───────┐ ┌─────┐ ┌────┐
                       │ system  │ │ network  │ │ prod. │ │file │ │loc │
                       │activity │ │ activity │ │metrics│ │actv │ │data│
                       └─────────┘ └──────────┘ └───────┘ └─────┘ └────┘

         ┌──────────────────────────────────────────────────────────┐
         │           Shared Infrastructure Modules                  │
         │  ┌────────┐ ┌────────┐ ┌──────────┐ ┌────────┐         │
         │  │ errors │ │ models │ │middleware│ │ crypto │         │
         │  └────────┘ └────────┘ └──────────┘ └────────┘         │
         └──────────────────────────────────────────────────────────┘
```

## Request Lifecycle

### 1. Incoming Request Flow

```
Client Request (HTTPS)
        │
        ▼
┌───────────────────┐
│   TLS Termination │  (handled by reverse proxy or Axum+rustls)
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│    CORS Layer     │  Check origin, methods, headers
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│  Tracing Layer    │  Create request span with trace_id
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│ Rate Limit Layer  │  Check per-user/per-IP limits
└────────┬──────────┘
         │          ── 429 Too Many Requests ──▶ Client
         ▼
┌───────────────────┐
│  Auth Middleware   │  Validate JWT, extract claims
└────────┬──────────┘
         │          ── 401 Unauthorized ──▶ Client
         ▼
┌───────────────────┐
│   Route Matching  │  Match path to handler
└────────┬──────────┘
         │          ── 404 Not Found ──▶ Client
         ▼
┌───────────────────┐
│  Input Extraction │  Deserialize + validate body/params
└────────┬──────────┘
         │          ── 400 Bad Request ──▶ Client
         ▼
┌───────────────────┐
│  Handler Logic    │  Authorization check + business logic
└────────┬──────────┘
         │          ── 403 Forbidden ──▶ Client
         ▼
┌───────────────────┐
│  Database Query   │  SQLx compile-time verified query
└────────┬──────────┘
         │          ── 500 Internal Error ──▶ Client
         ▼
┌───────────────────┐
│ Response Building │  Serialize response DTO to JSON
└────────┬──────────┘
         │
         ▼
    Client Response
```

### 2. Telemetry Ingestion Flow (Worker → Server)

```
Worker App                           Telemetry API                    PostgreSQL
─────────                           ─────────────                    ──────────
    │                                     │                               │
    │  1. Collect raw telemetry           │                               │
    │  2. Encrypt with E2E key           │                               │
    │  3. Batch multiple records          │                               │
    │                                     │                               │
    │  POST /api/v1/telemetry/system     │                               │
    │  Authorization: Bearer <jwt>        │                               │
    │  {                                  │                               │
    │    "records": [                     │                               │
    │      {                              │                               │
    │        "encrypted_payload": "...",  │                               │
    │        "client_timestamp": "...",   │                               │
    │        "category": "system"         │                               │
    │      }                              │                               │
    │    ]                                │                               │
    │  }                                  │                               │
    │ ───────────────────────────────────▶│                               │
    │                                     │  4. Validate JWT              │
    │                                     │  5. Verify worker identity    │
    │                                     │  6. Validate batch size       │
    │                                     │  7. Check consent flags       │
    │                                     │                               │
    │                                     │  INSERT INTO telemetry_...    │
    │                                     │ ─────────────────────────────▶│
    │                                     │                               │
    │                                     │◀──────────────────────────────│
    │                                     │  8. Return inserted IDs       │
    │◀────────────────────────────────────│                               │
    │  201 Created                        │                               │
    │  {                                  │                               │
    │    "data": {                        │                               │
    │      "batch_id": "...",             │                               │
    │      "records_created": 5,          │                               │
    │      "server_timestamp": "..."      │                               │
    │    }                                │                               │
    │  }                                  │                               │
```

### 3. Telemetry Retrieval Flow (Server → Admin)

```
Admin App                            Telemetry API                    PostgreSQL
─────────                            ─────────────                    ──────────
    │                                     │                               │
    │  GET /api/v1/telemetry/system       │                               │
    │  ?worker_id=...&from=...&to=...     │                               │
    │  Authorization: Bearer <jwt>         │                               │
    │ ───────────────────────────────────▶│                               │
    │                                     │  1. Validate JWT (admin role) │
    │                                     │  2. Verify admin owns worker  │
    │                                     │  3. Parse query params        │
    │                                     │                               │
    │                                     │  SELECT * FROM telemetry_...  │
    │                                     │  WHERE worker_id = $1         │
    │                                     │  AND created_at BETWEEN ...   │
    │                                     │ ─────────────────────────────▶│
    │                                     │                               │
    │                                     │◀──────────────────────────────│
    │                                     │  4. Return encrypted records  │
    │◀────────────────────────────────────│                               │
    │  200 OK                             │                               │
    │  {                                  │                               │
    │    "data": [                        │                               │
    │      {                              │                               │
    │        "id": "...",                 │                               │
    │        "encrypted_payload": "...",  │                               │
    │        "client_timestamp": "...",   │                               │
    │        "server_timestamp": "..."    │                               │
    │      }                              │                               │
    │    ],                               │                               │
    │    "pagination": { ... }            │                               │
    │  }                                  │                               │
    │                                     │                               │
    │  5. Decrypt payloads with E2E key  │                               │
    │  6. Display telemetry dashboard     │                               │
```

## Deployment Architecture

### Minimal Production Deployment

```
┌─────────────────────────────────────────────────────────────┐
│                     Production Server                        │
│                                                             │
│  ┌───────────────┐     ┌──────────────────┐                │
│  │  Nginx/Caddy  │────▶│  Telemetry API   │                │
│  │  (TLS + Proxy)│     │  (Rust Binary)   │                │
│  │  :443         │     │  :8080           │                │
│  └───────────────┘     └────────┬─────────┘                │
│                                 │                           │
│                        ┌────────┴─────────┐                │
│                        │   PostgreSQL 16  │                │
│                        │   :5432          │                │
│                        └──────────────────┘                │
└─────────────────────────────────────────────────────────────┘
```

### Scalable Production Deployment (Future)

```
                    ┌─────────────┐
                    │ Load Balancer│
                    │ (TLS Term.) │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
              ▼            ▼            ▼
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │ API #1   │ │ API #2   │ │ API #3   │
        │ (Rust)   │ │ (Rust)   │ │ (Rust)   │
        └────┬─────┘ └────┬─────┘ └────┬─────┘
             │             │             │
             └─────────────┼─────────────┘
                           │
                    ┌──────┴──────┐
                    │ PostgreSQL  │
                    │ Primary     │
                    └──────┬──────┘
                           │
                    ┌──────┴──────┐
                    │ PostgreSQL  │
                    │ Read Replica│
                    └─────────────┘
```

## Technology Decision Records

### TDR-001: Rust + Axum over alternatives

**Decision:** Use Rust with Axum as the web framework.

**Alternatives considered:**
- Go + Gin — Good performance, but less type safety
- Node.js + Express — Faster development, but lower performance
- Python + FastAPI — Good developer experience, but insufficient performance for telemetry workloads

**Rationale:**
- Maximum performance for telemetry ingestion workloads
- Memory safety without garbage collection pauses
- Type system prevents entire classes of bugs
- Axum's extractor pattern provides ergonomic request handling
- Tokio's async runtime handles high-concurrency efficiently
- Low memory footprint for deployment

### TDR-002: SQLx over ORM

**Decision:** Use SQLx with raw SQL queries over an ORM (Diesel, SeaORM).

**Rationale:**
- Compile-time SQL verification catches query errors early
- Full control over generated SQL for performance tuning
- No impedance mismatch between domain models and SQL
- Simpler mental model — SQL is well understood
- Better performance — no ORM overhead

### TDR-003: Module-based over Service-based architecture

**Decision:** Organize code by domain modules rather than technical service layers.

**Rationale:**
- Higher cohesion — related code lives together
- Easier navigation — find all code for a feature in one directory
- Better encapsulation — module internals are hidden
- Simpler refactoring — changes are localized to one module
- Future-proof — modules can be extracted to microservices

### TDR-004: UUID v7 for Primary Keys

**Decision:** Use UUID v7 (time-sortable) instead of auto-increment integers or UUID v4.

**Rationale:**
- Time-sortable — natural ordering without separate timestamp index
- No sequence contention — better for concurrent inserts
- Globally unique — safe for distributed future
- 128-bit — sufficient key space
- PostgreSQL native support
