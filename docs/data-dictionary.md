# Data Dictionary

## Overview

This document provides a comprehensive data dictionary for all entities, fields, and data types used in the Telemetry API. It serves as a single source of truth for understanding the data model.

---

## 1. Admins

**Table:** `admins`
**Description:** Administrator accounts that manage workers and view telemetry data.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | UUID | No | `gen_random_uuid()` | Primary key, UUID v7 (time-sortable) |
| `email` | VARCHAR(255) | No | - | Login email address, must be unique among active admins |
| `password_hash` | VARCHAR(255) | No | - | Bcrypt-hashed password (cost factor 12) |
| `name` | VARCHAR(255) | No | - | Display name of the administrator |
| `organization` | VARCHAR(255) | No | - | Company or organization name |
| `is_active` | BOOLEAN | No | `TRUE` | Account active status; `FALSE` means deactivated |
| `created_at` | TIMESTAMPTZ | No | `NOW()` | Timestamp of account creation (UTC) |
| `updated_at` | TIMESTAMPTZ | No | `NOW()` | Timestamp of last profile update (UTC), auto-updated via trigger |
| `deleted_at` | TIMESTAMPTZ | Yes | `NULL` | Soft delete timestamp; `NULL` means active |

**Constraints:**
- Unique email among non-deleted records (`WHERE deleted_at IS NULL`)

---

## 2. Workers

**Table:** `workers`
**Description:** Employee accounts linked to an admin. Their devices generate telemetry data.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | UUID | No | `gen_random_uuid()` | Primary key, UUID v7 |
| `admin_id` | UUID (FK) | No | - | References `admins.id`; the admin who manages this worker |
| `email` | VARCHAR(255) | No | - | Worker email, unique among active workers |
| `name` | VARCHAR(255) | No | - | Worker display name |
| `device_identifier` | VARCHAR(512) | No | - | Unique device fingerprint or serial number |
| `role_type` | `worker_role_type` ENUM | No | `'OFFICE'` | Worker role: `OFFICE` (desk/office) or `FIELD` (mobile/field) |
| `is_active` | BOOLEAN | No | `TRUE` | Account active status |
| `consent_flags` | JSONB | No | `{all: false}` | Granular consent per telemetry category (see below) |
| `work_hours_start` | TIME | Yes | `NULL` | Start of daily work window (local time); used for GPS boundary |
| `work_hours_end` | TIME | Yes | `NULL` | End of daily work window (local time); must be > start |
| `timezone` | VARCHAR(50) | No | `'America/Santiago'` | Worker's timezone in IANA format (e.g., `America/Santiago`) |
| `created_at` | TIMESTAMPTZ | No | `NOW()` | Account creation timestamp |
| `updated_at` | TIMESTAMPTZ | No | `NOW()` | Last update timestamp, auto-updated via trigger |
| `deleted_at` | TIMESTAMPTZ | Yes | `NULL` | Soft delete timestamp |

**Consent Flags JSONB Schema:**

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `system_activity` | boolean | `false` | Consent for system activity telemetry |
| `network_activity` | boolean | `false` | Consent for network/browsing telemetry |
| `productivity_basic` | boolean | `false` | Consent for keystroke/click frequency |
| `productivity_screenshots` | boolean | `false` | Consent for periodic screenshots (high sensitivity) |
| `file_activity` | boolean | `false` | Consent for file access logs |
| `location` | boolean | `false` | Consent for GPS tracking (FIELD role only) |

**Constraints:**
- `work_hours_start < work_hours_end` (when both are set)
- `location` consent cannot be `true` when `role_type = 'OFFICE'`
- Unique email among non-deleted records

---

## 3. Telemetry Tables (Common Schema)

All five telemetry tables share the same base schema. The only difference is the `subcategory` column, which exists only on `telemetry_productivity_metrics`.

### Common Columns

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | UUID | No | `gen_random_uuid()` | Primary key, UUID v7 |
| `worker_id` | UUID (FK) | No | - | References `workers.id`; the worker who generated this data |
| `encrypted_payload` | BYTEA | No | - | E2E encrypted telemetry data blob; server cannot decrypt |
| `payload_size` | INTEGER | No | - | Size of `encrypted_payload` in bytes |
| `client_timestamp` | TIMESTAMPTZ | No | - | When the telemetry data was collected on the worker device |
| `server_timestamp` | TIMESTAMPTZ | No | `NOW()` | When the server received and stored the data |
| `batch_id` | UUID | Yes | `NULL` | Groups records from the same batch submission; NULL if single |
| `created_at` | TIMESTAMPTZ | No | `NOW()` | Record creation timestamp |

### Tables

| Table Name | Category | Additional Columns | Description |
|-----------|----------|-------------------|-------------|
| `telemetry_system_activities` | System Activity | (none) | Idle time, apps, windows, CPU/RAM/net, USB |
| `telemetry_network_activities` | Network Activity | (none) | Domains visited, domain categories |
| `telemetry_productivity_metrics` | Productivity | `subcategory` ENUM | Keystroke/click counts, screenshots |
| `telemetry_file_activities` | File Activity | (none) | File access/modification logs |
| `telemetry_location_data` | Location | (none) | GPS coordinates during work hours |

### Productivity Subcategory ENUM

| Value | Description |
|-------|-------------|
| `BASIC` | Keystroke frequency and mouse click frequency metrics |
| `SCREENSHOT` | Periodic or event-triggered screen captures |

