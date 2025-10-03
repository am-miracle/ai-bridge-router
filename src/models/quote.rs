use serde::{Deserialize, Serialize};

/// Query parameters for bridge quotes request
#[derive(Debug, Deserialize)]
pub struct QuoteParams {
    /// Source chain (e.g., "ethereum", "polygon")
    pub from_chain: String,
    /// Destination chain (e.g., "arbitrum", "optimism")
    pub to_chain: String,
    /// Token symbol (e.g., "USDC", "ETH")
    pub token: String,
    /// Amount to bridge as float (e.g., 1.5 for 1.5 USDC)
    pub amount: f64,
}

/// Individual quote response format as specified
#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteResponse {
    /// Bridge name (e.g., "Connext", "Hop", "Axelar")
    pub bridge: String,
    /// Total fee cost
    pub cost: f64,
    /// Estimated time in seconds
    pub est_time: u64,
    /// Human-readable liquidity string
    pub liquidity: String,
    /// Heuristic score from 0.0 to 1.0 (higher is better)
    pub score: f64,
}

/// Aggregated response containing all bridge quotes
#[derive(Debug, Serialize, Deserialize)]
pub struct AggregatedQuotesResponse {
    /// List of normalized routes from all bridges
    pub routes: Vec<QuoteResponse>,
}

/// Error response for when no quotes are available
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}
