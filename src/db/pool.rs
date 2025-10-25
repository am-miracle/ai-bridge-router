use crate::config::Settings;
use crate::utils::errors::AppResult;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;
use tracing::{error, info};

/// Initialize PostgreSQL connection pool with Settings configuration
pub async fn init_pg_pool_with_config(settings: &Settings) -> AppResult<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(settings.database.max_connections)
        .min_connections(settings.database.min_connections)
        .acquire_timeout(Duration::from_secs(
            settings.database.connect_timeout_seconds,
        ))
        .idle_timeout(Duration::from_secs(settings.database.idle_timeout_seconds))
        .connect(&settings.database.url)
        .await?;

    // Test the connection
    sqlx::query("SELECT 1").fetch_one(&pool).await?;

    // Run migrations if needed
    run_migrations(&pool).await?;

    info!(
        "Database connection established successfully (max_conn={}, min_conn={})",
        settings.database.max_connections, settings.database.min_connections
    );
    Ok(pool)
}

/// Run database migrations
async fn run_migrations(pool: &PgPool) -> AppResult<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| {
            crate::utils::errors::AppError::internal(format!("Migration failed: {}", e))
        })?;

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
