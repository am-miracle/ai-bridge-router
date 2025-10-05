use crate::cache::CacheClient;
use crate::config::Settings;
use crate::db::pool::init_pg_pool_with_config;
use crate::utils::errors::{AppError, AppResult};

use sqlx::PgPool;
use std::time::Instant;

#[derive(Clone)]
pub struct AppState {
    pub start_time: Instant,
    pub pg_pool: PgPool,
    pub redis_client: CacheClient,
}

impl AppState {
    /// Create a new application state
    pub async fn new() -> AppResult<Self> {
        let start_time = Instant::now();

        // Load configuration
        let settings = Settings::new()
            .map_err(|e| AppError::config(format!("Failed to load settings: {}", e)))?;

        // Initialize database pool with settings
        let pg_pool = init_pg_pool_with_config(&settings).await?;

        // Initialize Redis client with settings
        let redis_client = CacheClient::with_settings(&settings).await?;

        Ok(Self {
            start_time,
            pg_pool,
            redis_client,
        })
    }

    /// Get database pool reference
    pub fn db(&self) -> &PgPool {
        &self.pg_pool
    }

    /// Get Redis client reference
    pub fn cache(&self) -> &CacheClient {
        &self.redis_client
    }

    /// Get server uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}
