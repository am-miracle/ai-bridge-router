use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::{get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

/// cBridge API base URL
const CBRIDGE_API_BASE: &str = "https://cbridge-prod2.celer.app/v2";

/// cBridge estimateAmt response
#[derive(Debug, Deserialize, Serialize)]
struct CbridgeEstimateResponse {
    #[serde(default)]
    err: Option<CbridgeError>,
    /// Equivalent value token amount in destination chain
    #[serde(default)]
    eq_value_token_amt: Option<String>,
    /// Bridge rate between source and destination chain
    #[serde(default)]
    bridge_rate: Option<f64>,
    /// Protocol fee percentage
    #[serde(default)]
    perc_fee: Option<String>,
    /// Base fee
    #[serde(default)]
    base_fee: Option<String>,
    /// Estimated receiving amount on destination chain
    #[serde(default)]
    estimated_receive_amt: Option<String>,
    /// Maximum slippage
    #[serde(default)]
    max_slippage: Option<u64>,
    /// Drop gas amount
    #[serde(default)]
    drop_gas_amt: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CbridgeError {
    #[serde(default)]
    code: Option<i32>,
    #[serde(default)]
    msg: Option<String>,
}

/// cBridge transfer latency response
#[derive(Debug, Deserialize)]
struct CbridgeLatencyResponse {
    #[serde(default)]
    #[allow(dead_code)]
    err: Option<CbridgeError>,
    /// Median transfer latency in seconds
    #[serde(default)]
    median_transfer_latency_in_second: Option<f64>,
}

/// Map chain names to cBridge chain IDs
fn map_chain_name(chain: &str) -> Result<u64, BridgeError> {
    let chain_id = match chain.to_lowercase().as_str() {
        "ethereum" | "eth" | "mainnet" => 1,
        "arbitrum" | "arb" | "arbitrum-one" => 42161,
        "optimism" | "opt" => 10,
        "polygon" | "matic" => 137,
        "avalanche" | "avax" => 43114,
        "bsc" | "binance" | "bnb" => 56,
        "fantom" | "ftm" => 250,
        "moonriver" => 1285,
        "moonbeam" => 1284,
        "aurora" => 1313161554,
        "harmony" => 1666600000,
        "celo" => 42220,
        "metis" => 1088,
        "base" => 8453,
        "scroll" => 534352,
        "linea" => 59144,
        "mantle" => 5000,
        _ => {
            return Err(BridgeError::UnsupportedRoute {
                from_chain: chain.to_string(),
                to_chain: "".to_string(),
            });
        }
    };
    Ok(chain_id)
}

/// Map asset symbols
fn map_asset_symbol(asset: &str) -> Result<String, BridgeError> {
    match asset.to_uppercase().as_str() {
        "USDC" => Ok("USDC".to_string()),
        "USDT" => Ok("USDT".to_string()),
        "ETH" | "WETH" => Ok("WETH".to_string()),
        "DAI" => Ok("DAI".to_string()),
        "BUSD" => Ok("BUSD".to_string()),
        "WBTC" => Ok("WBTC".to_string()),
        "CELR" => Ok("CELR".to_string()),
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
/// cBridge timing heavily depends on chain finality requirements
fn estimate_cbridge_time(from_chain: &str, to_chain: &str) -> u64 {
    let from = from_chain.to_lowercase();
    let to = to_chain.to_lowercase();

    // Ethereum requires significant finality (12+ confirmations)
    let is_eth_from = from.contains("ethereum") || from.contains("eth") || from == "mainnet";
    let is_eth_to = to.contains("ethereum") || to.contains("eth") || to == "mainnet";

    // BSC also requires more finality
    let is_bsc_from = from.contains("bsc") || from.contains("binance") || from.contains("bnb");
    let is_bsc_to = to.contains("bsc") || to.contains("binance") || to.contains("bnb");

    match (is_eth_from || is_bsc_from, is_eth_to || is_bsc_to) {
        // Ethereum or BSC involved: Slower (15-20 minutes)
        (true, _) | (_, true) => 1200,
        // L2 to L2 or fast chains: Moderate (5-8 minutes)
        (false, false) => 360,
    }
}

/// Get a quote from Celer cBridge
pub async fn get_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    let cache_key = format!(
        "cbridge:{}:{}:{}:{}",
        request.asset,
        request.from_chain,
        request.to_chain,
        request.amount.as_deref().unwrap_or("1000000")
    );

    get_cached_quote(&cache_key, &config.cache, || {
        fetch_cbridge_quote(request, config)
    })
    .await
}

/// Fetch quote from cBridge
async fn fetch_cbridge_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    retry_request(
        || fetch_cbridge_quote_once(request, config),
        config.retries,
        "cBridge API call",
    )
    .await
}

/// Single attempt to fetch cBridge quote
async fn fetch_cbridge_quote_once(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching cBridge quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Map chains and assets
    let src_chain_id = map_chain_name(&request.from_chain)?;
    let dst_chain_id = map_chain_name(&request.to_chain)?;
    let token_symbol = map_asset_symbol(&request.asset)?;

    // Get amount with proper decimals
    let decimals = get_token_decimals(&request.asset);
    let amount = request
        .amount
        .clone()
        .unwrap_or_else(|| (10_u128.pow(decimals)).to_string());

    // Convert slippage to cBridge format (0.5% -> 5000)
    let slippage_tolerance = (request.slippage * 10000.0) as u64;

    // Build estimateAmt API URL
    let estimate_url = format!(
        "{}/estimateAmt?src_chain_id={}&dst_chain_id={}&token_symbol={}&amt={}&slippage_tolerance={}",
        CBRIDGE_API_BASE, src_chain_id, dst_chain_id, token_symbol, amount, slippage_tolerance
    );

    info!("Requesting cBridge estimate from: {}", estimate_url);

    // Fetch estimate
    let estimate_response = config.client.get(&estimate_url).send().await;

    let estimate_resp = match estimate_response {
        Ok(resp) => resp,
        Err(e) => {
            info!("cBridge API network error: {}, creating estimate", e);
            return create_cbridge_estimate(request);
        }
    };

    let status = estimate_resp.status();
    if !status.is_success() {
        warn!(
            "cBridge estimateAmt API returned {}, creating estimate",
            status
        );
        return create_cbridge_estimate(request);
    }

    let response_text = estimate_resp.text().await.map_err(BridgeError::from)?;
    debug!("cBridge estimateAmt response: {}", response_text);

    let estimate_data: CbridgeEstimateResponse = match serde_json::from_str(&response_text) {
        Ok(data) => data,
        Err(e) => {
            info!("Failed to parse cBridge response: {}, creating estimate", e);
            return create_cbridge_estimate(request);
        }
    };

    // Check for API error
    if let Some(err) = &estimate_data.err
        && let Some(code) = err.code
    {
        let msg = err.msg.as_deref().unwrap_or("Unknown error");
        warn!("cBridge API error {}: {}", code, msg);
        return Err(BridgeError::BadResponse {
            message: format!("cBridge error {}: {}", code, msg),
        });
    }

    // Fetch transfer latency
    let latency_url = format!(
        "{}/getLatest7DayTransferLatencyForQuery?src_chain_id={}&dst_chain_id={}",
        CBRIDGE_API_BASE, src_chain_id, dst_chain_id
    );

    let est_time = match config.client.get(&latency_url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.text().await {
            Ok(text) => match serde_json::from_str::<CbridgeLatencyResponse>(&text) {
                Ok(latency_data) => latency_data
                    .median_transfer_latency_in_second
                    .map(|t| t as u64)
                    .unwrap_or_else(|| {
                        estimate_cbridge_time(&request.from_chain, &request.to_chain)
                    }),
                Err(_) => estimate_cbridge_time(&request.from_chain, &request.to_chain),
            },
            Err(_) => estimate_cbridge_time(&request.from_chain, &request.to_chain),
        },
        _ => estimate_cbridge_time(&request.from_chain, &request.to_chain),
    };

    // Calculate fee from response
    let divisor = 10_f64.powi(decimals as i32);
    let amount_f64 = amount
        .parse::<f64>()
        .unwrap_or(10_f64.powi(decimals as i32));
    let amount_readable = amount_f64 / divisor;

    let estimated_receive = estimate_data
        .estimated_receive_amt
        .as_ref()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(amount_f64);

    let estimated_receive_readable = estimated_receive / divisor;
    let fee_readable = amount_readable - estimated_receive_readable;

    let metadata = serde_json::json!({
        "eq_value_token_amt": estimate_data.eq_value_token_amt,
        "bridge_rate": estimate_data.bridge_rate,
        "perc_fee": estimate_data.perc_fee,
        "base_fee": estimate_data.base_fee,
        "estimated_receive_amt": estimate_data.estimated_receive_amt,
        "max_slippage": estimate_data.max_slippage,
        "drop_gas_amt": estimate_data.drop_gas_amt,
        "network": "Celer cBridge",
        "architecture": "state_guardian_network",
        "security_model": "sgn_pos_with_optimistic_rollup",
        "route": format!("{} -> {}", request.from_chain, request.to_chain),
        "note": "Real-time quote from cBridge API"
    });

    let quote = BridgeQuote {
        bridge: "Celer cBridge".to_string(),
        fee: fee_readable,
        est_time,
        metadata: Some(metadata),
    };

    info!(
        "cBridge quote retrieved: fee={:.6} {}, time={}s",
        quote.fee, request.asset, quote.est_time
    );

    Ok(quote)
}

/// Create estimated cBridge quote
fn create_cbridge_estimate(request: &BridgeQuoteRequest) -> Result<BridgeQuote, BridgeError> {
    // Verify route is supported
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    map_asset_symbol(&request.asset)?;

    // Calculate fee based on amount (percentage-based)
    let decimals = get_token_decimals(&request.asset);
    let divisor = 10_f64.powi(decimals as i32);

    let amount_f64 = request
        .amount
        .as_ref()
        .and_then(|a| a.parse::<f64>().ok())
        .unwrap_or(10_f64.powi(decimals as i32));

    let amount_readable = amount_f64 / divisor;

    // cBridge fees: 0.04% base fee + liquidity provider fee (~0.1% total)
    // Plus base fee which varies by token
    let (fee_percentage, base_gas_cost) = match request.asset.to_uppercase().as_str() {
        "USDC" | "USDT" => (0.001, 0.08),  // 0.1% + ~$0.08 base fee
        "ETH" | "WETH" => (0.001, 0.0002), // 0.1% + ~$0.60 gas (in ETH)
        "DAI" | "BUSD" => (0.001, 0.10),   // 0.1% + ~$0.10 base fee
        "WBTC" => (0.0012, 0.000008),      // 0.12% + ~$0.40 base fee (in BTC)
        "CELR" => (0.002, 5.0),            // 0.2% + 5 CELR base fee
        _ => (0.0015, 0.5),                // 0.15% + base estimate
    };

    let estimated_fee = (amount_readable * fee_percentage) + base_gas_cost;

    // cBridge timing depends on chain finality
    let est_time = estimate_cbridge_time(&request.from_chain, &request.to_chain);

    let metadata = serde_json::json!({
        "estimated": true,
        "fee_percentage": fee_percentage,
        "base_gas_cost": base_gas_cost,
        "amount": amount_readable,
        "network": "Celer cBridge",
        "architecture": "state_guardian_network",
        "security_model": "sgn_pos_with_optimistic_rollup",
        "supported_chains": ["ethereum", "bsc", "arbitrum", "optimism", "polygon", "avalanche", "fantom", "base", "scroll"],
        "note": "Estimated quote (API unavailable) - Calculated using typical cBridge fees",
        "route": format!("{} -> {}", request.from_chain, request.to_chain),
        "typical_time": format!("{}-{} minutes", est_time / 60 - 2, est_time / 60 + 2),
        "fee_formula": format!("{}% + {} {} base fee", fee_percentage * 100.0, base_gas_cost, request.asset),
        "liquidity": "Deep liquidity pools"
    });

    let quote = BridgeQuote {
        bridge: "Celer cBridge".to_string(),
        fee: estimated_fee,
        est_time,
        metadata: Some(metadata),
    };

    info!(
        "cBridge estimate created: fee={:.6} {}, time={}s",
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
        assert_eq!(map_chain_name("arbitrum").unwrap(), 42161);
        assert_eq!(map_chain_name("polygon").unwrap(), 137);
        assert_eq!(map_chain_name("base").unwrap(), 8453);
        assert!(map_chain_name("invalid").is_err());
    }

    #[test]
    fn test_asset_mapping() {
        assert_eq!(map_asset_symbol("USDC").unwrap(), "USDC");
        assert_eq!(map_asset_symbol("ETH").unwrap(), "WETH");
        assert_eq!(map_asset_symbol("CELR").unwrap(), "CELR");
        assert!(map_asset_symbol("UNKNOWN").is_err());
    }

    #[test]
    fn test_cbridge_estimate() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "ethereum".to_string(),
            to_chain: "polygon".to_string(),
            amount: Some("1000000".to_string()), // 1 USDC
            slippage: 0.5,
        };

        let quote = create_cbridge_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Celer cBridge");
        assert!(quote.fee > 0.0);
        // Ethereum to L2 should be slower (1200 seconds = 20 minutes)
        assert_eq!(quote.est_time, 1200);
        // Fee should be reasonable for 1 USDC (0.1% + $0.08 = ~$0.09)
        assert!(quote.fee > 0.08 && quote.fee < 0.15);
    }

    #[test]
    fn test_cbridge_time_estimates() {
        // Ethereum routes should be slower
        assert_eq!(estimate_cbridge_time("ethereum", "polygon"), 1200);
        assert_eq!(estimate_cbridge_time("arbitrum", "ethereum"), 1200);
        assert_eq!(estimate_cbridge_time("bsc", "arbitrum"), 1200);

        // L2 to L2 should be faster
        assert_eq!(estimate_cbridge_time("arbitrum", "polygon"), 360);
        assert_eq!(estimate_cbridge_time("optimism", "base"), 360);
    }

    #[test]
    fn test_token_decimals() {
        assert_eq!(get_token_decimals("USDC"), 6);
        assert_eq!(get_token_decimals("WBTC"), 8);
        assert_eq!(get_token_decimals("ETH"), 18);
    }
}
