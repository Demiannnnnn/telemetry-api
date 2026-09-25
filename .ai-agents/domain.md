# Domain Model

## Business Context

The Telemetry API exists within the **Enterprise Device Management (EDM)** domain. It provides the backend infrastructure for monitoring enterprise-owned devices used by employees (workers), under the supervision of administrators. The platform's primary value proposition is **privacy-first telemetry** — collecting operational metrics while ensuring that the server operator cannot access the actual data content through end-to-end encryption.

## Ubiquitous Language

The following terms have specific meanings within this domain and MUST be used consistently throughout the codebase, documentation, and communication:

| Term | Definition |
|------|-----------|
| **Admin** | An administrator who manages one or more workers. Has permission to view (decrypt) telemetry data for their workers. Holds the E2E decryption key. |
| **Worker** | An employee whose enterprise device runs the worker app. Generates telemetry data. Has given explicit consent. Holds the E2E encryption key. |
| **Telemetry** | Operational metrics collected from worker devices. Always stored encrypted. |
| **Encrypted Payload** | An opaque binary blob containing telemetry data encrypted by the worker app. The server cannot read this data. |
| **Metadata** | Non-sensitive contextual information about a telemetry record (timestamps, category, worker ID). Stored in plaintext. |
| **Consent** | Explicit, informed agreement by a worker to have specific categories of telemetry data collected. Recorded and auditable. |
| **E2E Key** | End-to-end encryption key shared between an admin and their workers. Never transmitted to or stored on the server. |
| **Session** | An authenticated period represented by a JWT access token and refresh token pair. |
| **Batch** | A collection of telemetry records submitted in a single API request for efficiency. |
| **Work Hours** | The configured time window during which certain sensitive telemetry (e.g., GPS) may be collected. |

## Domain Entities

### Admin

The Admin is the primary user who supervises workers and views telemetry data.

**Properties:**
- `id` (UUID v7) — Unique identifier
- `email` (String) — Login email, unique
- `password_hash` (String) — Bcrypt-hashed password
- `name` (String) — Display name
- `organization` (String) — Company/organization name
- `is_active` (Boolean) — Account status
- `created_at` (Timestamp) — Registration timestamp
- `updated_at` (Timestamp) — Last profile update

**Business Rules:**
- An admin can manage many workers
- An admin can only view telemetry for workers assigned to them
- An admin holds the E2E decryption key (client-side)
- Deleting an admin should soft-delete (deactivate) all associated workers

### Worker

The Worker represents an employee whose device generates telemetry data.

**Properties:**
- `id` (UUID v7) — Unique identifier
- `admin_id` (UUID) — FK to owning admin
- `email` (String) — Worker email, unique
- `name` (String) — Display name
- `device_identifier` (String) — Unique device fingerprint
- `role_type` (Enum) — OFFICE | FIELD (determines GPS eligibility)
- `is_active` (Boolean) — Account status
- `consent_flags` (JSON) — Granular consent per telemetry category
- `work_hours_start` (Time) — Start of work window (for GPS)
- `work_hours_end` (Time) — End of work window (for GPS)
- `timezone` (String) — Worker's timezone (IANA format)
- `created_at` (Timestamp)
- `updated_at` (Timestamp)

**Business Rules:**
- A worker belongs to exactly ONE admin (simplification for initial version)
- A worker must provide explicit consent before data collection begins
- Consent is granular per telemetry category
- Workers with `role_type = OFFICE` cannot have GPS telemetry enabled
- Work hours define the window for GPS data collection

**Consent Flags Structure:**
```json
{
  "system_activity": true,
  "network_activity": true,
  "productivity_basic": true,
  "productivity_screenshots": false,
  "file_activity": true,
  "location": false
}
```

### Telemetry Record (Abstract Base)

All telemetry records share common metadata:

