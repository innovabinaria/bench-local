mod app;
mod error;
mod handlers;
mod metrics;
mod state;

use crate::state::{AppState, Config};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), error::AppError> {
    // Logs controllable via env: RUST_LOG=info|debug|trace

    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let cfg = Config::from_env()?;
    tracing::info!(
        port = cfg.port,
        pool_max = cfg.pool_max_connections,
        pool_min = cfg.pool_min_connections,
        connect_timeout_ms = cfg.db_connect_timeout.as_millis(),
        acquire_timeout_ms = cfg.db_acquire_timeout.as_millis(),
        "Starting rust-service"
    );

    let state = AppState::new(&cfg).await?;
    let router = app::build_router(state);

    let addr = format!("0.0.0.0:{}", cfg.port);
    tracing::info!(%addr, "Listening");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(error::AppError::Io)?;

    axum::serve(listener, router)
        .await
        .map_err(error::AppError::Io)?;

    Ok(())
}

#[cfg(test)]
mod app_tests;

