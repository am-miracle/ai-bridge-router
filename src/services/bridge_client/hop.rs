use serde::Deserialize;
use tracing::{debug, info, warn};

use super::{format_liquidity, get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

/// Hop Protocol API endpoint
/// Public API for quotes and transfer status
const HOP_API_BASE: &str = "https://api.hop.exchange";

/// Hop API version
const HOP_API_VERSION: &str = "v1";

/// Hop quote response structure from /v1/quote endpoint
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HopQuoteResponse {
    /// Amount in specified in smallest unit
    amount_in: String,
    /// Slippage percentage
    slippage: f64,
    /// Minimum amount out from origin AMM
    amount_out_min: String,
    /// Minimum amount out at destination AMM
    destination_amount_out_min: String,
    /// Bonder fee including destination transaction cost
    bonder_fee: String,
    /// Estimated amount to receive at destination
    /// Note: Hop API has a typo - uses "estimatedRecieved" instead of "estimatedReceived"
    #[serde(rename = "estimatedRecieved")]
    estimated_received: String,
    /// Deadline timestamp for origin swap
    deadline: u64,
    /// Deadline timestamp for destination swap
    destination_deadline: u64,
}

/// Map chain names to Hop chain slugs
/// Supported chains: ethereum, optimism, arbitrum, polygon, gnosis, nova, base, linea, polygonzk
fn map_chain_name(chain: &str) -> Result<String, BridgeError> {
    let hop_chain = match chain.to_lowercase().as_str() {
        "ethereum" | "eth" | "mainnet" => "ethereum",
        "optimism" | "opt" => "optimism",
        "arbitrum" | "arbitrum-one" | "arb" => "arbitrum",
        "polygon" | "matic" => "polygon",
        "gnosis" | "xdai" => "gnosis",
        "nova" | "arbitrum-nova" => "nova",
        "base" => "base",
        "linea" => "linea",
        "polygonzk" | "polygon-zk" | "zkevm" | "polygon-zkevm" => "polygonzk",
        _ => {
            return Err(BridgeError::UnsupportedRoute {
                from_chain: chain.to_string(),
                to_chain: "".to_string(),
            });
        }
    };
    Ok(hop_chain.to_string())
}

/// Map asset symbols to Hop token symbols
fn map_asset_symbol(asset: &str) -> Result<String, BridgeError> {
    match asset.to_uppercase().as_str() {
        "USDC" => Ok("USDC".to_string()),
        "USDC.E" | "USDCE" => Ok("USDC.e".to_string()),
        "USDT" => Ok("USDT".to_string()),
        "DAI" => Ok("DAI".to_string()),
        "ETH" | "WETH" => Ok("ETH".to_string()),
        "MATIC" | "WMATIC" => Ok("MATIC".to_string()),
        "HOP" => Ok("HOP".to_string()),
        "SNX" => Ok("SNX".to_string()),
        "SUSD" => Ok("sUSD".to_string()),
        "RETH" => Ok("rETH".to_string()),
        "MAGIC" => Ok("MAGIC".to_string()),
        _ => Err(BridgeError::UnsupportedAsset {
            asset: asset.to_string(),
        }),
    }
}

/// Get token decimals for amount conversion
fn get_token_decimals(token: &str) -> u32 {
    match token.to_uppercase().as_str() {
        "USDC" | "USDT" => 6, // 6 decimals
        _ => 18,              // Default 18 decimals for ETH and most ERC20
    }
}

/// Convert amount string in smallest unit to human readable
fn parse_amount(amount_str: &str, token: &str) -> Result<f64, BridgeError> {
    let amount = amount_str
        .parse::<u128>()
        .map_err(|_| BridgeError::BadResponse {
            message: format!("Invalid amount: {}", amount_str),
        })?;

    let decimals = get_token_decimals(token);
    let divisor = 10_u128.pow(decimals);

    Ok(amount as f64 / divisor as f64)
}

/// Get a quote from Hop Protocol
pub async fn get_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    let cache_key = format!(
        "hop:{}:{}:{}:{}",
        request.asset,
        request.from_chain,
        request.to_chain,
        request.amount.as_deref().unwrap_or("1000000")
    );

    get_cached_quote(&cache_key, &config.cache, || {
        fetch_hop_quote(request, config)
    })
    .await
}

/// Fetch quote directly from Hop API
async fn fetch_hop_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    retry_request(
        || fetch_hop_quote_once(request, config),
        config.retries,
        "Hop API call",
    )
    .await
}

