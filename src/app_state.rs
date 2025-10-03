use crate::cache::CacheClient;
use crate::utils::errors::AppResult;
use sqlx::PgPool;
use std::time::Instant;

/// Global application state
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

        // Initialize database pool
        let pg_pool = crate::db::pool::init_pg_pool().await?;

        // Initialize Redis client
        let redis_client = CacheClient::new().await?;

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
