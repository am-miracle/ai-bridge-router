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
    /// Optional slippage tolerance as percentage (e.g., 0.5 for 0.5%)
    /// Defaults to 0.5% if not provided
    #[serde(default = "default_slippage")]
    pub slippage: f64,
}

fn default_slippage() -> f64 {
    0.5 // 0.5% default slippage
}

/// Cost details for a bridge quote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostDetails {
    /// Total fee in token units
    pub total_fee: f64,
    /// Total fee in USD
    pub total_fee_usd: f64,
    /// Cost breakdown
    pub breakdown: CostBreakdown,
}

/// Breakdown of costs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    /// Bridge protocol fee
    pub bridge_fee: f64,
    /// Estimated gas cost in USD (source + destination)
    pub gas_estimate_usd: f64,
    /// Detailed gas breakdown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_details: Option<GasDetails>,
}

/// Detailed gas cost breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasDetails {
    /// Source chain gas cost in USD
    pub source_gas_usd: f64,
    /// Destination chain gas cost in USD
    pub destination_gas_usd: f64,
    /// Source chain name
    pub source_chain: String,
    /// Destination chain name
    pub destination_chain: String,
    /// Source gas price in Gwei
    pub source_gas_price_gwei: f64,
    /// Destination gas price in Gwei
    pub destination_gas_price_gwei: f64,
    /// Source gas limit
    pub source_gas_limit: u64,
    /// Destination gas limit
    pub destination_gas_limit: u64,
}

/// Output amount details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputDetails {
    /// Expected amount to receive
    pub expected: f64,
    /// Guaranteed minimum amount after slippage
    pub minimum: f64,
    /// Input amount
    pub input: f64,
}

/// Timing details for the transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingDetails {
    /// Estimated time in seconds
    pub seconds: u64,
    /// Human-readable display format
    pub display: String,
    /// Speed category
    pub category: String, // "fast" | "medium" | "slow"
}

/// Security information for the bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityDetails {
    /// Overall security score (0.0 to 1.0)
    pub score: f64,
    /// Security level category
    pub level: String, // "high" | "medium" | "low"
    /// Has security audit
    pub has_audit: bool,
    /// Has known exploit history
    pub has_exploit: bool,
}

/// Individual quote response with detailed breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteResponse {
    /// Bridge name (e.g., "Hop", "Axelar", "Everclear")
    pub bridge: String,
    /// Overall heuristic score from 0.0 to 1.0 (higher is better)
    pub score: f64,
    /// Cost details
    pub cost: CostDetails,
    /// Output amount details
    pub output: OutputDetails,
    /// Timing details
    pub timing: TimingDetails,
    /// Security details
    pub security: SecurityDetails,
    /// Whether this route is available for use
    pub available: bool,
    /// Operational status
    pub status: String, // "operational" | "degraded" | "unavailable"
    /// Warnings about this route
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Request metadata in response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetadata {
    pub from: String,
    pub to: String,
    pub token: String,
    pub amount: f64,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub total_routes: usize,
    pub available_routes: usize,
    pub request: RequestMetadata,
}

/// Aggregated response containing all bridge quotes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedQuotesResponse {
    /// List of detailed routes from all bridges
    pub routes: Vec<QuoteResponse>,
    /// Metadata about the request and response
    pub metadata: ResponseMetadata,
    /// List of errors for bridges that failed or timed out
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub errors: Vec<BridgeQuoteError>,
}

/// Error information for a single bridge quote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeQuoteError {
    pub bridge: String,
    pub error: String,
}

/// Error response for when no quotes are available
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Helper function to categorize timing
pub fn categorize_timing(seconds: u64) -> String {
    match seconds {
        0..=120 => "fast".to_string(),     // <= 2 minutes
        121..=600 => "medium".to_string(), // 2-10 minutes
        _ => "slow".to_string(),           // > 10 minutes
    }
}

/// Helper function to format timing display
pub fn format_timing(seconds: u64) -> String {
    if seconds < 60 {
        format!("~{} sec", seconds)
    } else if seconds < 3600 {
        format!("~{} min", seconds / 60)
    } else {
        format!("~{} hr", seconds / 3600)
    }
}

/// Helper function to categorize security level
pub fn categorize_security(score: f64) -> String {
    if score >= 0.7 {
        "high".to_string()
    } else if score >= 0.4 {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_timing() {
        assert_eq!(categorize_timing(60), "fast");
        assert_eq!(categorize_timing(120), "fast");
        assert_eq!(categorize_timing(300), "medium");
        assert_eq!(categorize_timing(600), "medium");
        assert_eq!(categorize_timing(1200), "slow");
    }

    #[test]
    fn test_format_timing() {
        assert_eq!(format_timing(45), "~45 sec");
        assert_eq!(format_timing(180), "~3 min");
        assert_eq!(format_timing(3600), "~1 hr");
    }

    #[test]
    fn test_categorize_security() {
        assert_eq!(categorize_security(0.85), "high");
        assert_eq!(categorize_security(0.5), "medium");
        assert_eq!(categorize_security(0.3), "low");
    }
}