---

## 4. Refresh Tokens

**Table:** `refresh_tokens`
**Description:** JWT refresh tokens for token rotation authentication.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | UUID | No | `gen_random_uuid()` | Primary key |
| `user_id` | UUID | No | - | ID of the admin or worker who owns this token |
| `user_role` | `user_role` ENUM | No | - | `ADMIN` or `WORKER` |
| `token_hash` | VARCHAR(512) | No | - | SHA-256 hash of the refresh token (never store plaintext) |
| `expires_at` | TIMESTAMPTZ | No | - | Token expiration timestamp |
| `is_revoked` | BOOLEAN | No | `FALSE` | Whether this token has been revoked |
| `created_at` | TIMESTAMPTZ | No | `NOW()` | Token creation timestamp |
| `revoked_at` | TIMESTAMPTZ | Yes | `NULL` | When the token was revoked (if applicable) |

---

## 5. Audit Logs

**Table:** `audit_logs`
**Description:** Immutable audit trail for security and compliance monitoring.

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | UUID | No | `gen_random_uuid()` | Primary key |
| `actor_id` | UUID | No | - | ID of the user who performed the action |
| `actor_role` | `user_role` ENUM | No | - | Role of the actor: `ADMIN` or `WORKER` |
| `action` | VARCHAR(100) | No | - | Action performed (see action types below) |
| `resource_type` | VARCHAR(100) | No | - | Type of resource affected (see below) |
| `resource_id` | UUID | Yes | `NULL` | ID of the affected resource (if applicable) |
| `ip_address` | INET | Yes | `NULL` | Client IP address |
| `user_agent` | VARCHAR(512) | Yes | `NULL` | Client user agent string |
| `metadata` | JSONB | Yes | `NULL` | Additional context (varies by action) |
| `created_at` | TIMESTAMPTZ | No | `NOW()` | Timestamp of the action |

### Action Types

| Action | Description | Resource Type |
|--------|-------------|---------------|
| `AUTH_LOGIN` | User logged in | `admin` / `worker` |
| `AUTH_LOGIN_FAILED` | Failed login attempt | `admin` / `worker` |
| `AUTH_LOGOUT` | User logged out | `admin` / `worker` |
| `AUTH_TOKEN_REFRESH` | Token refreshed | `refresh_token` |
| `ADMIN_CREATED` | Admin account created | `admin` |
| `ADMIN_UPDATED` | Admin profile updated | `admin` |
| `ADMIN_DEACTIVATED` | Admin account deactivated | `admin` |
| `WORKER_CREATED` | Worker registered | `worker` |
| `WORKER_UPDATED` | Worker profile updated | `worker` |
| `WORKER_DEACTIVATED` | Worker deactivated | `worker` |
| `CONSENT_UPDATED` | Worker consent flags changed | `worker` |
| `TELEMETRY_INGESTED` | Telemetry batch received | `telemetry_batch` |
| `TELEMETRY_ACCESSED` | Admin retrieved telemetry | `telemetry_*` |
| `TELEMETRY_DELETED` | Telemetry record deleted | `telemetry_*` |
| `DATA_EXPORT_REQUESTED` | Worker requested data export | `worker` |
| `DATA_DELETION_REQUESTED` | Worker requested data deletion | `worker` |

---

## 6. Enum Types

### `worker_role_type`

| Value | Description |
|-------|-------------|
| `OFFICE` | Office/desk worker; NOT eligible for GPS tracking |
| `FIELD` | Field/mobile worker; eligible for GPS tracking with consent |

### `user_role`

| Value | Description |
|-------|-------------|
| `ADMIN` | Administrator role — manages workers, views telemetry |
| `WORKER` | Worker role — generates telemetry, manages own consent |

### `productivity_subcategory`

| Value | Description |
|-------|-------------|
| `BASIC` | Keystroke and mouse click frequency metrics |
| `SCREENSHOT` | Screen capture images |

---

## 7. Index Strategy

### Primary Indexes (Automatic)

All primary keys have automatic B-tree indexes.

### Query-Optimized Indexes

| Table | Index | Columns | Type | Purpose |
|-------|-------|---------|------|---------|
| `admins` | `idx_admins_email` | `email` (partial: deleted_at IS NULL) | Unique | Login lookup |
| `workers` | `idx_workers_admin_id` | `admin_id` | B-tree | List workers by admin |
| `workers` | `idx_workers_email` | `email` (partial: deleted_at IS NULL) | Unique | Login lookup |
| `telemetry_*` | `idx_t*_worker_id` | `worker_id` | B-tree | Filter by worker |
| `telemetry_*` | `idx_t*_client_timestamp` | `client_timestamp` | B-tree | Time-range queries |
| `telemetry_*` | `idx_t*_worker_client_ts` | `worker_id, client_timestamp DESC` | Composite | Worker + time range (most common query) |
| `telemetry_*` | `idx_t*_batch_id` | `batch_id` (partial: NOT NULL) | B-tree | Batch lookups |
| `refresh_tokens` | `idx_rt_token_hash` | `token_hash` | B-tree | Token validation |
| `audit_logs` | `idx_al_actor_id` | `actor_id` | B-tree | Actor history |
| `audit_logs` | `idx_al_created_at` | `created_at` | B-tree | Time-range queries |
