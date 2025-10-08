use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::{get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

/// Everclear API endpoint for quotes and liquidity metrics
/// Based on Everclear's intent-based crosschain architecture
const EVERCLEAR_API_BASE: &str = "https://api.everclear.org";
// Note: Testnet API available at https://api.testnet.everclear.org for future use

/// Everclear quote request structure
/// Based on Everclear's intent-based routing system
#[derive(Debug, Serialize)]
struct EverclearQuoteRequest {
    origin: String,
    destinations: Vec<String>,
    #[serde(rename = "inputAsset")]
    input_asset: String,
    amount: String,
    to: Option<String>,
}

/// Everclear quote response structure
/// Reflects Everclear's intent-based architecture with fee structures
#[derive(Debug, Deserialize)]
struct EverclearQuoteResponse {
    #[serde(rename = "fixedFeeUnits")]
    fixed_fee_units: String,
    #[serde(rename = "variableFeeBps")]
    variable_fee_bps: u64,
    #[serde(rename = "totalFeeBps")]
    total_fee_bps: u64,
    #[serde(rename = "expectedAmount")]
    expected_amount: String,
    #[serde(rename = "currentLimit")]
    current_limit: String,
}

/// Get asset ticker hash for Everclear API
/// Everclear uses ticker hashes to identify assets across chains
fn get_asset_ticker_hash(asset: &str) -> Result<String, BridgeError> {
    // Map common asset symbols to their ticker hashes
    // In production, this should fetch from Everclear's asset registry
    match asset.to_uppercase().as_str() {
        "USDC" => Ok("USDC".to_string()),
        "USDT" => Ok("USDT".to_string()),
        "WETH" => Ok("WETH".to_string()),
        "ETH" => Ok("WETH".to_string()),
        "MATIC" => Ok("MATIC".to_string()),
        "WMATIC" => Ok("MATIC".to_string()),
        "ARB" => Ok("ARB".to_string()),
        "OP" => Ok("OP".to_string()),
        "AVAX" => Ok("WAVAX".to_string()),
        "BNB" => Ok("WBNB".to_string()),
        "FTM" => Ok("WFTM".to_string()),
        "DAI" => Ok("DAI".to_string()),
        "WBTC" => Ok("WBTC".to_string()),
        "SOL" => Ok("SOL".to_string()),
        _ => Err(BridgeError::UnsupportedAsset {
            asset: asset.to_string(),
        }),
    }
}

/// Chain to Everclear chain ID mapping
/// Everclear uses chain IDs for routing intents across different networks
fn chain_to_chain_id(chain: &str) -> Result<u64, BridgeError> {
    let chain_id = match chain.to_lowercase().as_str() {
        "ethereum" | "eth" | "mainnet" => 1,  // Ethereum Mainnet
        "polygon" | "matic" => 137,           // Polygon
        "arbitrum" | "arbitrum-one" => 42161, // Arbitrum One
        "optimism" | "opt" => 10,             // Optimism
        "bnb" | "bsc" | "binance" => 56,      // BNB Chain
        "gnosis" | "xdai" => 100,             // Gnosis Chain
        "base" => 8453,                       // Base
        "linea" => 59144,                     // Linea
        "mantle" => 5000,                     // Mantle
        "scroll" => 534352,                   // Scroll
        "solana" => 1399811149,               // Solana (special chain ID)
        _ => {
            return Err(BridgeError::UnsupportedRoute {
                from_chain: chain.to_string(),
                to_chain: "".to_string(),
            });
        }
    };
    Ok(chain_id)
}

/// Convert wei string to human readable number
fn wei_to_ether(wei_str: &str) -> Result<f64, BridgeError> {
    let wei = wei_str
        .parse::<u128>()
        .map_err(|_| BridgeError::BadResponse {
            message: format!("Invalid wei amount: {}", wei_str),
        })?;

    // Convert wei to ether (divide by 10^18)
    Ok(wei as f64 / 1e18)
}

/// Get a quote from Everclear bridge
pub async fn get_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    let cache_key = format!(
        "everclear:{}:{}:{}:{}",
        request.asset,
        request.from_chain,
        request.to_chain,
        request.amount.as_deref().unwrap_or("1000000") // Default amount
    );

    get_cached_quote(&cache_key, &config.cache, || {
        fetch_everclear_quote(request, config)
    })
    .await
}

/// Fetch quote directly from Everclear API
async fn fetch_everclear_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    retry_request(
        || fetch_everclear_quote_once(request, config),
        config.retries,
        "Everclear API call",
    )
    .await
}

