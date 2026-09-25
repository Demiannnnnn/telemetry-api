# Security Model

## Overview

The Telemetry API implements a **zero-knowledge security architecture** where the server acts as an encrypted relay and storage layer. The server never possesses the ability to decrypt telemetry data. This document describes the complete security model including encryption, authentication, authorization, and operational security.

## End-to-End Encryption (E2E)

### Architecture

```
┌──────────────┐                                          ┌──────────────┐
│  Worker App  │                                          │  Admin App   │
│              │                                          │              │
│ ┌──────────┐ │          ┌──────────────────┐           │ ┌──────────┐ │
│ │Encryption│ │          │  Telemetry API   │           │ │Decryption│ │
│ │  Key 🔑  │ │          │                  │           │ │  Key 🔑  │ │
│ └──────────┘ │          │  ┌────────────┐  │           │ └──────────┘ │
│              │          │  │ Encrypted  │  │           │              │
│  Plaintext   │  E2E     │  │  Storage   │  │  E2E      │  Plaintext   │
│  Telemetry ──┼─────────▶│  │ (BYTEA)    │──┼──────────▶│  Telemetry   │
│  Data        │ Encrypted│  │            │  │ Encrypted │  Data         │
│              │ Payload  │  └────────────┘  │ Payload   │              │
└──────────────┘          │                  │           └──────────────┘
                          │  ❌ No keys     │
                          │  ❌ No decrypt  │
                          │  ❌ No plaintext│
                          └──────────────────┘
```

### Key Management

| Aspect | Detail |
|--------|--------|
| **Key Generation** | Generated on the admin device during worker onboarding |
| **Key Distribution** | Shared between admin and worker apps via out-of-band channel (QR code, secure link) |
| **Key Storage** | Stored locally on each device's secure enclave / keychain |
| **Key Rotation** | Managed entirely client-side; server is unaware of key changes |
| **Server Knowledge** | Zero — the server never sees, stores, or processes encryption keys |

### What the Server Sees

| Data Point | Server Visibility |
|-----------|-------------------|
| Who sent the data (worker_id) | ✅ Visible |
| When it was collected (timestamp) | ✅ Visible |
| What category it belongs to | ✅ Visible |
| Size of the payload | ✅ Visible |
| Actual telemetry content | ❌ Encrypted, not visible |
| Encryption keys | ❌ Never stored or transmitted |

### Encryption Requirements (Client-Side)

The following requirements apply to the client applications (Worker and Admin apps). The server does not implement or enforce encryption — it simply stores whatever encrypted blob it receives.

- **Algorithm:** AES-256-GCM (recommended) or ChaCha20-Poly1305
- **Key Derivation:** HKDF-SHA256 from shared secret
- **Nonce:** Unique per-message, never reused
- **Authenticated Encryption:** Authentication tag included in payload

## Authentication

