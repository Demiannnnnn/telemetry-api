# Tarea 01: Infraestructura Base — Configuración, Errores y Tracing

| Atributo | Detalle |
|:---|:---|
| **ID de Tarea** | `TASK-01` |
| **Módulos Afectados** | [`src/config/`](file:///Users/demian/Documents/GitHub/telemetry-api/src/config/), [`src/errors/`](file:///Users/demian/Documents/GitHub/telemetry-api/src/errors/), [`src/models/`](file:///Users/demian/Documents/GitHub/telemetry-api/src/models/) |
| **Dependencias** | Ninguna (Tarea Raíz) |
| **Prioridad** | Crítica / P0 |

---

## 1. Contexto y Arquitectura

El núcleo de la API requiere una base inmutable para:
1. **Configuración Tipada:** Carga de variables de entorno mediante `dotenvy`, serializada en una estructura inmutable (`Config`) con valores por defecto seguros y validación temprana en el arranque.
2. **Jerarquía Centralizada de Errores (`AppError`):** Manejo de errores idiomático con `thiserror`, que se convierte directamente en respuestas HTTP de Axum (`IntoResponse`) garantizando que ningún error filtre trazas internas, contraseñas o detalles de la base de datos al cliente.
3. **Estructuras de Respuesta Comunes (DTOs):** Normalización de la estructura JSON en todas las respuestas exitosas y de error:
   - Recurso único: `{ "data": T }`
   - Colección paginada: `{ "data": [T], "pagination": PaginationMeta }`
   - Error: `{ "error": { "code": string, "message": string, "details": Option<Value> } }`
4. **Sistema de Tracing:** Inicialización de `tracing-subscriber` con soporte para formato JSON estructurado o compacto, filtrado dinámico mediante `RUST_LOG`, y redacción obligatoria de cabeceras sensibles.

```mermaid
flowchart TD
    Env[Variables de Entorno .env] --> Dotenv[dotenvy::dotenv]
    Dotenv --> ConfigStruct[Config::from_env]
    ConfigStruct --> AppState[axum::extract::State Arc AppState]
    
    Handler[Axum Handler Logic] -->|Result T, AppError| HandlerResult
    HandlerResult -->|Err AppError| IntoResponse[impl IntoResponse for AppError]
    IntoResponse --> JsonErrorResponse[HTTP Status + JSON Error Response]
```

---

## 2. Requerimientos Funcionales y Reglas de Negocio

- **RF-01-1 (Fail Fast):** Si alguna variable requerida (`DATABASE_URL`, `JWT_SECRET`) falta o es inválida en el arranque, la aplicación debe fallar inmediatamente imprimiendo un mensaje claro y terminar el proceso con código de salida diferente de 0.
- **RF-01-2 (Formato Canónico de Errores):** Cualquier error originado en cualquier capa de la aplicación debe mapearse a un código de error unificado en mayúsculas (`VALIDATION_ERROR`, `UNAUTHORIZED`, `FORBIDDEN`, `RESOURCE_NOT_FOUND`, `CONFLICT`, `INTERNAL_ERROR`).
- **RF-01-3 (Seguridad de Errores Internos):** Los errores de base de datos (`sqlx::Error`) o fallos no controlados deben ser registrados con nivel `tracing::error!` con su contexto completo en los logs del servidor, pero al cliente externo se le debe entregar un código `INTERNAL_ERROR` y el mensaje genérico `"An unexpected internal server error occurred"`.
- **RF-01-4 (Contrato de Paginación):** Definir parámetros de consulta estándar `PaginationParams` (`page`, `per_page`) y metadata de respuesta `PaginationMeta` (`total`, `page`, `per_page`, `total_pages`).

---

## 3. Arquitectura de Código y Pseudocódigo

### A. Módulo de Configuración ([`src/config/mod.rs`](file:///Users/demian/Documents/GitHub/telemetry-api/src/config/mod.rs))

```rust
// Pseudocódigo / Estructura en Rust
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub database_url: String,
    pub database_max_connections: u32,
    pub jwt_secret: String,
    pub jwt_access_expiration_hours: i64,
    pub jwt_refresh_expiration_days: i64,
    pub environment: Environment,
    pub max_payload_bytes: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    Test,
    Production,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok(); // Ignorar si no existe el archivo .env físico

        let server_host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let server_port = std::env::var("SERVER_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidPort)?;

        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| ConfigError::MissingVariable("DATABASE_URL"))?;

        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| ConfigError::MissingVariable("JWT_SECRET"))?;

        if jwt_secret.len() < 32 {
            return Err(ConfigError::WeakSecret("JWT_SECRET must be at least 32 characters"));
        }

        // Parsear restantes y retornar instancia inmutable
        Ok(Self { /* ... */ })
    }
}
```

### B. Módulo de Errores ([`src/errors/mod.rs`](file:///Users/demian/Documents/GitHub/telemetry-api/src/errors/mod.rs))

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Authentication required")]
    Unauthorized(String),

    #[error("Forbidden access: {0}")]
    Forbidden(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Payload too large: {0}")]
    PayloadTooLarge(String),

    #[error("Too many requests")]
    RateLimited,

    #[error("Database error occurred")]
    Database(#[from] sqlx::Error),

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetails,
}

#[derive(Serialize)]
pub struct ErrorDetails {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "RESOURCE_NOT_FOUND", msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg.clone()),
            AppError::PayloadTooLarge(msg) => (StatusCode::PAYLOAD_TOO_LARGE, "PAYLOAD_TOO_LARGE", msg.clone()),
            AppError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED", "Rate limit exceeded".to_string()),
            AppError::Database(err) => {
                tracing::error!(error = %err, "Database query failure");
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "An internal database error occurred".to_string())
            },
            AppError::Internal(err) => {
                tracing::error!(error = %err, "Unhandled application error");
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "An unexpected server error occurred".to_string())
            }
        };

        let body = Json(ErrorResponse {
            error: ErrorDetails {
                code,
                message,
                details: None,
            },
        });

        (status, body).into_response()
    }
}
```

### C. Módulo de Modelos Comunes ([`src/models/mod.rs`](file:///Users/demian/Documents/GitHub/telemetry-api/src/models/mod.rs))

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct SingleResponse<T> {
    pub data: T,
}

#[derive(Serialize)]
pub struct CollectionResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Serialize, Debug, Clone)]
pub struct PaginationMeta {
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

impl PaginationParams {
    pub fn offset(&self) -> i64 {
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page.unwrap_or(20).clamp(1, 100);
        ((page - 1) * per_page) as i64
    }

    pub fn limit(&self) -> i64 {
        self.per_page.unwrap_or(20).clamp(1, 100) as i64
    }
}
```

