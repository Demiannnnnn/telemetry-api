# API Specification

## Base URL

```
https://api.example.com/api/v1
```

## Authentication

All endpoints (except where noted) require a valid JWT token in the `Authorization` header:

```
Authorization: Bearer <jwt_token>
```

## Common Response Format

### Success (Single Resource)
```json
{
  "data": { ... }
}
```

### Success (Collection)
```json
{
  "data": [ ... ],
  "pagination": {
    "total": 150,
    "page": 1,
    "per_page": 20,
    "total_pages": 8
  }
}
```

### Error
```json
{
  "error": {
    "code": "RESOURCE_NOT_FOUND",
    "message": "Worker with the specified ID was not found",
    "details": null
  }
}
```

---

## 1. Health Check

### `GET /health`

> **Authentication:** Not required

Check API service health.

**Response:** `200 OK`
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

---

## 2. Authentication

### `POST /auth/register`

> **Authentication:** Not required

Register a new admin account.

**Request Body:**
```json
{
  "email": "admin@company.com",
  "password": "SecureP@ssw0rd!",
  "name": "John Doe",
  "organization": "Acme Corp"
}
```

**Response:** `201 Created`
```json
{
  "data": {
    "id": "01912345-6789-7abc-def0-123456789abc",
    "email": "admin@company.com",
    "name": "John Doe",
    "organization": "Acme Corp",
    "access_token": "eyJ...",
    "refresh_token": "eyJ...",
    "created_at": "2024-01-15T10:30:00Z"
  }
}
```

**Errors:**
| Code | Status | Condition |
|------|--------|-----------|
| `VALIDATION_ERROR` | 400 | Invalid email format, weak password |
| `CONFLICT` | 409 | Email already registered |

---

### `POST /auth/login`

> **Authentication:** Not required

Authenticate and receive tokens.

**Request Body:**
```json
{
  "email": "admin@company.com",
  "password": "SecureP@ssw0rd!",
  "role": "ADMIN"
}
```

**Response:** `200 OK`
```json
{
  "data": {
    "access_token": "eyJ...",
    "refresh_token": "eyJ...",
    "token_type": "Bearer",
    "expires_in": 86400,
    "user": {
      "id": "01912345-6789-7abc-def0-123456789abc",
      "email": "admin@company.com",
      "name": "John Doe",
      "role": "ADMIN"
    }
  }
}
```

**Errors:**
| Code | Status | Condition |
|------|--------|-----------|
| `INVALID_CREDENTIALS` | 401 | Wrong email or password |
| `VALIDATION_ERROR` | 400 | Missing required fields |

---

### `POST /auth/refresh`

> **Authentication:** Not required (uses refresh token)

Refresh access token using a valid refresh token. Implements **token rotation** — each refresh token can only be used once.

**Request Body:**
```json
{
  "refresh_token": "eyJ..."
}
```

**Response:** `200 OK`
```json
{
  "data": {
    "access_token": "eyJ...",
    "refresh_token": "eyJ...",
    "token_type": "Bearer",
    "expires_in": 86400
  }
}
```

**Errors:**
| Code | Status | Condition |
|------|--------|-----------|
| `TOKEN_EXPIRED` | 401 | Refresh token expired |
| `UNAUTHORIZED` | 401 | Invalid or revoked refresh token |

---

### `POST /auth/logout`

> **Authentication:** Required

Revoke the current refresh token.

**Request Body:**
```json
{
  "refresh_token": "eyJ..."
}
```

**Response:** `204 No Content`

---

## 3. Admin Management

### `GET /admins/me`

> **Authentication:** Required (Admin)

Get the authenticated admin's profile.

**Response:** `200 OK`
```json
{
  "data": {
    "id": "01912345-6789-7abc-def0-123456789abc",
    "email": "admin@company.com",
    "name": "John Doe",
    "organization": "Acme Corp",
    "is_active": true,
    "worker_count": 15,
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T10:30:00Z"
  }
}
```

---

### `PUT /admins/me`

> **Authentication:** Required (Admin)

Update the authenticated admin's profile.

**Request Body:**
```json
{
  "name": "John Updated",
  "organization": "New Org Name"
}
```

**Response:** `200 OK`
```json
{
  "data": {
    "id": "...",
    "name": "John Updated",
    "organization": "New Org Name",
    "updated_at": "2024-01-16T10:30:00Z"
  }
}
```

---