### JWT Token Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    Authentication Flow                        │
│                                                              │
│  1. Login Request                                            │
│     POST /auth/login { email, password, role }               │
│     ──────────────────────────────────────────▶               │
│                                                              │
│  2. Server validates credentials                             │
│     - bcrypt password verification                           │
│     - Account status check                                   │
│                                                              │
│  3. Token Generation                                         │
│     ◀──────────────────────────────────────────               │
│     {                                                        │
│       access_token:  JWT (short-lived, 24h)                  │
│       refresh_token: JWT (long-lived, 7d, single-use)        │
│     }                                                        │
│                                                              │
│  4. Authenticated Requests                                   │
│     Authorization: Bearer <access_token>                     │
│     ──────────────────────────────────────────▶               │
│                                                              │
│  5. Token Refresh (when access_token expires)                │
│     POST /auth/refresh { refresh_token }                     │
│     ──────────────────────────────────────────▶               │
│                                                              │
│     New tokens issued, old refresh_token invalidated         │
│     ◀──────────────────────────────────────────               │
└──────────────────────────────────────────────────────────────┘
```

### JWT Claims

**Access Token:**
```json
{
  "sub": "01912345-6789-7abc-def0-123456789abc",
  "role": "ADMIN",
  "email": "admin@company.com",
  "iat": 1705312200,
  "exp": 1705398600,
  "iss": "telemetry-api"
}
```

### Token Security Measures

| Measure | Implementation |
|---------|----------------|
| **Short-lived access tokens** | 24-hour maximum lifetime |
| **Refresh token rotation** | Each refresh token is single-use; a new one is issued with each refresh |
| **Refresh token revocation** | Tokens are hashed and stored in `refresh_tokens` table |
| **Token family detection** | Reuse of a revoked refresh token invalidates the entire token family |
| **Secure storage** | Refresh tokens stored as SHA-256 hashes, never plaintext |
| **Algorithm** | HS256 or RS256 (configurable) |

### Password Security

| Measure | Implementation |
|---------|----------------|
| **Hashing** | bcrypt with cost factor 12 |
| **Minimum length** | 8 characters |
| **Complexity** | At least one uppercase, one lowercase, one digit, one special character |
| **No plaintext storage** | Passwords are never stored or logged in plaintext |

## Authorization

### Role-Based Access Control (RBAC)

```
┌────────────────────────────────────────────────────────────┐
│                   Authorization Matrix                      │
├──────────────────────┬──────────┬───────────┬──────────────┤
│ Resource/Action      │  Admin   │  Worker   │  No Auth     │
├──────────────────────┼──────────┼───────────┼──────────────┤
│ POST /auth/register  │    -     │    -      │    ✅        │
│ POST /auth/login     │    -     │    -      │    ✅        │
│ POST /auth/refresh   │    -     │    -      │    ✅ *      │
│ GET /health          │    -     │    -      │    ✅        │
├──────────────────────┼──────────┼───────────┼──────────────┤
│ GET /admins/me       │    ✅    │    ❌     │    ❌        │
│ PUT /admins/me       │    ✅    │    ❌     │    ❌        │
├──────────────────────┼──────────┼───────────┼──────────────┤
│ POST /workers        │    ✅    │    ❌     │    ❌        │
│ GET /workers         │    ✅    │    ❌     │    ❌        │
│ GET /workers/:id     │    ✅ ** │ ✅ own    │    ❌        │
│ PUT /workers/:id     │    ✅ ** │    ❌     │    ❌        │
│ PATCH /workers/consent│   ❌    │ ✅ own    │    ❌        │
│ DELETE /workers/:id  │    ✅ ** │    ❌     │    ❌        │
├──────────────────────┼──────────┼───────────┼──────────────┤
│ POST /telemetry/*    │    ❌    │ ✅ own    │    ❌        │
│ GET /telemetry/*     │    ✅ ** │    ❌     │    ❌        │
│ DELETE /telemetry/*  │    ✅ ** │    ❌     │    ❌        │
├──────────────────────┼──────────┼───────────┼──────────────┤
│ POST /data-request   │    ❌    │ ✅ own    │    ❌        │
│ POST /data-deletion  │    ❌    │ ✅ own    │    ❌        │
└──────────────────────┴──────────┴───────────┴──────────────┘

 * Requires valid refresh token
** Only for workers owned by the admin
```

### Ownership Verification

Every request that accesses worker data includes an ownership check:

```
1. Extract admin_id from JWT claims
2. Load worker by ID
3. Verify worker.admin_id == admin_id from JWT
4. If mismatch → 403 Forbidden
```

This ensures that even if an admin knows another worker's UUID, they cannot access data for workers they don't own.

## Transport Security

| Measure | Implementation |
|---------|----------------|
| **TLS Version** | TLS 1.3 minimum (1.2 accepted with strong ciphers) |
| **HSTS** | `Strict-Transport-Security: max-age=31536000; includeSubDomains` |
| **Certificate** | Valid, trusted CA certificate |
| **HTTPS Only** | HTTP requests redirected to HTTPS |

## Input Validation & Sanitization

| Check | Applied To |
|-------|-----------|
| **Schema validation** | All request bodies (serde + validator) |
| **Size limits** | Request body (10 MB max), batch size (100 records max) |
| **Type checking** | UUID format, timestamp format, enum values |
| **SQL injection prevention** | Parameterized queries via SQLx macros |
| **Content-Type enforcement** | `application/json` required |

## Rate Limiting

Rate limiting prevents abuse and DoS attacks:

| Endpoint | Limit | Window |
|----------|-------|--------|
| Login/Register | 5 requests | per minute per IP |
| Telemetry ingestion | 10 requests | per minute per worker |
| General API | 100 requests | per minute per user |
| Data subject requests | 5 requests | per day per worker |

## Audit Logging

All security-relevant events are logged to the `audit_logs` table:

| Event | Logged Data |
|-------|------------|
| Login attempt (success/failure) | user_id, IP, user_agent, result |
| Token refresh | user_id, IP |
| Worker registration | admin_id, worker_id |
| Consent change | worker_id, old_flags, new_flags |
| Telemetry access | admin_id, worker_id, category, time_range |
| Data deletion request | worker_id, categories, reason |
| Account deactivation | actor_id, target_id |

### Audit Log Immutability

- Audit logs are **append-only** — no UPDATE or DELETE operations allowed
- Logs are retained for a minimum of 2 years (legal requirement)
- Logs contain no encrypted payload content

## Security Headers

The API returns the following security headers:

```
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 0
Content-Security-Policy: default-src 'none'
Cache-Control: no-store, no-cache, must-revalidate
Pragma: no-cache
Referrer-Policy: no-referrer
```

## Threat Model

### Threats and Mitigations

| Threat | Risk | Mitigation |
|--------|------|------------|
| Server compromise | High | E2E encryption — compromised server yields only encrypted blobs |
| Database breach | High | All telemetry data encrypted; passwords bcrypt-hashed |
| JWT theft | Medium | Short-lived tokens, refresh rotation, token family invalidation |
| Brute force login | Medium | Rate limiting, account lockout (future), bcrypt cost factor |
| SQL injection | Medium | Compile-time verified parameterized queries (SQLx) |
| Unauthorized data access | High | Ownership verification on every request |
| Insider threat (server operator) | High | Zero-knowledge architecture — operator cannot decrypt data |
| Man-in-the-middle | High | TLS 1.3, HSTS, certificate pinning (client-side) |
| Replay attack | Medium | Nonce in encryption, timestamp validation, token expiry |

## Incident Response

In case of a security incident:

1. **Detection** — Audit logs and monitoring alerts
2. **Containment** — Revoke all active tokens, disable affected accounts
3. **Assessment** — Determine scope of compromise
4. **Notification** — Notify affected admins and workers within 72 hours (per Chilean law)
5. **Remediation** — Patch vulnerability, rotate server secrets
6. **Documentation** — Record incident details and response actions