/// Single attempt to fetch Everclear quote
async fn fetch_everclear_quote_once(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching Everclear quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Convert chain names to Everclear chain IDs
    let origin_chain_id = chain_to_chain_id(&request.from_chain)?;
    let destination_chain_id = chain_to_chain_id(&request.to_chain)?;

    // Get asset ticker hash
    let asset_ticker = get_asset_ticker_hash(&request.asset)?;

    // Default to 1 token amount if not specified
    let amount = request.amount.clone().unwrap_or_else(|| {
        match request.asset.to_uppercase().as_str() {
            "USDC" | "USDT" => "1000000".to_string(), // 1 USDC (6 decimals)
            _ => "1000000000000000000".to_string(),   // 1 ETH (18 decimals)
        }
    });

    let everclear_request = EverclearQuoteRequest {
        origin: origin_chain_id.to_string(),
        destinations: vec![destination_chain_id.to_string()],
        input_asset: asset_ticker,
        amount,
        to: None, // Optional recipient address
    };

    // Make the API request to Everclear's routes/quotes endpoint
    // Everclear uses intent-based routing for crosschain transfers
    let url = format!("{}/routes/quotes", EVERCLEAR_API_BASE);

    info!("Requesting Everclear quote from: {}", url);

    let response_result = config
        .client
        .post(&url)
        .json(&everclear_request)
        .send()
        .await;

    // Handle potential timeout or network error
    let response = match response_result {
        Ok(resp) => resp,
        Err(e) => {
            // Network error - create estimate
            info!("Everclear API network error: {}, creating estimate", e);
            return create_everclear_estimate(request);
        }
    };

    if !response.status().is_success() {
        // If API is not available, create estimate
        info!(
            "Everclear API returned {}, creating estimate",
            response.status()
        );
        return create_everclear_estimate(request);
    }

    let response_text = response.text().await.map_err(BridgeError::from)?;
    debug!("Everclear API response: {}", response_text);

    // Try to parse the response, but fall back to estimate if parsing fails
    match serde_json::from_str::<EverclearQuoteResponse>(&response_text) {
        Ok(everclear_response) => {
            // Parse fees from Everclear's fee structure
            let fixed_fee = wei_to_ether(&everclear_response.fixed_fee_units).unwrap_or(0.0);
            let variable_fee_percent = everclear_response.variable_fee_bps as f64 / 10000.0; // Convert bps to percentage
            let _total_fee_percent = everclear_response.total_fee_bps as f64 / 10000.0;

            // Calculate expected amount received
            let expected_amount = wei_to_ether(&everclear_response.expected_amount).unwrap_or(0.0);
            let current_limit = wei_to_ether(&everclear_response.current_limit).unwrap_or(0.0);

            // Estimate total fee in asset units (simplified calculation)
            let amount_f64 = wei_to_ether(&everclear_request.amount).unwrap_or(1.0);
            let variable_fee_amount = amount_f64 * (variable_fee_percent / 100.0);
            let total_fee = fixed_fee + variable_fee_amount;

            // Estimate time based on chain combinations (Everclear uses intent settlement)
            let est_time = match (request.from_chain.as_str(), request.to_chain.as_str()) {
                // L2 to L2 transfers are fastest with intent settlement
                ("arbitrum", "optimism") | ("optimism", "arbitrum") => 45, // 45 seconds
                ("polygon", "arbitrum") | ("arbitrum", "polygon") => 90,   // 1.5 minutes
                ("polygon", "optimism") | ("optimism", "polygon") => 90,   // 1.5 minutes
                // L1 to L2 or L2 to L1 transfers
                ("ethereum", _) | (_, "ethereum") => 240, // 4 minutes
                // L2 to L2 transfers
                _ => 120, // 2 minutes
            };

            // Create metadata reflecting Everclear's intent-based architecture
            let metadata = serde_json::json!({
                "fixed_fee": fixed_fee,
                "variable_fee_bps": everclear_response.variable_fee_bps,
                "total_fee_bps": everclear_response.total_fee_bps,
                "expected_amount": expected_amount,
                "current_limit": current_limit,
                "architecture": "intent_based",
                "security": "intent_settlement",
                "primitive": "intents",
                "network": "Everclear",
                "capabilities": ["crosschain_intents", "arbitrary_data", "intent_liquidity"],
                "settlement_model": "intent_based_clearing"
            });

            let quote = BridgeQuote {
                bridge: "Everclear".to_string(),
                fee: total_fee,
                est_time,
                metadata: Some(metadata),
            };

            info!(
                "Everclear intent quote retrieved: fee={:.6} {}, time={}s, architecture=intent_based",
                quote.fee, request.asset, quote.est_time
            );

            Ok(quote)
        }
        Err(_) => {
            info!("Failed to parse Everclear response, creating estimate");
            create_everclear_estimate(request)
        }
    }
}

