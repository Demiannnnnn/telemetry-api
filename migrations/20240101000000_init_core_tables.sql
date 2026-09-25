-- Migration: Init core tables (admins and workers)
-- Creates extensions, functions, enums, core tables, triggers, and privacy constraints.

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION trigger_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 1. Admins Table
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

-- Indexes for admins
CREATE UNIQUE INDEX idx_admins_email ON admins(email) WHERE deleted_at IS NULL;
CREATE INDEX idx_admins_organization ON admins(organization);
CREATE INDEX idx_admins_is_active ON admins(is_active) WHERE is_active = TRUE;

-- Trigger for auto-updating updated_at on admins
CREATE TRIGGER trigger_admins_updated_at
    BEFORE UPDATE ON admins
    FOR EACH ROW
    EXECUTE FUNCTION trigger_set_updated_at();

-- 2. Worker Role Enum
CREATE TYPE worker_role_type AS ENUM ('OFFICE', 'FIELD');

-- 3. Workers Table
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

-- Indexes for workers
CREATE UNIQUE INDEX idx_workers_email ON workers(email) WHERE deleted_at IS NULL;
CREATE INDEX idx_workers_admin_id ON workers(admin_id);
CREATE INDEX idx_workers_device_identifier ON workers(device_identifier);
CREATE INDEX idx_workers_role_type ON workers(role_type);
CREATE INDEX idx_workers_is_active ON workers(is_active) WHERE is_active = TRUE;

-- Constraints for workers
ALTER TABLE workers ADD CONSTRAINT chk_workers_work_hours
    CHECK (work_hours_start IS NULL OR work_hours_end IS NULL OR work_hours_start < work_hours_end);

ALTER TABLE workers ADD CONSTRAINT chk_workers_location_consent
    CHECK (
        NOT (consent_flags->>'location' = 'true' AND role_type = 'OFFICE')
    );

-- Trigger for auto-updating updated_at on workers
CREATE TRIGGER trigger_workers_updated_at
    BEFORE UPDATE ON workers
    FOR EACH ROW
    EXECUTE FUNCTION trigger_set_updated_at();
