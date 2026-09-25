# Tarea 04: Pipeline de Middleware y Guards de Seguridad

| Atributo | Detalle |
|:---|:---|
| **ID de Tarea** | `TASK-04` |
| **Módulos Afectados** | [`src/middleware/`](file:///Users/demian/Documents/GitHub/telemetry-api/src/middleware/), [`src/routes/`](file:///Users/demian/Documents/GitHub/telemetry-api/src/routes/) |
| **Dependencias** | [`TASK-01`](file:///Users/demian/Documents/GitHub/telemetry-api/tasks/task-01-foundations-config-errors.md), [`TASK-03`](file:///Users/demian/Documents/GitHub/telemetry-api/tasks/task-03-authentication-and-crypto.md) |
| **Prioridad** | Alta / P1 |

---

## 1. Contexto y Arquitectura

El pipeline de middlewares actúa como el escudo perimetral de la API. Toda petición entrante es procesada en capas ordenadas secuencialmente antes de alcanzar los controladores de negocio:

```mermaid
flowchart TD
    Req[Petición HTTP Entrante] --> SecHeaders[1. Security Headers Layer]
    SecHeaders --> CORS[2. CORS Layer]
    CORS --> Trace[3. Tracing Layer con Sanitización]
    Trace --> BodyLimit[4. Request Body Limit (10MB)]
    BodyLimit --> RateLimit[5. Rate Limiting Guard (Token Bucket / Window)]
    RateLimit --> Extractors{6. Axum Extractors}
    Extractors -->|AuthUser| AuthGuard[Validación de JWT]
    AuthGuard -->|RequireAdmin / RequireWorker| RoleGuard[Control de Acceso RBAC]
    RoleGuard --> Handler[7. Handler de Dominio]
    Handler --> Resp[Respuesta HTTP Saliente]
```

---

## 2. Requerimientos Funcionales y Componentes

### 1. Inyección de Cabeceras de Seguridad
Toda respuesta emitida por el servidor debe incluir los siguientes encabezados para mitigar ataques comunes (XSS, Clickjacking, MIME sniffing):
```http
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 0
Content-Security-Policy: default-src 'none'
Cache-Control: no-store, no-cache, must-revalidate
Pragma: no-cache
Referrer-Policy: no-referrer
```

### 2. Capa CORS (`tower_http::cors::CorsLayer`)
- Configurar orígenes permitidos (desde `Config`).
- Métodos permitidos: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `OPTIONS`.
- Encabezados expuestos: `X-Total-Count`, `X-Total-Pages`, `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`.

### 3. Límite de Tamaño de Petición (`RequestBodyLimitLayer`)
- Limitar el cuerpo a **10 MB** (`10 * 1024 * 1024` bytes).
- Peticiones que superen el límite deben ser abortadas inmediatamente devolviendo HTTP `413 Payload Too Large` con código `PAYLOAD_TOO_LARGE`.

### 4. Sanitización en Logging (`TraceLayer`)
- Prohibir la captura o impresión de la cabecera `Authorization` o `Cookie`.
- Prohibir el volcado de los cuerpos de peticiones dirigidas a `/api/v1/telemetry/*`.

### 5. Control de Tasa (Rate Limiting)
- Control de tasa en memoria por IP o por usuario autenticado:
  - Autenticación (`/auth/*`): 5 peticiones/minuto.
  - Ingesta de telemetría: 10 peticiones/minuto por trabajador.
  - API general: 100 peticiones/minuto.

### 6. Extractores de Axum (`FromRequestParts`)
- `AuthUser`: Extrae y valida el JWT de la cabecera `Authorization: Bearer <token>`.
- `RequireAdmin`: Valida que el `AuthUser` tenga rol `UserRole::Admin` (de lo contrario retorna `403 Forbidden`).
- `RequireWorker`: Valida que el `AuthUser` tenga rol `UserRole::Worker` (de lo contrario retorna `403 Forbidden`).

---

## 3. Arquitectura de Código y Pseudocódigo

### A. Extractores de Autenticación y Rol ([`src/middleware/mod.rs`](file:///Users/demian/Documents/GitHub/telemetry-api/src/middleware/mod.rs))

```rust
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts},
};
use uuid::Uuid;
use crate::{
    crypto::{self, Claims, UserRole},
    errors::AppError,
    AppState,
};

/// Estructura que representa al usuario autenticado extraído del token JWT.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub role: UserRole,
    pub email: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        // 1. Extraer cabecera Authorization
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|val| val.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

        // 2. Extraer prefijo Bearer
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::Unauthorized("Invalid Authorization header format. Expected Bearer <token>".to_string()))?;

        // 3. Validar token y claims
        let claims = crypto::verify_jwt(token, &app_state.config.jwt_secret)?;

        Ok(AuthUser {
            id: claims.sub,
            role: claims.role,
            email: claims.email,
        })
    }
}

/// Extractor Guard para requerir rol exclusivo de Administrador.
#[derive(Debug, Clone)]
pub struct RequireAdmin(pub AuthUser);

#[async_trait]
impl<S> FromRequestParts<S> for RequireAdmin
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        if auth_user.role != UserRole::Admin {
            return Err(AppError::Forbidden("Administrator privileges required".to_string()));
        }
        Ok(RequireAdmin(auth_user))
    }
}

/// Extractor Guard para requerir rol exclusivo de Worker.
#[derive(Debug, Clone)]
pub struct RequireWorker(pub AuthUser);

#[async_trait]
impl<S> FromRequestParts<S> for RequireWorker
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        if auth_user.role != UserRole::Worker {
            return Err(AppError::Forbidden("Worker privileges required".to_string()));
        }
        Ok(RequireWorker(auth_user))
    }
}
```

### B. Helper de Verificación de Propiedad (Ownership Verification)

```rust
use sqlx::PgPool;
use uuid::Uuid;
use crate::errors::AppError;

/// Verifica en base de datos que el trabajador especificado pertenezca al administrador autenticado.
pub async fn verify_admin_owns_worker(
    admin_id: Uuid,
    worker_id: Uuid,
    pool: &PgPool,
) -> Result<(), AppError> {
    let record = sqlx::query!(
        "SELECT admin_id FROM workers WHERE id = $1 AND deleted_at IS NULL",
        worker_id
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    match record {
        Some(row) if row.admin_id == admin_id => Ok(()),
        Some(_) => Err(AppError::Forbidden("You do not have permission to access this worker's data".to_string())),
        None => Err(AppError::NotFound(format!("Worker with ID '{}' not found", worker_id))),
    }
}
```

---

## 4. Prácticas de Programación y Reglas de Seguridad

- **Fallo Seguro (Fail-Safe Defaults):** Si una cabecera de autenticación no está presente o el token está mal formado, el extractor debe rechazar inmediatamente la petición sin llegar a la base de datos ni invocar el controlador.
- **Sanitización de Trazas:** Configurar el `TraceLayer` de `tower-http` para filtrar cabeceras sensibles:
  ```rust
  use tower_http::trace::TraceLayer;
  use tracing::Level;

  let trace_layer = TraceLayer::new_for_http()
      .make_span_with(|req: &http::Request<_>| {
          tracing::info_span!(
              "http_request",
              method = %req.method(),
              uri = %req.uri().path(),
              // NO incluir req.headers() ni el body
          )
      });
  ```

---

## 5. Validaciones y Restricciones

- Encabezado `Authorization`: Obligatorio formato exacto `Bearer <jwt_token>`.
- Límite de carga: Todo payload HTTP mayor a 10MB debe cortarse en el middleware con status 413.
- Cabecera `Content-Type`: Las peticiones POST/PUT deben requerir `application/json`.

---

## 6. Logs y Observabilidad

- Cuando un usuario sea bloqueado por rate limit, emitir `tracing::warn!(client_ip = %ip, "Rate limit reached")`.
- Cuando un intento de acceso no autorizado o prohibido falle, emitir `tracing::warn!(reason = %msg, path = %path, "Authorization failed")`.
- **Nunca registrar tokens ni cookies en los logs.**

---

## 7. Criterios de Aceptación y Definición de Terminado (DoD)

1. [ ] Los extractores `AuthUser`, `RequireAdmin` y `RequireWorker` están implementados y rechazan peticiones inválidas con `401 Unauthorized` o `403 Forbidden`.
2. [ ] Peticiones de más de 10 MB son rechazadas automáticamente con `413 Payload Too Large`.
3. [ ] Cabeceras de seguridad (`X-Frame-Options`, `Content-Security-Policy`, etc.) están presentes en todas las respuestas HTTP.
4. [ ] La función `verify_admin_owns_worker` previene efectivamente que un administrador acceda a trabajadores de otro administrador.
5. [ ] Pruebas unitarias de extractores pasando limpiamente.

---

## 8. Especificación de Pruebas

### Pruebas Unitarias
- `test_middleware_auth_missing_header_returns_401`: Petición a endpoint protegido sin cabecera `Authorization` debe retornar 401.
- `test_middleware_auth_invalid_bearer_prefix_returns_401`: Petición con `Authorization: Basic 123` debe retornar 401 con mensaje de formato inválido.
- `test_middleware_require_admin_with_worker_token_returns_403`: Petición a ruta administrativa usando un token emitido para `WORKER` debe retornar 403 Forbidden.

### Pruebas de Integración (Base de Datos)
- `test_ownership_guard_admin_accessing_other_admin_worker_returns_403`: Crear Administrador A con Trabajador A y Administrador B; verificar que `verify_admin_owns_worker` para Administrador B con Trabajador A retorne `AppError::Forbidden`.
