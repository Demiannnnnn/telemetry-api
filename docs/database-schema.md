# Database Schema Design

## Overview

This document defines the complete PostgreSQL database schema for the Telemetry API. All telemetry data is stored as encrypted binary payloads — the server never has access to the plaintext content. Only metadata (timestamps, identifiers, categories) is stored in plaintext to enable querying, filtering, and pagination.

## Entity Relationship Diagram

```
┌──────────────────────┐       ┌──────────────────────┐
│       admins          │       │      workers          │
├──────────────────────┤       ├──────────────────────┤
│ id          UUID PK  │       │ id          UUID PK  │
│ email       VARCHAR  │◄──────│ admin_id    UUID FK  │
│ password_hash VARCHAR│  1:N  │ email       VARCHAR  │
│ name        VARCHAR  │       │ name        VARCHAR  │
│ organization VARCHAR │       │ device_id   VARCHAR  │
│ is_active   BOOLEAN  │       │ role_type   ENUM     │
│ created_at  TIMESTAMPTZ│     │ is_active   BOOLEAN  │
│ updated_at  TIMESTAMPTZ│     │ consent_flags JSONB  │
│ deleted_at  TIMESTAMPTZ│     │ work_hours_start TIME│
└──────────────────────┘       │ work_hours_end   TIME│
                                │ timezone    VARCHAR  │
                                │ created_at  TIMESTAMPTZ│
                                │ updated_at  TIMESTAMPTZ│
                                │ deleted_at  TIMESTAMPTZ│
                                └───────┬──────────────┘
                                        │
                    ┌───────────────────┼───────────────────────┐
                    │                   │                       │
                    ▼                   ▼                       ▼
    ┌───────────────────────┐ ┌──────────────────────┐ ┌──────────────────────┐
    │telemetry_system       │ │telemetry_network     │ │telemetry_productivity│
    │_activities            │ │_activities            │ │_metrics              │
    ├───────────────────────┤ ├──────────────────────┤ ├──────────────────────┤
    │ id         UUID PK   │ │ id         UUID PK   │ │ id         UUID PK   │
    │ worker_id  UUID FK   │ │ worker_id  UUID FK   │ │ worker_id  UUID FK   │
    │ encrypted_ BYTEA     │ │ encrypted_ BYTEA     │ │ encrypted_ BYTEA     │
    │  payload             │ │  payload             │ │  payload             │
    │ payload_   INT       │ │ payload_   INT       │ │ payload_   INT       │
    │  size                │ │  size                │ │  size                │
    │ client_    TIMESTAMPTZ│ │ client_    TIMESTAMPTZ│ │ subcategory VARCHAR  │
    │  timestamp           │ │  timestamp           │ │ client_    TIMESTAMPTZ│
    │ server_    TIMESTAMPTZ│ │ server_    TIMESTAMPTZ│ │  timestamp           │
    │  timestamp           │ │  timestamp           │ │ server_    TIMESTAMPTZ│
    │ batch_id   UUID      │ │ batch_id   UUID      │ │  timestamp           │
    │ created_at TIMESTAMPTZ│ │ created_at TIMESTAMPTZ│ │ batch_id   UUID      │
    └───────────────────────┘ └──────────────────────┘ │ created_at TIMESTAMPTZ│
                                                        └──────────────────────┘
    ┌───────────────────────┐ ┌──────────────────────┐
    │telemetry_file         │ │telemetry_location    │
    │_activities            │ │_data                 │
    ├───────────────────────┤ ├──────────────────────┤
    │ id         UUID PK   │ │ id         UUID PK   │
    │ worker_id  UUID FK   │ │ worker_id  UUID FK   │
    │ encrypted_ BYTEA     │ │ encrypted_ BYTEA     │
    │  payload             │ │  payload             │
    │ payload_   INT       │ │ payload_   INT       │
    │  size                │ │  size                │
    │ client_    TIMESTAMPTZ│ │ client_    TIMESTAMPTZ│
    │  timestamp           │ │  timestamp           │
    │ server_    TIMESTAMPTZ│ │ server_    TIMESTAMPTZ│
    │  timestamp           │ │  timestamp           │
    │ batch_id   UUID      │ │ batch_id   UUID      │
    │ created_at TIMESTAMPTZ│ │ created_at TIMESTAMPTZ│
    └───────────────────────┘ └──────────────────────┘

    ┌───────────────────────┐ ┌──────────────────────┐
    │  refresh_tokens       │ │  audit_logs          │
    ├───────────────────────┤ ├──────────────────────┤
    │ id         UUID PK   │ │ id         UUID PK   │
    │ user_id    UUID      │ │ actor_id   UUID      │
    │ user_role  VARCHAR   │ │ actor_role VARCHAR    │
    │ token_hash VARCHAR   │ │ action     VARCHAR    │
    │ expires_at TIMESTAMPTZ│ │ resource   VARCHAR    │
    │ is_revoked BOOLEAN   │ │ resource_id UUID      │
    │ created_at TIMESTAMPTZ│ │ ip_address INET      │
    │ revoked_at TIMESTAMPTZ│ │ user_agent VARCHAR    │
    └───────────────────────┘ │ metadata   JSONB     │
                               │ created_at TIMESTAMPTZ│
                               └──────────────────────┘
```