## 4. Worker Management

### `POST /workers`

> **Authentication:** Required (Admin)

Register a new worker under the authenticated admin.

**Request Body:**
```json
{
  "email": "worker@company.com",
  "name": "Jane Smith",
  "device_identifier": "DEVICE-ABC-123",
  "role_type": "OFFICE",
  "work_hours_start": "09:00",
  "work_hours_end": "18:00",
  "timezone": "America/Santiago"
}
```

**Response:** `201 Created`
```json
{
  "data": {
    "id": "01912345-6789-7abc-def0-123456789def",
    "admin_id": "01912345-6789-7abc-def0-123456789abc",
    "email": "worker@company.com",
    "name": "Jane Smith",
    "device_identifier": "DEVICE-ABC-123",
    "role_type": "OFFICE",
    "is_active": true,
    "consent_flags": {
      "system_activity": false,
      "network_activity": false,
      "productivity_basic": false,
      "productivity_screenshots": false,
      "file_activity": false,
      "location": false
    },
    "work_hours_start": "09:00",
    "work_hours_end": "18:00",
    "timezone": "America/Santiago",
    "created_at": "2024-01-15T10:30:00Z"
  }
}
```

---

### `GET /workers`

> **Authentication:** Required (Admin)

List all workers for the authenticated admin.

**Query Parameters:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `page` | int | 1 | Page number |
| `per_page` | int | 20 | Items per page (max 100) |
| `is_active` | bool | - | Filter by active status |
| `role_type` | string | - | Filter by OFFICE or FIELD |
| `search` | string | - | Search by name or email |
| `sort` | string | `created_at` | Sort field |
| `order` | string | `desc` | Sort order (asc/desc) |

**Response:** `200 OK`
```json
{
  "data": [
    {
      "id": "...",
      "email": "worker@company.com",
      "name": "Jane Smith",
      "role_type": "OFFICE",
      "is_active": true,
      "created_at": "..."
    }
  ],
  "pagination": {
    "total": 15,
    "page": 1,
    "per_page": 20,
    "total_pages": 1
  }
}
```

---

### `GET /workers/:id`

> **Authentication:** Required (Admin or same Worker)

Get a specific worker's profile.

**Response:** `200 OK`

---

### `PUT /workers/:id`

> **Authentication:** Required (Admin)

Update a worker's profile.

---

### `PATCH /workers/:id/consent`

> **Authentication:** Required (Worker — own consent only)

Update consent flags. Workers can only modify their own consent.

**Request Body:**
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

**Response:** `200 OK`
```json
{
  "data": {
    "id": "...",
    "consent_flags": {
      "system_activity": true,
      "network_activity": true,
      "productivity_basic": true,
      "productivity_screenshots": false,
      "file_activity": true,
      "location": false
    },
    "updated_at": "..."
  }
}
```

**Errors:**
| Code | Status | Condition |
|------|--------|-----------|
| `FORBIDDEN` | 403 | Worker trying to update another worker's consent |
| `VALIDATION_ERROR` | 400 | OFFICE worker trying to enable location |

---

### `DELETE /workers/:id`

> **Authentication:** Required (Admin)

Soft-delete (deactivate) a worker.

**Response:** `204 No Content`

---

## 5. Telemetry Endpoints

All telemetry endpoints follow the same pattern for each category. The endpoint structure is:

```
POST   /telemetry/{category}           # Submit telemetry batch (Worker)
GET    /telemetry/{category}           # Retrieve telemetry (Admin)
GET    /telemetry/{category}/:id       # Retrieve single record (Admin)
DELETE /telemetry/{category}/:id       # Delete single record (Admin)
```

Where `{category}` is one of: `system`, `network`, `productivity`, `files`, `location`.

### `POST /telemetry/{category}`

> **Authentication:** Required (Worker)

Submit a batch of encrypted telemetry records.

**Request Body:**
```json
{
  "records": [
    {
      "encrypted_payload": "<base64-encoded-encrypted-data>",
      "client_timestamp": "2024-01-15T10:30:00Z",
      "subcategory": "BASIC"
    }
  ]
}
```

> **Note:** `subcategory` is only required for `/telemetry/productivity` (values: `BASIC` or `SCREENSHOT`).

**Validation Rules:**
- Maximum 100 records per batch
- Maximum 10 MB per request body
- `encrypted_payload` must be valid base64
- `client_timestamp` must not be in the future (with 5-minute tolerance)
- Worker must have consent for the given category enabled
- For `location`: worker must be `FIELD` role type and within work hours

