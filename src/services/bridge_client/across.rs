use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::{get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

/// Across Protocol API
const ACROSS_API_BASE: &str = "https://app.across.to/api";

/// Across suggested fees response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AcrossSuggestedFeesResponse {
    /// Estimated fill time in seconds
    #[serde(default)]
    estimated_fill_time_sec: Option<u64>,
    /// Total relay fee percentage
    #[serde(default)]
    relay_fee_pct: Option<String>,
    /// Total relay fee amount
    #[serde(default)]
    relay_fee_total: Option<String>,
    /// LP fee percentage
    #[serde(default)]
    lp_fee_pct: Option<String>,
    /// Capital fee percentage
    #[serde(default)]
    capital_fee_pct: Option<String>,
    /// Relayer gas fee percentage
    #[allow(dead_code)]
    #[serde(default)]
    relayer_gas_fee_pct: Option<String>,
    /// Output amount after fees
    #[serde(default)]
    output_amount: Option<String>,
    /// Timestamp when quote expires
    #[allow(dead_code)]
    #[serde(default)]
    timestamp: Option<String>,
    /// Whether the route is supported
    #[serde(default)]
    is_amount_too_low: bool,
    /// Limits information
    #[serde(default)]
    limits: Option<AcrossLimits>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AcrossLimits {
    #[serde(default)]
    min_deposit: Option<String>,
    #[serde(default)]
    max_deposit: Option<String>,
    #[serde(default)]
    max_deposit_instant: Option<String>,
}

/// Map chain names to Across chain IDs
fn map_chain_name(chain: &str) -> Result<u64, BridgeError> {
    let chain_id = match chain.to_lowercase().as_str() {
        "ethereum" | "eth" | "mainnet" => 1,
        "optimism" | "opt" => 10,
        "polygon" | "matic" => 137,
        "arbitrum" | "arb" | "arbitrum-one" => 42161,
        "base" => 8453,
        "linea" => 59144,
        "mode" => 34443,
        "zksync" | "zksync-era" => 324,
        "blast" => 81457,
        "lisk" => 1135,
        "scroll" => 534352,
        "redstone" => 690,
        "zora" => 7777777,
        "world chain" | "wc" | "world-chain" => 480,
        "ink" => 57073,
        "soneium" => 1868,
        "unichain" => 130,
        "lens" => 232,
        "bnb-smart-chain" | "bnb" => 56,
        "solana" => 34268394551451,
        "hyper-evm" => 999,
        "plasma" => 9745,
        "hyper-core" => 1337,
        _ => {
            return Err(BridgeError::UnsupportedRoute {
                from_chain: chain.to_string(),
                to_chain: "".to_string(),
            });
        }
    };
    Ok(chain_id)
}

/// Get token address for Across (uses contract addresses)
fn get_token_address(asset: &str, chain: &str) -> Result<String, BridgeError> {
    // Across uses specific token addresses per chain
    // Simplified mapping - in production, use full token registry
    match (asset.to_uppercase().as_str(), chain.to_lowercase().as_str()) {
        // USDC addresses
        ("USDC", "ethereum") => Ok("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string()),
        ("USDC", "arbitrum") => Ok("0xaf88d065e77c8cC2239327C5EDb3A432268e5831".to_string()),
        ("USDC", "optimism") => Ok("0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85".to_string()),
        ("USDC", "polygon") => Ok("0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359".to_string()),
        ("USDC", "base") => Ok("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string()),

        // WETH addresses
        ("ETH" | "WETH", "ethereum") => {
            Ok("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string())
        }
        ("ETH" | "WETH", "arbitrum") => {
            Ok("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1".to_string())
        }
        ("ETH" | "WETH", "optimism") => {
            Ok("0x4200000000000000000000000000000000000006".to_string())
        }
        ("ETH" | "WETH", "polygon") => Ok("0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619".to_string()),
        ("ETH" | "WETH", "base") => Ok("0x4200000000000000000000000000000000000006".to_string()),

        // DAI
        ("DAI", "ethereum") => Ok("0x6B175474E89094C44Da98b954EedeAC495271d0F".to_string()),
        ("DAI", "arbitrum") => Ok("0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1".to_string()),
        ("DAI", "optimism") => Ok("0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1".to_string()),
        ("DAI", "polygon") => Ok("0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063".to_string()),

        // WBTC
        ("WBTC", "ethereum") => Ok("0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599".to_string()),
        ("WBTC", "arbitrum") => Ok("0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f".to_string()),
        ("WBTC", "optimism") => Ok("0x68f180fcCe6836688e9084f035309E29Bf0A2095".to_string()),

        _ => Err(BridgeError::UnsupportedAsset {
            asset: asset.to_string(),
        }),
    }
}

/// Get token decimals
fn get_token_decimals(token: &str) -> u32 {
    match token.to_uppercase().as_str() {
        "USDC" | "USDT" => 6,
        "WBTC" => 8,
        _ => 18,
    }
}

/// Estimate transfer time based on chain pair
/// Across uses optimistic bridging with different speeds per route
fn estimate_across_time(from_chain: &str, to_chain: &str) -> u64 {
    let from = from_chain.to_lowercase();
    let to = to_chain.to_lowercase();

    // L2 to L2 transfers are fastest (1-3 minutes)
    let l2_chains = [
        "optimism", "arbitrum", "base", "polygon", "linea", "blast", "mode", "zksync",
    ];
    let from_is_l2 = l2_chains.iter().any(|&l2| from.contains(l2));
    let to_is_l2 = l2_chains.iter().any(|&l2| to.contains(l2));

    match (from_is_l2, to_is_l2) {
        // L2 to L2: Very fast (1-2 minutes)
        (true, true) => 90,
        // L1 to L2 or L2 to L1: Fast (2-4 minutes)
        (true, false) | (false, true) => 180,
        // L1 to L1: Moderate (3-5 minutes)
        (false, false) => 240,
    }
}

/// Get a quote from Across Protocol
pub async fn get_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    let cache_key = format!(
        "across:{}:{}:{}:{}",
        request.asset,
        request.from_chain,
        request.to_chain,
        request.amount.as_deref().unwrap_or("1000000")
    );

    get_cached_quote(&cache_key, &config.cache, || {
        fetch_across_quote(request, config)
    })
    .await
}

/// Fetch quote from Across API
async fn fetch_across_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    retry_request(
        || fetch_across_quote_once(request, config),
        config.retries,
        "Across API call",
    )
    .await
}

/// Single attempt to fetch Across quote
async fn fetch_across_quote_once(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching Across quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Map chains
    let origin_chain_id = map_chain_name(&request.from_chain)?;
    let destination_chain_id = map_chain_name(&request.to_chain)?;

    // Get token addresses for both origin and destination chains
    let input_token = get_token_address(&request.asset, &request.from_chain)?;
    let output_token = get_token_address(&request.asset, &request.to_chain)?;

    // Get amount or use default
    let amount = request.amount.clone().unwrap_or_else(|| {
        let decimals = get_token_decimals(&request.asset);
        (10_u128.pow(decimals)).to_string()
    });

    // Build API URL for suggested fees
    let url = format!(
        "{}/suggested-fees?inputToken={}&outputToken={}&originChainId={}&destinationChainId={}&amount={}",
        ACROSS_API_BASE, input_token, output_token, origin_chain_id, destination_chain_id, amount
    );

    info!("Requesting Across quote from: {}", url);

    let response_result = config.client.get(&url).send().await;

    let response = match response_result {
        Ok(resp) => resp,
        Err(e) => {
            info!("Across API network error: {}, creating estimate", e);
            return create_across_estimate(request);
        }
    };

    let status = response.status();
    if !status.is_success() {
        warn!("Across API returned {}, creating estimate", status);
        return create_across_estimate(request);
    }

    let response_text = response.text().await.map_err(BridgeError::from)?;
    debug!("Across API response: {}", response_text);

    match serde_json::from_str::<AcrossSuggestedFeesResponse>(&response_text) {
        Ok(across_response) => {
            // Check if amount is too low
            if across_response.is_amount_too_low {
                return Err(BridgeError::BadResponse {
                    message: "Amount too low for Across Protocol".to_string(),
                });
            }

            // Parse amounts and calculate fee
            let decimals = get_token_decimals(&request.asset);
            let divisor = 10_f64.powi(decimals as i32);

            // Get input amount
            let input_amount = request
                .amount
                .as_ref()
                .and_then(|a| a.parse::<f64>().ok())
                .unwrap_or(1_000_000.0);

            let input_readable = input_amount / divisor;

            // Calculate fee from output amount if available
            let fee_readable = if let Some(output_str) = &across_response.output_amount {
                let output_amount = output_str.parse::<f64>().unwrap_or(input_amount);
                let output_readable = output_amount / divisor;
                input_readable - output_readable
            } else {
                // Fallback to percentage calculation
                let fee_pct_str = across_response.relay_fee_pct.as_deref().unwrap_or("0");
                // Fee percentage is in basis points (e.g., "78905024308003" represents a very small percentage)
                // Need to divide by 1e18 to get actual percentage
                let fee_pct = fee_pct_str.parse::<f64>().unwrap_or(0.0) / 1e18;
                input_readable * fee_pct
            };

            // Use estimated fill time from API or calculate based on route
            let est_time = across_response
                .estimated_fill_time_sec
                .unwrap_or_else(|| estimate_across_time(&request.from_chain, &request.to_chain));

            // Extract fee percentage for metadata
            let relay_fee_pct = across_response
                .relay_fee_pct
                .as_deref()
                .and_then(|s| s.parse::<f64>().ok())
                .map(|pct| pct / 1e18)
                .unwrap_or(0.0);

            let metadata = serde_json::json!({
                "relay_fee_pct": relay_fee_pct,
                "relay_fee_total": across_response.relay_fee_total,
                "capital_fee_pct": across_response.capital_fee_pct,
                "lp_fee_pct": across_response.lp_fee_pct,
                "estimated_fill_time_sec": across_response.estimated_fill_time_sec,
                "output_amount": across_response.output_amount,
                "limits": across_response.limits,
                "network": "Across Protocol",
                "architecture": "optimistic_bridging_intent_based",
                "security_model": "uma_optimistic_oracle",
                "route": format!("{} -> {}", request.from_chain, request.to_chain),
                "note": "Across uses optimistic validation for fast transfers"
            });

            let quote = BridgeQuote {
                bridge: "Across".to_string(),
                fee: fee_readable,
                est_time,
                metadata: Some(metadata),
            };

            info!(
                "Across quote retrieved: fee={:.6} {} ({:.4}%), time={}s",
                quote.fee,
                request.asset,
                relay_fee_pct * 100.0,
                quote.est_time
            );

            Ok(quote)
        }
        Err(e) => {
            info!("Failed to parse Across response: {}, creating estimate", e);
            create_across_estimate(request)
        }
    }
}

/// Create estimated Across quote
fn create_across_estimate(request: &BridgeQuoteRequest) -> Result<BridgeQuote, BridgeError> {
    // Verify route is supported
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    get_token_address(&request.asset, &request.from_chain)?;

    // Calculate fee based on amount (percentage-based)
    let decimals = get_token_decimals(&request.asset);
    let divisor = 10_f64.powi(decimals as i32);

    let amount_f64 = request
        .amount
        .as_ref()
        .and_then(|a| a.parse::<f64>().ok())
        .unwrap_or(10_f64.powi(decimals as i32));

    let amount_readable = amount_f64 / divisor;

    // Across typical fees: 0.08-0.15% (relay fee) + gas costs
    // Gas costs vary by chain and token
    let (fee_percentage, base_gas_cost) = match request.asset.to_uppercase().as_str() {
        "USDC" | "USDT" => (0.0012, 0.15),  // 0.12% + ~$0.15 gas
        "ETH" | "WETH" => (0.0010, 0.0003), // 0.10% + ~$0.90 gas (in ETH)
        "DAI" => (0.0012, 0.20),            // 0.12% + ~$0.20 gas
        "WBTC" => (0.0015, 0.000012),       // 0.15% + ~$0.60 gas (in BTC)
        _ => (0.0015, 1.0),                 // 0.15% + base gas estimate
    };

    let estimated_fee = (amount_readable * fee_percentage) + base_gas_cost;

    // Time estimates based on chain pair and typical congestion
    // Across is very fast (optimistic bridging), but varies by route
    let est_time = estimate_across_time(&request.from_chain, &request.to_chain);

    let metadata = serde_json::json!({
        "estimated": true,
        "fee_percentage": fee_percentage,
        "base_gas_cost": base_gas_cost,
        "amount": amount_readable,
        "network": "Across Protocol",
        "architecture": "optimistic_bridging_intent_based",
        "security_model": "uma_optimistic_oracle",
        "supported_chains": ["ethereum", "optimism", "polygon", "arbitrum", "base", "linea", "blast"],
        "note": "Estimated quote (API unavailable) - Calculated using typical Across fees",
        "route": format!("{} -> {}", request.from_chain, request.to_chain),
        "typical_time": format!("{}-{} minutes", est_time / 60 - 1, est_time / 60 + 1),
        "fee_formula": format!("{}% + {} {} gas", fee_percentage * 100.0, base_gas_cost, request.asset)
    });

    let quote = BridgeQuote {
        bridge: "Across".to_string(),
        fee: estimated_fee,
        est_time,
        metadata: Some(metadata),
    };

    info!(
        "Across estimate created: fee={:.6} {}, time={}s",
        quote.fee, request.asset, quote.est_time
    );

    Ok(quote)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_mapping() {
        assert_eq!(map_chain_name("ethereum").unwrap(), 1);
        assert_eq!(map_chain_name("optimism").unwrap(), 10);
        assert_eq!(map_chain_name("polygon").unwrap(), 137);
        assert_eq!(map_chain_name("arbitrum").unwrap(), 42161);
        assert_eq!(map_chain_name("base").unwrap(), 8453);
        assert!(map_chain_name("invalid-chain").is_err());
    }

    #[test]
    fn test_token_decimals() {
        assert_eq!(get_token_decimals("USDC"), 6);
        assert_eq!(get_token_decimals("WBTC"), 8);
        assert_eq!(get_token_decimals("ETH"), 18);
    }

    #[test]
    fn test_across_estimate() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "ethereum".to_string(),
            to_chain: "arbitrum".to_string(),
            amount: Some("1000000".to_string()), // 1 USDC
            slippage: 0.5,
        };

        let quote = create_across_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Across");
        assert!(quote.fee > 0.0);
        // Ethereum (L1) to Arbitrum (L2) should be 180 seconds
        assert_eq!(quote.est_time, 180);
        // Fee should be reasonable for 1 USDC (0.12% + $0.15 = ~$0.162)
        assert!(quote.fee > 0.15 && quote.fee < 0.25);
    }

    #[test]
    fn test_across_time_estimates() {
        // L2 to L2 should be fastest
        assert_eq!(estimate_across_time("optimism", "arbitrum"), 90);
        assert_eq!(estimate_across_time("base", "polygon"), 90);

        // L1 to L2 or L2 to L1 should be fast
        assert_eq!(estimate_across_time("ethereum", "arbitrum"), 180);
        assert_eq!(estimate_across_time("optimism", "ethereum"), 180);

        // L1 to L1 should be moderate
        assert_eq!(estimate_across_time("ethereum", "ethereum"), 240);
    }
}