## Table Definitions

### 1. `admins`

Stores administrator accounts.

```sql
CREATE TABLE admins (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           VARCHAR(255) NOT NULL UNIQUE,
    password_hash   VARCHAR(255) NOT NULL,
    name            VARCHAR(255) NOT NULL,
    organization    VARCHAR(255) NOT NULL,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at      TIMESTAMPTZ
);

-- Indexes
CREATE UNIQUE INDEX idx_admins_email ON admins(email) WHERE deleted_at IS NULL;
CREATE INDEX idx_admins_organization ON admins(organization);
CREATE INDEX idx_admins_is_active ON admins(is_active) WHERE is_active = TRUE;
```

### 2. `workers`

Stores worker accounts linked to admins.

```sql
-- Custom enum type for worker role
CREATE TYPE worker_role_type AS ENUM ('OFFICE', 'FIELD');

CREATE TABLE workers (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    admin_id            UUID NOT NULL REFERENCES admins(id) ON DELETE CASCADE,
    email               VARCHAR(255) NOT NULL UNIQUE,
    name                VARCHAR(255) NOT NULL,
    device_identifier   VARCHAR(512) NOT NULL,
    role_type           worker_role_type NOT NULL DEFAULT 'OFFICE',
    is_active           BOOLEAN NOT NULL DEFAULT TRUE,
    consent_flags       JSONB NOT NULL DEFAULT '{
        "system_activity": false,
        "network_activity": false,
        "productivity_basic": false,
        "productivity_screenshots": false,
        "file_activity": false,
        "location": false
    }'::jsonb,
    work_hours_start    TIME,
    work_hours_end      TIME,
    timezone            VARCHAR(50) NOT NULL DEFAULT 'America/Santiago',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at          TIMESTAMPTZ
);

-- Indexes
CREATE UNIQUE INDEX idx_workers_email ON workers(email) WHERE deleted_at IS NULL;
CREATE INDEX idx_workers_admin_id ON workers(admin_id);
CREATE INDEX idx_workers_device_identifier ON workers(device_identifier);
CREATE INDEX idx_workers_role_type ON workers(role_type);
CREATE INDEX idx_workers_is_active ON workers(is_active) WHERE is_active = TRUE;

-- Constraints
ALTER TABLE workers ADD CONSTRAINT chk_workers_work_hours
    CHECK (work_hours_start IS NULL OR work_hours_end IS NULL OR work_hours_start < work_hours_end);

ALTER TABLE workers ADD CONSTRAINT chk_workers_location_consent
    CHECK (
        NOT (consent_flags->>'location' = 'true' AND role_type = 'OFFICE')
    );
```

### 3. `telemetry_system_activities`

Stores encrypted system activity telemetry (idle time, apps, windows, CPU/RAM/network, USB events).

```sql
CREATE TABLE telemetry_system_activities (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id           UUID NOT NULL REFERENCES workers(id) ON DELETE CASCADE,
    encrypted_payload   BYTEA NOT NULL,
    payload_size        INTEGER NOT NULL,
    client_timestamp    TIMESTAMPTZ NOT NULL,
    server_timestamp    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    batch_id            UUID,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_tsa_worker_id ON telemetry_system_activities(worker_id);
CREATE INDEX idx_tsa_client_timestamp ON telemetry_system_activities(client_timestamp);
CREATE INDEX idx_tsa_server_timestamp ON telemetry_system_activities(server_timestamp);
CREATE INDEX idx_tsa_batch_id ON telemetry_system_activities(batch_id) WHERE batch_id IS NOT NULL;
CREATE INDEX idx_tsa_worker_client_ts ON telemetry_system_activities(worker_id, client_timestamp DESC);
```

### 4. `telemetry_network_activities`

Stores encrypted network/browsing activity telemetry (domains, categories).