**Common Properties:**
- `id` (UUID v7) — Unique identifier
- `worker_id` (UUID) — FK to worker who generated this data
- `encrypted_payload` (Binary) — E2E encrypted telemetry data
- `payload_size` (Integer) — Size of encrypted payload in bytes
- `client_timestamp` (Timestamp) — When data was collected on device
- `server_timestamp` (Timestamp) — When server received the data
- `batch_id` (UUID, nullable) — Groups records from same batch submission
- `created_at` (Timestamp)

### SystemActivity

Telemetry about device system activity.

**Category:** `system_activity`

**Data collected (inside encrypted payload, server cannot see):**
- Idle time duration
- Active application name and duration
- Window titles
- CPU usage percentage
- RAM usage percentage
- Network bandwidth usage (up/down)
- USB device connection/disconnection events

### NetworkActivity

Telemetry about network and browsing activity.

**Category:** `network_activity`

**Data collected (inside encrypted payload):**
- Domain/URL visited
- Domain category (work, social_media, entertainment, news, other)
- Visit timestamp
- Duration on domain

### ProductivityMetrics

Telemetry about user input activity.

**Category:** `productivity`

**Data collected (inside encrypted payload):**
- Keystroke count per interval (NEVER content)
- Mouse click count per interval
- Screenshot image data (when consented, with visible notice)

**Special Rule:** Screenshot data requires `productivity_screenshots` consent flag to be true, separate from basic `productivity_basic` consent.

### FileActivity

Telemetry about file access on corporate directories.

**Category:** `file_activity`

**Data collected (inside encrypted payload):**
- File path (corporate directories only)
- Action type (read, write, delete, rename, move)
- Timestamp of action
- File size (optional)

### LocationData

GPS telemetry for field workers.

**Category:** `location`

**Data collected (inside encrypted payload):**
- Latitude
- Longitude
- Accuracy (meters)
- Altitude (optional)
- Speed (optional)

**Special Rules:**
- Only collected for workers with `role_type = FIELD`
- Only collected during configured `work_hours_start` to `work_hours_end`
- Requires explicit `location` consent flag
- Requires documented operational justification from admin

## Domain Events

These are significant business events that should be logged and potentially trigger side effects:

| Event | Trigger | Side Effects |
|-------|---------|--------------|
| `AdminRegistered` | New admin account created | Welcome email (future) |
| `WorkerRegistered` | New worker assigned to admin | Consent collection required |
| `ConsentUpdated` | Worker modifies consent flags | Audit log entry |
| `ConsentRevoked` | Worker revokes consent for a category | Stop collecting that category |
| `TelemetryIngested` | New telemetry batch received | Increment counters |
| `TelemetryAccessed` | Admin retrieves telemetry data | Audit log entry |
| `WorkerDeactivated` | Worker removed from monitoring | Stop all data collection |
| `DataDeletionRequested` | Worker requests data deletion | Schedule deletion job |

## Aggregate Boundaries

```
┌─────────────────────────────────┐
│ Admin Aggregate                 │
│                                 │
│  Admin (root)                   │
│    └── Workers[]                │
│          └── ConsentFlags       │
└─────────────────────────────────┘

┌─────────────────────────────────┐
│ Telemetry Aggregate             │
│                                 │
│  TelemetryRecord (root)         │
│    ├── SystemActivity           │
│    ├── NetworkActivity          │
│    ├── ProductivityMetrics      │
│    ├── FileActivity             │
│    └── LocationData             │
└─────────────────────────────────┘
```

## Invariants

1. **A worker MUST have exactly one admin** — No orphan workers, no multi-admin workers
2. **Telemetry data MUST reference a valid, active worker** — No telemetry for deactivated workers
3. **Consent flags MUST be recorded before telemetry collection** — No data without consent
4. **Location telemetry MUST only exist for FIELD workers** — Office workers cannot have location data
5. **Screenshots MUST have separate consent** — `productivity_screenshots` is independent from `productivity_basic`
6. **All telemetry payloads MUST be encrypted** — No plaintext telemetry in the database
7. **Work hours MUST be valid** — `work_hours_start` < `work_hours_end`
8. **Batch submissions MUST not exceed size limits** — Maximum 100 records per batch
