use crate::{error::AppError, metrics::Metrics};
use sqlx::{postgres::PgConnectOptions, postgres::PgPoolOptions, PgPool};

use std::{env, str::FromStr, sync::Arc, time::Duration};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub metrics: Arc<Metrics>,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub port: u16,

    // Pool tuning
    pub pool_max_connections: u32,
    pub pool_min_connections: u32,

    // Timeouts
    pub db_connect_timeout: Duration, // timeout al crear el pool (startup)
    pub db_acquire_timeout: Duration, // timeout al esperar un conn del pool
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let database_url =
            env::var("DATABASE_URL").map_err(|_| AppError::missing_env("DATABASE_URL"))?;

        if !(database_url.starts_with("postgres://") || database_url.starts_with("postgresql://")) {
            return Err(AppError::invalid_config(
                "DATABASE_URL must start with postgres:// or postgresql://",
            ));
        }

        let port = parse_u16_env("PORT").unwrap_or(8080);
        if port == 0 {
            return Err(AppError::invalid_config("PORT must be between 1 and 65535"));
        }

        let pool_max_connections = parse_u32_env("DB_POOL_MAX_CONNECTIONS").unwrap_or(10);
        let pool_min_connections = parse_u32_env("DB_POOL_MIN_CONNECTIONS").unwrap_or(0);

        if pool_max_connections == 0 {
            return Err(AppError::invalid_config(
                "DB_POOL_MAX_CONNECTIONS must be >= 1",
            ));
        }
        if pool_min_connections > pool_max_connections {
            return Err(AppError::invalid_config(
                "DB_POOL_MIN_CONNECTIONS must be <= DB_POOL_MAX_CONNECTIONS",
            ));
        }

        let connect_timeout_secs = parse_u64_env("DB_CONNECT_TIMEOUT_SECS").unwrap_or(5);
        let acquire_timeout_secs = parse_u64_env("DB_ACQUIRE_TIMEOUT_SECS").unwrap_or(2);

        if connect_timeout_secs == 0 || connect_timeout_secs > 60 {
            return Err(AppError::invalid_config(
                "DB_CONNECT_TIMEOUT_SECS must be between 1 and 60",
            ));
        }
        if acquire_timeout_secs == 0 || acquire_timeout_secs > 60 {
            return Err(AppError::invalid_config(
                "DB_ACQUIRE_TIMEOUT_SECS must be between 1 and 60",
            ));
        }

        Ok(Self {
            database_url,
            port,
            pool_max_connections,
            pool_min_connections,
            db_connect_timeout: Duration::from_secs(connect_timeout_secs),
            db_acquire_timeout: Duration::from_secs(acquire_timeout_secs),
        })
    }
}

impl AppState {
    pub async fn new(cfg: &Config) -> Result<Self, AppError> {
        // Parse robusto del connection string
        let connect_opts = PgConnectOptions::from_str(&cfg.database_url).map_err(|_| {
            AppError::invalid_config(
                "DATABASE_URL is not a valid Postgres connection string (PgConnectOptions parse failed)",
            )
        })?;

        // Pool tuning
        let pool_fut = PgPoolOptions::new()
            .max_connections(cfg.pool_max_connections)
            .min_connections(cfg.pool_min_connections)
            .acquire_timeout(cfg.db_acquire_timeout)
            .idle_timeout(Duration::from_secs(30))
            .max_lifetime(Duration::from_secs(300))
            .connect_with(connect_opts);

        // Timeout externo (startup). Esto es lo más compatible.
        let pool = tokio::time::timeout(cfg.db_connect_timeout, pool_fut)
            .await
            .map_err(|_| AppError::invalid_config("DB connection timed out while creating pool"))?
            .map_err(AppError::Db)?;

        let metrics = Arc::new(Metrics::new());
        Ok(Self { pool, metrics })
    }

    /// Estado para tests: pool lazy (no requiere DB real).
    #[cfg(test)]
    pub fn for_tests() -> Self {
        let connect_opts = PgConnectOptions::from_str("postgres://postgres:postgres@localhost:5432/appdb")
            .expect("parse PgConnectOptions");

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(1))
            .connect_lazy_with(connect_opts);

        let metrics = Arc::new(Metrics::new());
        Self { pool, metrics }
    }
}

fn parse_u16_env(key: &str) -> Option<u16> {
    env::var(key).ok()?.parse::<u16>().ok()
}
fn parse_u32_env(key: &str) -> Option<u32> {
    env::var(key).ok()?.parse::<u32>().ok()
}
fn parse_u64_env(key: &str) -> Option<u64> {
    env::var(key).ok()?.parse::<u64>().ok()
}