```sql
CREATE TABLE telemetry_network_activities (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id           UUID NOT NULL REFERENCES workers(id) ON DELETE CASCADE,
    encrypted_payload   BYTEA NOT NULL,
    payload_size        INTEGER NOT NULL,
    client_timestamp    TIMESTAMPTZ NOT NULL,
    server_timestamp    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    batch_id            UUID,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_tna_worker_id ON telemetry_network_activities(worker_id);
CREATE INDEX idx_tna_client_timestamp ON telemetry_network_activities(client_timestamp);
CREATE INDEX idx_tna_server_timestamp ON telemetry_network_activities(server_timestamp);
CREATE INDEX idx_tna_batch_id ON telemetry_network_activities(batch_id) WHERE batch_id IS NOT NULL;
CREATE INDEX idx_tna_worker_client_ts ON telemetry_network_activities(worker_id, client_timestamp DESC);
```

### 5. `telemetry_productivity_metrics`

Stores encrypted productivity telemetry (keystroke frequency, click frequency, screenshots).

```sql
-- Subcategory enum for productivity metrics
CREATE TYPE productivity_subcategory AS ENUM ('BASIC', 'SCREENSHOT');

CREATE TABLE telemetry_productivity_metrics (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id           UUID NOT NULL REFERENCES workers(id) ON DELETE CASCADE,
    encrypted_payload   BYTEA NOT NULL,
    payload_size        INTEGER NOT NULL,
    subcategory         productivity_subcategory NOT NULL DEFAULT 'BASIC',
    client_timestamp    TIMESTAMPTZ NOT NULL,
    server_timestamp    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    batch_id            UUID,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_tpm_worker_id ON telemetry_productivity_metrics(worker_id);
CREATE INDEX idx_tpm_subcategory ON telemetry_productivity_metrics(subcategory);
CREATE INDEX idx_tpm_client_timestamp ON telemetry_productivity_metrics(client_timestamp);
CREATE INDEX idx_tpm_server_timestamp ON telemetry_productivity_metrics(server_timestamp);
CREATE INDEX idx_tpm_batch_id ON telemetry_productivity_metrics(batch_id) WHERE batch_id IS NOT NULL;
CREATE INDEX idx_tpm_worker_client_ts ON telemetry_productivity_metrics(worker_id, client_timestamp DESC);
CREATE INDEX idx_tpm_worker_subcategory ON telemetry_productivity_metrics(worker_id, subcategory);
```

### 6. `telemetry_file_activities`

Stores encrypted file access/modification telemetry.

```sql
CREATE TABLE telemetry_file_activities (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id           UUID NOT NULL REFERENCES workers(id) ON DELETE CASCADE,
    encrypted_payload   BYTEA NOT NULL,
    payload_size        INTEGER NOT NULL,
    client_timestamp    TIMESTAMPTZ NOT NULL,
    server_timestamp    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    batch_id            UUID,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_tfa_worker_id ON telemetry_file_activities(worker_id);
CREATE INDEX idx_tfa_client_timestamp ON telemetry_file_activities(client_timestamp);
CREATE INDEX idx_tfa_server_timestamp ON telemetry_file_activities(server_timestamp);
CREATE INDEX idx_tfa_batch_id ON telemetry_file_activities(batch_id) WHERE batch_id IS NOT NULL;
CREATE INDEX idx_tfa_worker_client_ts ON telemetry_file_activities(worker_id, client_timestamp DESC);
```

### 7. `telemetry_location_data`

Stores encrypted GPS/location telemetry.

```sql
CREATE TABLE telemetry_location_data (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    worker_id           UUID NOT NULL REFERENCES workers(id) ON DELETE CASCADE,
    encrypted_payload   BYTEA NOT NULL,
    payload_size        INTEGER NOT NULL,
    client_timestamp    TIMESTAMPTZ NOT NULL,
    server_timestamp    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    batch_id            UUID,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_tld_worker_id ON telemetry_location_data(worker_id);
CREATE INDEX idx_tld_client_timestamp ON telemetry_location_data(client_timestamp);
CREATE INDEX idx_tld_server_timestamp ON telemetry_location_data(server_timestamp);
CREATE INDEX idx_tld_batch_id ON telemetry_location_data(batch_id) WHERE batch_id IS NOT NULL;
CREATE INDEX idx_tld_worker_client_ts ON telemetry_location_data(worker_id, client_timestamp DESC);
```

### 8. `refresh_tokens`

Stores JWT refresh tokens for token rotation.