**Response:** `201 Created`
```json
{
  "data": {
    "batch_id": "01912345-6789-7abc-def0-000000000001",
    "records_created": 5,
    "category": "system",
    "server_timestamp": "2024-01-15T10:30:05Z"
  }
}
```

**Errors:**
| Code | Status | Condition |
|------|--------|-----------|
| `VALIDATION_ERROR` | 400 | Invalid payload format, future timestamps |
| `FORBIDDEN` | 403 | Worker lacks consent for this category |
| `PAYLOAD_TOO_LARGE` | 413 | Batch exceeds size limit |
| `RATE_LIMITED` | 429 | Too many submissions |

---

### `GET /telemetry/{category}`

> **Authentication:** Required (Admin)

Retrieve encrypted telemetry records for a specific worker.

**Query Parameters:**
| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `worker_id` | UUID | Yes | Worker whose data to retrieve |
| `from` | ISO 8601 | No | Start of time range (client_timestamp) |
| `to` | ISO 8601 | No | End of time range (client_timestamp) |
| `page` | int | No | Page number (default: 1) |
| `per_page` | int | No | Items per page (default: 20, max: 100) |
| `sort` | string | No | Sort field: `client_timestamp` or `server_timestamp` |
| `order` | string | No | `asc` or `desc` (default: `desc`) |
| `subcategory` | string | No | For productivity only: `BASIC` or `SCREENSHOT` |

**Response:** `200 OK`
```json
{
  "data": [
    {
      "id": "01912345-6789-7abc-def0-111111111111",
      "worker_id": "01912345-6789-7abc-def0-123456789def",
      "encrypted_payload": "<base64-encoded-encrypted-data>",
      "payload_size": 2048,
      "client_timestamp": "2024-01-15T10:30:00Z",
      "server_timestamp": "2024-01-15T10:30:05Z",
      "batch_id": "01912345-6789-7abc-def0-000000000001"
    }
  ],
  "pagination": {
    "total": 500,
    "page": 1,
    "per_page": 20,
    "total_pages": 25
  }
}
```

**Errors:**
| Code | Status | Condition |
|------|--------|-----------|
| `VALIDATION_ERROR` | 400 | Missing worker_id, invalid date range |
| `FORBIDDEN` | 403 | Admin doesn't own the specified worker |
| `RESOURCE_NOT_FOUND` | 404 | Worker doesn't exist |

---

### `GET /telemetry/{category}/:id`

> **Authentication:** Required (Admin)

Retrieve a single telemetry record by ID.

**Response:** `200 OK`

---

### `DELETE /telemetry/{category}/:id`

> **Authentication:** Required (Admin)

Hard-delete a single telemetry record (for data subject requests).

**Response:** `204 No Content`

---

## 6. Data Subject Rights

### `POST /workers/me/data-request`

> **Authentication:** Required (Worker)

Request a copy of all telemetry data associated with the authenticated worker (data portability right).

**Request Body:**
```json
{
  "type": "EXPORT",
  "categories": ["system_activity", "network_activity"],
  "from": "2024-01-01T00:00:00Z",
  "to": "2024-01-31T23:59:59Z"
}
```

**Response:** `202 Accepted`
```json
{
  "data": {
    "request_id": "...",
    "status": "PENDING",
    "estimated_completion": "2024-01-15T11:30:00Z"
  }
}
```

---

### `POST /workers/me/data-deletion`

> **Authentication:** Required (Worker)

Request deletion of telemetry data (right to erasure).

**Request Body:**
```json
{
  "categories": ["all"],
  "reason": "Personal request"
}
```

**Response:** `202 Accepted`
```json
{
  "data": {
    "request_id": "...",
    "status": "PENDING",
    "note": "Data will be deleted within 30 days as per retention policy"
  }
}
```

---

## Rate Limiting

Rate limits are applied per authenticated user:

| Endpoint Group | Rate Limit |
|---------------|------------|
| Authentication | 5 req/min per IP |
| Admin CRUD | 100 req/min |
| Worker CRUD | 100 req/min |
| Telemetry Ingestion (POST) | 10 req/min per worker |
| Telemetry Retrieval (GET) | 60 req/min per admin |
| Data Subject Requests | 5 req/day per worker |

Rate limit headers included in responses:
```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1705312200
```
