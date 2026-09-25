-- Migration: Create auth (refresh tokens) and immutable audit log tables

-- 1. User Role Enum
CREATE TYPE user_role AS ENUM ('ADMIN', 'WORKER');

-- 2. Refresh Tokens Table (Rotation & Token Family Security)
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

CREATE INDEX idx_rt_user_id ON refresh_tokens(user_id);
CREATE INDEX idx_rt_token_hash ON refresh_tokens(token_hash);
CREATE INDEX idx_rt_expires_at ON refresh_tokens(expires_at);
CREATE INDEX idx_rt_is_revoked ON refresh_tokens(is_revoked) WHERE is_revoked = FALSE;

-- 3. Audit Logs Table (Immutable Audit Trail)
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

CREATE INDEX idx_al_actor_id ON audit_logs(actor_id);
CREATE INDEX idx_al_action ON audit_logs(action);
CREATE INDEX idx_al_resource_type ON audit_logs(resource_type);
CREATE INDEX idx_al_resource_id ON audit_logs(resource_id);
CREATE INDEX idx_al_created_at ON audit_logs(created_at);
CREATE INDEX idx_al_actor_action ON audit_logs(actor_id, action);