---

## 4. Prácticas de Programación y Reglas de Seguridad

- **Prohibición de `.unwrap()` y `.expect()`:** Utilizar propagación de errores mediante operador `?` y conversiones automáticas con `#[from]`.
- **Inmutabilidad:** La estructura `Config` debe ser inmutable y compartida como `Arc<Config>` o dentro de un `AppState` de Axum.
- **Redacción de Credenciales en Logs:** Al imprimir logs de configuración, jamás imprimir el contenido de `DATABASE_URL` completo (contiene contraseñas) ni `JWT_SECRET`. Registrar únicamente valores enmascarados como `postgres://user:***@host:5432/db`.

---

## 5. Validaciones y Restricciones

- `SERVER_PORT`: Debe ser un número entero entre `1` y `65535`.
- `JWT_SECRET`: Mínimo 32 caracteres para asegurar entropía suficiente para firmas HMAC-SHA256.
- `PaginationParams`: `page` mínimo `1`, `per_page` con límite estricto de `1` a `100` registros.

---

## 6. Logs y Observabilidad

- Inicializar el suscriptor de tracing en `main.rs` con formato estructurado:
  ```rust
  tracing_subscriber::fmt()
      .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
      .with_target(false)
      .compact()
      .init();
  ```
- En caso de error de base de datos o interno, loguear con campos estructurados: `tracing::error!(error = %err, context = "...", "Message")`.

---

## 7. Criterios de Aceptación y Definición de Terminado (DoD)

1. [ ] El módulo `config` lee variables desde archivo o entorno y valida que `JWT_SECRET` tenga longitud $\ge 32$.
2. [ ] `AppError` implementa `IntoResponse` y mapea adecuadamente cada variante de error al código HTTP y formato JSON especificado.
3. [ ] Los errores internos de base de datos o fallos del sistema nunca filtran queries, nombres de tablas ni credenciales al cliente.
4. [ ] Las estructuras `SingleResponse<T>`, `CollectionResponse<T>` y `PaginationParams` están exportadas y disponibles en `models`.
5. [ ] Pruebas unitarias pasan con `cargo test`.
6. [ ] Cero warnings con `cargo clippy -- -D warnings`.

---

## 8. Especificación de Pruebas

### Pruebas Unitarias
- `test_config_missing_jwt_secret_fails`: Verificar que la omisión de `JWT_SECRET` produce un error de configuración.
- `test_config_weak_jwt_secret_fails`: Verificar que un secreto de menos de 32 caracteres es rechazado.
- `test_pagination_defaults_and_limits`: Verificar que `per_page` mayor a 100 se recorta a 100 y páginas menores a 1 toman valor 1.
- `test_app_error_into_response_mapping`: Instanciar `AppError::NotFound("worker")` y comprobar que devuelve código de estado HTTP 404 con JSON `{"error": {"code": "RESOURCE_NOT_FOUND", "message": "worker", "details": null}}`.