/// Single attempt to fetch Hop quote using the /v1/quote endpoint
async fn fetch_hop_quote_once(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching Hop quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Map chain names to Hop slugs
    let from_chain = map_chain_name(&request.from_chain)?;
    let to_chain = map_chain_name(&request.to_chain)?;

    // Map asset symbol
    let token = map_asset_symbol(&request.asset)?;

    // Get amount in smallest unit (default to 1 token if not specified)
    let amount = request.amount.clone().unwrap_or_else(|| {
        let decimals = get_token_decimals(&token);
        (10_u128.pow(decimals)).to_string()
    });

    // Use slippage from request (user-provided or default)
    let slippage = request.slippage;

    // Build query parameters for /v1/quote endpoint
    // Network is always mainnet (testnet support can be added via env var if needed)
    let url = format!(
        "{}/{}/quote?amount={}&token={}&fromChain={}&toChain={}&slippage={}",
        HOP_API_BASE, HOP_API_VERSION, amount, token, from_chain, to_chain, slippage
    );

    info!("Requesting Hop quote from: {}", url);

    let response_result = config.client.get(&url).send().await;

    // Handle potential timeout or network error
    let response = match response_result {
        Ok(resp) => resp,
        Err(e) => {
            info!("Hop API network error: {}, creating estimate", e);
            return create_hop_estimate(request);
        }
    };

    let status = response.status();
    if !status.is_success() {
        warn!("Hop API returned {}, creating estimate", status);

        // Log response body for debugging
        if let Ok(body) = response.text().await {
            debug!("Hop API error response: {}", body);
        }

        return create_hop_estimate(request);
    }

    let response_text = response.text().await.map_err(BridgeError::from)?;
    debug!("Hop API response: {}", response_text);

    // Try to parse the response, but fall back to estimate if parsing fails
    match serde_json::from_str::<HopQuoteResponse>(&response_text) {
        Ok(hop_response) => {
            // Parse bonder fee (includes destination tx cost)
            let bonder_fee = parse_amount(&hop_response.bonder_fee, &token)?;

            // Parse estimated received amount
            let estimated_received = parse_amount(&hop_response.estimated_received, &token)?;

            // Calculate total fee as the difference
            let amount_in = parse_amount(&hop_response.amount_in, &token)?;
            let total_fee = amount_in - estimated_received;

            // Estimate time based on route
            // L2 to L2 via Hop is typically fast (few minutes)
            // L1 to L2 or L2 to L1 takes longer
            let est_time = match (request.from_chain.as_str(), request.to_chain.as_str()) {
                // L1 to L2 or L2 to L1 (needs to wait for finality)
                ("ethereum", _) => 1200, // ~20 minutes for L1 to L2
                (_, "ethereum") => 900,  // ~15 minutes for L2 to L1
                // L2 to L2 transfers (fastest)
                _ => 180, // ~3 minutes for L2 to L2
            };

            // Hop typically has good liquidity
            let liquidity = format_liquidity(1_000_000.0, &request.asset);

            let metadata = serde_json::json!({
                "amount_in": hop_response.amount_in,
                "amount_out_min": hop_response.amount_out_min,
                "destination_amount_out_min": hop_response.destination_amount_out_min,
                "bonder_fee": bonder_fee,
                "estimated_received": estimated_received,
                "slippage": hop_response.slippage,
                "deadline": hop_response.deadline,
                "destination_deadline": hop_response.destination_deadline,
                "network": "Hop Protocol",
                "architecture": "rollup_to_rollup_amm",
                "security_model": "optimistic_bridges_with_bonders",
                "route": format!("{} -> {}", request.from_chain, request.to_chain)
            });

            let quote = BridgeQuote {
                bridge: "Hop".to_string(),
                fee: total_fee,
                est_time,
                liquidity,
                score: None,
                metadata: Some(metadata),
            };

            info!(
                "Hop quote retrieved: fee={:.6} {}, bonder_fee={:.6}, time={}s, liquidity={}",
                quote.fee, request.asset, bonder_fee, quote.est_time, quote.liquidity
            );

            Ok(quote)
        }
        Err(e) => {
            info!("Failed to parse Hop response: {}, creating estimate", e);
            create_hop_estimate(request)
        }
    }
}

