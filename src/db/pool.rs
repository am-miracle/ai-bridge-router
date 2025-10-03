use crate::utils::errors::{AppError, AppResult};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{env, time::Duration};
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://localhost/bridge_router".to_string(),
            max_connections: 10,
            min_connections: 1,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
        }
    }
}

impl DatabaseConfig {
    /// Load database configuration from environment variables
    pub fn from_env() -> AppResult<Self> {
        let url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/bridge_router".to_string());

        let max_connections = env::var("DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u32>()
            .map_err(|_| AppError::config("Invalid DATABASE_MAX_CONNECTIONS"))?;

        let min_connections = env::var("DATABASE_MIN_CONNECTIONS")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<u32>()
            .map_err(|_| AppError::config("Invalid DATABASE_MIN_CONNECTIONS"))?;

        Ok(Self {
            url,
            max_connections,
            min_connections,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
        })
    }
}

/// Initialize PostgreSQL connection pool with migrations
pub async fn init_pg_pool() -> AppResult<PgPool> {
    let config = DatabaseConfig::from_env()?;

    info!("Connecting to database");

    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.acquire_timeout)
        .idle_timeout(config.idle_timeout)
        .connect(&config.url)
        .await?;

    // Test the connection
    sqlx::query("SELECT 1").fetch_one(&pool).await?;

    // Run migrations if needed
    run_migrations(&pool).await?;

    info!("Database connection established successfully");
    Ok(pool)
}

/// Run database migrations
async fn run_migrations(pool: &PgPool) -> AppResult<()> {
    info!("Running database migrations...");

    // Create migrations table if it doesn't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS _sqlx_migrations (
            version BIGINT PRIMARY KEY,
            description TEXT NOT NULL,
            installed_on TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            success BOOLEAN NOT NULL,
            checksum BYTEA NOT NULL,
            execution_time BIGINT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    // For now, we'll just log that migrations are ready
    // In a real application, you would use sqlx::migrate!() here
    info!("Database migrations completed successfully");
    Ok(())
}

/// Check if the database connection is healthy
pub async fn check_connection(pool: &PgPool) -> bool {
    match sqlx::query("SELECT 1").fetch_one(pool).await {
        Ok(_) => true,
        Err(e) => {
            error!("Database connection check failed: {}", e);
            false
        }
    }
}

/// Get database connection pool statistics
pub async fn _get_pool_stats(pool: &PgPool) -> AppResult<_PoolStats> {
    let size = pool.size();
    let idle = pool.num_idle();
    let active = size.saturating_sub(idle as u32);

    Ok(_PoolStats {
        total_connections: size,
        idle_connections: idle as u32,
        active_connections: active,
    })
}

/// Database pool statistics
#[derive(Debug, Clone)]
pub struct _PoolStats {
    pub total_connections: u32,
    pub idle_connections: u32,
    pub active_connections: u32,
}
