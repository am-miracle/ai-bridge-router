use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

use crate::cache::CacheClient;

/// Unified bridge quote response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeQuote {
    /// Bridge name (e.g., "Connext", "Hop", "Axelar")
    pub bridge: String,
    /// Fee in native units (e.g., ETH, USDC)
    pub fee: f64,
    /// Estimated time in seconds
    pub est_time: u64,
    /// Human-readable liquidity string (e.g., "1,000,000 USDC")
    pub liquidity: String,
    /// Heuristic score from 0.0 to 1.0 (higher is better)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// Optional additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Bridge-specific error types
#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("Request timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Bad response from bridge API: {message}")]
    BadResponse { message: String },

    #[error("Unsupported asset: {asset}")]
    UnsupportedAsset { asset: String },

    #[error("Unsupported route: {from_chain} -> {to_chain}")]
    UnsupportedRoute {
        from_chain: String,
        to_chain: String,
    },

    // #[error("Insufficient liquidity for amount: {amount}")]
    // InsufficientLiquidity { amount: String },
    #[error("Network error: {source}")]
    Network { source: reqwest::Error },

    #[error("JSON parsing error: {source}")]
    JsonParsing { source: serde_json::Error },

    #[error("API rate limit exceeded")]
    RateLimited,

    #[error("Bridge service unavailable")]
    ServiceUnavailable,

    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl From<reqwest::Error> for BridgeError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            BridgeError::Timeout { timeout_ms: 30000 }
        } else if err.is_status() {
            match err.status() {
                Some(reqwest::StatusCode::TOO_MANY_REQUESTS) => BridgeError::RateLimited,
                Some(reqwest::StatusCode::SERVICE_UNAVAILABLE) => BridgeError::ServiceUnavailable,
                _ => BridgeError::Network { source: err },
            }
        } else {
            BridgeError::Network { source: err }
        }
    }
}

impl From<serde_json::Error> for BridgeError {
    fn from(err: serde_json::Error) -> Self {
        BridgeError::JsonParsing { source: err }
    }
}

/// Bridge quote request parameters
#[derive(Debug, Clone)]
pub struct BridgeQuoteRequest {
    pub asset: String,
    pub from_chain: String,
    pub to_chain: String,
    pub amount: Option<String>,
}

/// Shared HTTP client configuration for all bridge clients
#[derive(Clone)]
pub struct BridgeClientConfig {
    pub client: reqwest::Client,
    pub timeout: Duration,
    pub retries: u32,
    pub cache: Option<CacheClient>,
    pub hop_config: Option<crate::services::bridge_client::hop::HopConfig>,
}

impl BridgeClientConfig {
    /// Create a new bridge client configuration
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("bridge-router/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            timeout: Duration::from_secs(30),
            retries: 3,
            cache: None,
            hop_config: None,
        }
    }

    /// Set cache client for response caching
    pub fn with_cache(mut self, cache: CacheClient) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Set timeout for requests
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set number of retries for failed requests
    pub fn with_retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    /// Set Hop configuration
    pub fn with_hop_config(
        mut self,
        hop_config: crate::services::bridge_client::hop::HopConfig,
    ) -> Self {
        self.hop_config = Some(hop_config);
        self
    }
}

impl Default for BridgeClientConfig {
    fn default() -> Self {
        Self::new()
    }
}