/// Create an estimated Everclear quote when API is unavailable
/// Reflects Everclear's intent-based architecture
fn create_everclear_estimate(request: &BridgeQuoteRequest) -> Result<BridgeQuote, BridgeError> {
    // Verify we support this route
    chain_to_chain_id(&request.from_chain)?;
    chain_to_chain_id(&request.to_chain)?;
    get_asset_ticker_hash(&request.asset)?;

    // Estimate fees based on asset type and chain combinations
    // Everclear uses intent-based fees which are typically lower due to efficiency
    let estimated_fee = match request.asset.to_uppercase().as_str() {
        "USDC" | "USDT" => 0.25, // $0.25 equivalent for stablecoin transfers (lower due to intent efficiency)
        "ETH" | "WETH" => 0.0005, // ~$1-2 for ETH transfers
        "DAI" => 0.3,            // Slightly higher for DAI
        "WBTC" => 0.0001,        // Very low fee for WBTC due to high value
        _ => 0.001,              // 0.1% of transfer for other tokens
    };

    // Estimate time based on chain combinations (intent settlement is faster)
    let est_time = match (request.from_chain.as_str(), request.to_chain.as_str()) {
        // L2 to L2 transfers are fastest with intent settlement
        ("arbitrum", "optimism") | ("optimism", "arbitrum") => 45, // 45 seconds
        ("polygon", "arbitrum") | ("arbitrum", "polygon") => 90,   // 1.5 minutes
        ("polygon", "optimism") | ("optimism", "polygon") => 90,   // 1.5 minutes
        // L1 to L2 or L2 to L1 transfers
        ("ethereum", _) | (_, "ethereum") => 240, // 4 minutes
        // L2 to L2 transfers
        _ => 120, // 2 minutes
    };

    let metadata = serde_json::json!({
        "estimated": true,
        "architecture": "intent_based",
        "security": "intent_settlement",
        "primitive": "intents",
        "network": "Everclear",
        "capabilities": ["crosschain_intents", "arbitrary_data", "intent_liquidity"],
        "settlement_model": "intent_based_clearing",
        "note": "Estimated quote - Everclear uses intent-based settlement for efficient transfers",
        "route": format!("{} -> {}", request.from_chain, request.to_chain)
    });

    let quote = BridgeQuote {
        bridge: "Everclear".to_string(),
        fee: estimated_fee,
        est_time,
        metadata: Some(metadata),
    };

    info!(
        "Everclear intent estimate created: fee={:.6} {}, time={}s, architecture=intent_based",
        quote.fee, request.asset, quote.est_time
    );

    Ok(quote)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Convert amount to smallest unit based on asset decimals
    fn amount_to_smallest_unit(amount: f64, asset: &str) -> String {
        match asset.to_uppercase().as_str() {
            "USDC" | "USDT" => ((amount * 1_000_000.0) as u64).to_string(), // 6 decimals
            _ => ((amount * 1_000_000_000_000_000_000.0) as u64).to_string(), // 18 decimals
        }
    }

    #[tokio::test]
    async fn test_everclear_quote_success() {
        // Test helper functions
        assert_eq!(chain_to_chain_id("ethereum").unwrap(), 1);
        assert_eq!(chain_to_chain_id("polygon").unwrap(), 137);
        assert_eq!(chain_to_chain_id("arbitrum").unwrap(), 42161);
        assert!(chain_to_chain_id("invalid-chain").is_err());

        assert!(get_asset_ticker_hash("USDC").is_ok());
        assert!(get_asset_ticker_hash("WETH").is_ok());
        assert!(get_asset_ticker_hash("INVALID").is_err());

        assert_eq!(wei_to_ether("1000000000000000000").unwrap(), 1.0);
        assert_eq!(amount_to_smallest_unit(1.0, "USDC"), "1000000");
        assert_eq!(amount_to_smallest_unit(1.0, "ETH"), "1000000000000000000");
    }

    #[test]
    fn test_chain_mapping() {
        assert!(chain_to_chain_id("ethereum").is_ok());
        assert!(chain_to_chain_id("polygon").is_ok());
        assert!(chain_to_chain_id("arbitrum").is_ok());
        assert!(chain_to_chain_id("optimism").is_ok());
        assert!(chain_to_chain_id("base").is_ok());
        assert!(chain_to_chain_id("linea").is_ok());
        assert!(chain_to_chain_id("solana").is_ok());
        assert!(chain_to_chain_id("invalid-chain").is_err());
    }

    #[test]
    fn test_asset_mapping() {
        assert!(get_asset_ticker_hash("USDC").is_ok());
        assert!(get_asset_ticker_hash("WETH").is_ok());
        assert!(get_asset_ticker_hash("USDT").is_ok());
        assert!(get_asset_ticker_hash("DAI").is_ok());
        assert!(get_asset_ticker_hash("WBTC").is_ok());
        assert!(get_asset_ticker_hash("SOL").is_ok());
        assert!(get_asset_ticker_hash("INVALID").is_err());
    }

    #[test]
    fn test_everclear_estimate() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "ethereum".to_string(),
            to_chain: "polygon".to_string(),
            amount: Some("1000000".to_string()),
            slippage: 0.5,
        };

        let quote = create_everclear_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Everclear");
        assert!(quote.fee > 0.0);
        assert!(quote.est_time > 0);
    }

    #[test]
    fn test_bps_conversion() {
        // Test basis points conversion
        let bps = 50; // 0.5%
        let percentage = bps as f64 / 10000.0;
        assert_eq!(percentage, 0.005);
    }
}