/// Create an estimated Hop quote when API is unavailable
fn create_hop_estimate(request: &BridgeQuoteRequest) -> Result<BridgeQuote, BridgeError> {
    // Verify we support this route
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    map_asset_symbol(&request.asset)?;

    // Estimate fees based on asset type and route
    let estimated_fee = match request.asset.to_uppercase().as_str() {
        "USDC" | "USDT" => 0.3,   // ~$0.30 for stablecoins
        "DAI" => 0.4,             // ~$0.40 for DAI
        "ETH" | "WETH" => 0.0008, // ~$2-3 for ETH
        "RETH" => 0.0008,         // Similar to ETH
        "MATIC" => 0.5,           // ~$0.50 worth of MATIC
        _ => 0.002,               // 0.2% for other tokens
    };

    // Estimate time based on route complexity
    let est_time = match (request.from_chain.as_str(), request.to_chain.as_str()) {
        // L1 to L2 (slower due to L1 finality)
        ("ethereum", _) => 1200, // ~20 minutes
        // L2 to L1 (needs to wait for challenge period - optimistic rollups)
        (_, "ethereum") => 900, // ~15 minutes
        // L2 to L2 (fastest route via Hop)
        _ => 180, // ~3 minutes
    };

    // Hop typically has good liquidity across popular routes
    let liquidity = format_liquidity(2_000_000.0, &request.asset);

    let metadata = serde_json::json!({
        "estimated": true,
        "network": "Hop Protocol",
        "architecture": "rollup_to_rollup_amm",
        "security_model": "optimistic_bridges_with_bonders",
        "supported_chains": ["ethereum", "optimism", "arbitrum", "polygon", "gnosis", "nova", "base", "linea", "polygonzk"],
        "note": "Estimated quote - Hop uses AMM pools and bonders for fast L2 transfers",
        "route": format!("{} -> {}", request.from_chain, request.to_chain)
    });

    let quote = BridgeQuote {
        bridge: "Hop".to_string(),
        fee: estimated_fee,
        est_time,
        liquidity,
        score: None,
        metadata: Some(metadata),
    };

    info!(
        "Hop estimate created: fee={:.6} {}, time={}s, liquidity={}",
        quote.fee, request.asset, quote.est_time, quote.liquidity
    );

    Ok(quote)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_mapping() {
        assert_eq!(map_chain_name("ethereum").unwrap(), "ethereum");
        assert_eq!(map_chain_name("optimism").unwrap(), "optimism");
        assert_eq!(map_chain_name("arbitrum").unwrap(), "arbitrum");
        assert_eq!(map_chain_name("polygon").unwrap(), "polygon");
        assert_eq!(map_chain_name("gnosis").unwrap(), "gnosis");
        assert_eq!(map_chain_name("nova").unwrap(), "nova");
        assert_eq!(map_chain_name("base").unwrap(), "base");
        assert_eq!(map_chain_name("linea").unwrap(), "linea");
        assert_eq!(map_chain_name("polygonzk").unwrap(), "polygonzk");
        assert_eq!(map_chain_name("polygon-zk").unwrap(), "polygonzk");
        assert_eq!(map_chain_name("zkevm").unwrap(), "polygonzk");
        assert!(map_chain_name("invalid-chain").is_err());
    }

    #[test]
    fn test_asset_mapping() {
        assert_eq!(map_asset_symbol("USDC").unwrap(), "USDC");
        assert_eq!(map_asset_symbol("USDC.E").unwrap(), "USDC.e");
        assert_eq!(map_asset_symbol("USDCE").unwrap(), "USDC.e");
        assert_eq!(map_asset_symbol("ETH").unwrap(), "ETH");
        assert_eq!(map_asset_symbol("WETH").unwrap(), "ETH");
        assert_eq!(map_asset_symbol("DAI").unwrap(), "DAI");
        assert_eq!(map_asset_symbol("MATIC").unwrap(), "MATIC");
        assert_eq!(map_asset_symbol("HOP").unwrap(), "HOP");
        assert_eq!(map_asset_symbol("SNX").unwrap(), "SNX");
        assert_eq!(map_asset_symbol("SUSD").unwrap(), "sUSD");
        assert_eq!(map_asset_symbol("RETH").unwrap(), "rETH");
        assert_eq!(map_asset_symbol("MAGIC").unwrap(), "MAGIC");
        assert!(map_asset_symbol("UNKNOWN").is_err());
    }

    #[test]
    fn test_token_decimals() {
        assert_eq!(get_token_decimals("USDC"), 6);
        assert_eq!(get_token_decimals("USDT"), 6);
        assert_eq!(get_token_decimals("ETH"), 18);
        assert_eq!(get_token_decimals("DAI"), 18);
    }

    #[test]
    fn test_amount_parsing() {
        // 1 USDC (6 decimals)
        assert_eq!(parse_amount("1000000", "USDC").unwrap(), 1.0);
        // 0.5 USDC
        assert_eq!(parse_amount("500000", "USDC").unwrap(), 0.5);
        // 1 ETH (18 decimals)
        assert_eq!(parse_amount("1000000000000000000", "ETH").unwrap(), 1.0);
        // Invalid amount
        assert!(parse_amount("invalid", "USDC").is_err());
    }

    #[test]
    fn test_hop_estimate() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "optimism".to_string(),
            to_chain: "arbitrum".to_string(),
            amount: Some("1000000".to_string()),
            slippage: 0.5,
        };

        let quote = create_hop_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Hop");
        assert!(quote.fee > 0.0);
        assert!(quote.est_time > 0);
        assert!(!quote.liquidity.is_empty());
        assert!(quote.metadata.is_some());
    }

    #[test]
    fn test_unsupported_routes() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "invalid-chain".to_string(),
            to_chain: "arbitrum".to_string(),
            amount: Some("1000000".to_string()),
            slippage: 0.5,
        };

        assert!(create_hop_estimate(&request).is_err());
    }

    #[test]
    fn test_unsupported_assets() {
        let request = BridgeQuoteRequest {
            asset: "UNKNOWN-TOKEN".to_string(),
            from_chain: "optimism".to_string(),
            to_chain: "arbitrum".to_string(),
            amount: Some("1000000".to_string()),
            slippage: 0.5,
        };

        assert!(create_hop_estimate(&request).is_err());
    }
}
