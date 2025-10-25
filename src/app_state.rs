use crate::cache::CacheClient;
use crate::config::Settings;
use crate::db::pool::init_pg_pool_with_config;
use crate::services::{GasPriceService, TokenPriceService};
use crate::utils::errors::{AppError, AppResult};

use sqlx::PgPool;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct AppState {
    pub start_time: Instant,
    pub pg_pool: PgPool,
    pub redis_client: CacheClient,
    pub gas_price_service: Arc<GasPriceService>,
    pub token_price_service: Arc<TokenPriceService>,
    pub http_client: reqwest::Client,
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

        // Initialize gas price service with Etherscan V2 API
        let gas_price_service =
            Arc::new(GasPriceService::new(settings.api_keys.etherscan_v2.clone()));

        // Initialize token price service
        let token_price_service =
            Arc::new(TokenPriceService::new(settings.api_keys.coingecko.clone()));

        // Create shared HTTP client with connection pooling
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .user_agent("bridge-router/1.0")
            .build()
            .map_err(|e| AppError::config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            start_time,
            pg_pool,
            redis_client,
            gas_price_service,
            token_price_service,
            http_client,
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

    /// Get gas price service reference
    pub fn gas_price_service(&self) -> &GasPriceService {
        &self.gas_price_service
    }

    /// Get token price service reference
    pub fn token_price_service(&self) -> &TokenPriceService {
        &self.token_price_service
    }

    /// Get shared HTTP client reference
    pub fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }
}