```sql
CREATE TYPE user_role AS ENUM ('ADMIN', 'WORKER');

CREATE TABLE refresh_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL,
    user_role   user_role NOT NULL,
    token_hash  VARCHAR(512) NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    is_revoked  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at  TIMESTAMPTZ
);

-- Indexes
CREATE INDEX idx_rt_user_id ON refresh_tokens(user_id);
CREATE INDEX idx_rt_token_hash ON refresh_tokens(token_hash);
CREATE INDEX idx_rt_expires_at ON refresh_tokens(expires_at);
CREATE INDEX idx_rt_is_revoked ON refresh_tokens(is_revoked) WHERE is_revoked = FALSE;
```

### 9. `audit_logs`

Immutable audit trail for compliance and security monitoring.

```sql
CREATE TABLE audit_logs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_id        UUID NOT NULL,
    actor_role      user_role NOT NULL,
    action          VARCHAR(100) NOT NULL,
    resource_type   VARCHAR(100) NOT NULL,
    resource_id     UUID,
    ip_address      INET,
    user_agent      VARCHAR(512),
    metadata        JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_al_actor_id ON audit_logs(actor_id);
CREATE INDEX idx_al_action ON audit_logs(action);
CREATE INDEX idx_al_resource_type ON audit_logs(resource_type);
CREATE INDEX idx_al_resource_id ON audit_logs(resource_id);
CREATE INDEX idx_al_created_at ON audit_logs(created_at);
CREATE INDEX idx_al_actor_action ON audit_logs(actor_id, action);
```

## Triggers

### Auto-update `updated_at` Trigger

```sql
-- Function to auto-update updated_at
CREATE OR REPLACE FUNCTION trigger_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply to admins
CREATE TRIGGER set_updated_at_admins
    BEFORE UPDATE ON admins
    FOR EACH ROW
    EXECUTE FUNCTION trigger_set_updated_at();

-- Apply to workers
CREATE TRIGGER set_updated_at_workers
    BEFORE UPDATE ON workers
    FOR EACH ROW
    EXECUTE FUNCTION trigger_set_updated_at();
```

## Partitioning Strategy (Future)

For high-volume telemetry tables, consider range partitioning by `created_at`:

```sql
-- Example: Partition telemetry_system_activities by month
CREATE TABLE telemetry_system_activities (
    -- ... same columns ...
) PARTITION BY RANGE (created_at);

CREATE TABLE telemetry_system_activities_2024_01
    PARTITION OF telemetry_system_activities
    FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');
```

## Data Retention Policy

| Table | Retention | Action |
|-------|-----------|--------|
| `admins` | Indefinite | Soft delete only |
| `workers` | Indefinite | Soft delete only |
| `telemetry_*` | Configurable (default 90 days) | Hard delete after retention + 30 day grace |
| `refresh_tokens` | 30 days after expiry | Hard delete |
| `audit_logs` | 2 years (legal requirement) | Archive to cold storage |

## Encrypted Payload Structure (Client-Side Reference)

> **Note:** The following is for documentation purposes only. The server never sees this structure — it's encrypted before transmission.

The encrypted payload for each telemetry category contains:

### System Activity Payload
```json
{
  "idle_seconds": 120,
  "active_app": "Visual Studio Code",
  "active_app_duration_seconds": 3600,
  "window_title": "main.rs — telemetry-api",
  "cpu_usage_percent": 45.2,
  "ram_usage_percent": 67.8,
  "network_bytes_sent": 1024000,
  "network_bytes_received": 5120000,
  "usb_events": [
    {"device": "USB Flash Drive", "action": "connected", "timestamp": "..."}
  ]
}
```

### Network Activity Payload
```json
{
  "domain": "github.com",
  "full_url": "https://github.com/user/repo",
  "category": "WORK",
  "visit_timestamp": "...",
  "duration_seconds": 300
}
```

### Productivity Metrics Payload (Basic)
```json
{
  "interval_start": "...",
  "interval_end": "...",
  "keystroke_count": 1523,
  "mouse_click_count": 342,
  "active_seconds": 3540
}
```

### Productivity Metrics Payload (Screenshot)
```json
{
  "capture_timestamp": "...",
  "image_format": "PNG",
  "image_data_base64": "...",
  "resolution": {"width": 1920, "height": 1080},
  "is_blurred": false
}
```

### File Activity Payload
```json
{
  "file_path": "/corporate/documents/report.xlsx",
  "action": "WRITE",
  "timestamp": "...",
  "file_size_bytes": 102400
}
```

### Location Data Payload
```json
{
  "latitude": -33.4489,
  "longitude": -70.6693,
  "accuracy_meters": 10.5,
  "altitude_meters": 520.0,
  "speed_kmh": 0.0,
  "timestamp": "..."
}
```
