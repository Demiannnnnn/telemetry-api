-- Migration: Create telemetry storage tables
-- Stores encrypted binary blobs (Zero-Knowledge) with queryable metadata.

-- 1. System Activity Telemetry
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

CREATE INDEX idx_tsa_worker_id ON telemetry_system_activities(worker_id);
CREATE INDEX idx_tsa_client_timestamp ON telemetry_system_activities(client_timestamp);
CREATE INDEX idx_tsa_server_timestamp ON telemetry_system_activities(server_timestamp);
CREATE INDEX idx_tsa_batch_id ON telemetry_system_activities(batch_id) WHERE batch_id IS NOT NULL;
CREATE INDEX idx_tsa_worker_client_ts ON telemetry_system_activities(worker_id, client_timestamp DESC);

-- 2. Network Activity Telemetry
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

CREATE INDEX idx_tna_worker_id ON telemetry_network_activities(worker_id);
CREATE INDEX idx_tna_client_timestamp ON telemetry_network_activities(client_timestamp);
CREATE INDEX idx_tna_server_timestamp ON telemetry_network_activities(server_timestamp);
CREATE INDEX idx_tna_batch_id ON telemetry_network_activities(batch_id) WHERE batch_id IS NOT NULL;
CREATE INDEX idx_tna_worker_client_ts ON telemetry_network_activities(worker_id, client_timestamp DESC);

-- 3. Productivity Subcategory Enum and Telemetry Table
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

CREATE INDEX idx_tpm_worker_id ON telemetry_productivity_metrics(worker_id);
CREATE INDEX idx_tpm_subcategory ON telemetry_productivity_metrics(subcategory);
CREATE INDEX idx_tpm_client_timestamp ON telemetry_productivity_metrics(client_timestamp);
CREATE INDEX idx_tpm_server_timestamp ON telemetry_productivity_metrics(server_timestamp);
CREATE INDEX idx_tpm_batch_id ON telemetry_productivity_metrics(batch_id) WHERE batch_id IS NOT NULL;
CREATE INDEX idx_tpm_worker_client_ts ON telemetry_productivity_metrics(worker_id, client_timestamp DESC);
CREATE INDEX idx_tpm_worker_subcategory ON telemetry_productivity_metrics(worker_id, subcategory);

-- 4. File Activity Telemetry
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

CREATE INDEX idx_tfa_worker_id ON telemetry_file_activities(worker_id);
CREATE INDEX idx_tfa_client_timestamp ON telemetry_file_activities(client_timestamp);
CREATE INDEX idx_tfa_server_timestamp ON telemetry_file_activities(server_timestamp);
CREATE INDEX idx_tfa_batch_id ON telemetry_file_activities(batch_id) WHERE batch_id IS NOT NULL;
CREATE INDEX idx_tfa_worker_client_ts ON telemetry_file_activities(worker_id, client_timestamp DESC);

-- 5. Location Telemetry (Field workers only)
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

CREATE INDEX idx_tld_worker_id ON telemetry_location_data(worker_id);
CREATE INDEX idx_tld_client_timestamp ON telemetry_location_data(client_timestamp);
CREATE INDEX idx_tld_server_timestamp ON telemetry_location_data(server_timestamp);
CREATE INDEX idx_tld_batch_id ON telemetry_location_data(batch_id) WHERE batch_id IS NOT NULL;
CREATE INDEX idx_tld_worker_client_ts ON telemetry_location_data(worker_id, client_timestamp DESC);
