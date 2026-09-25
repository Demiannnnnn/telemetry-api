//! # Telemetry API Entry Point
//!
//! Enterprise device telemetry REST API with end-to-end encryption.
//! Initializes configuration, database connection pool, structured tracing,
//! middleware pipeline, and manages graceful shutdown.

use std::net::SocketAddr;
use std::sync::Arc;
use telemetry_api::{
    config::Config,
    database,
    middleware::{build_cors_layer, security_headers_middleware},
    routes::create_router,
    AppState,
};
use tokio::signal;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Load typed configuration
    let config = Config::from_env()?;

    // 2. Initialize structured tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .init();

    tracing::info!("Starting Telemetry API v{}", env!("CARGO_PKG_VERSION"));

    // 3. Initialize database connection pool and run pending migrations
    let db_pool = database::init_pool(&config).await?;

    // 4. Construct application state
    let state = Arc::new(AppState::new(db_pool, config.clone()));

    // 5. Assemble router with middleware pipeline
    let cors = build_cors_layer(&config);
    let app = create_router(state)
        .layer(axum::middleware::from_fn(security_headers_middleware))
        .layer(cors)
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            config.max_payload_bytes,
        ))
        .layer(tower_http::trace::TraceLayer::new_for_http());

    // 6. Bind TCP listener
    let addr = SocketAddr::new(config.server_host.parse()?, config.server_port);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Server listening on http://{}", addr);

    // 7. Serve incoming requests with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Telemetry API shutdown completed gracefully");
    Ok(())
}

/// Listens for termination signals (Ctrl+C / SIGINT and SIGTERM on Unix) to enable graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
            sig.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Received Ctrl+C, initiating shutdown..."),
        _ = terminate => tracing::info!("Received SIGTERM, initiating shutdown..."),
    }
}
