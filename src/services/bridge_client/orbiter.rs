use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::{get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

/// Orbiter API base URL
const ORBITER_API_BASE: &str = "https://api.orbiter.finance";

/// Orbiter quote request payload
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrbiterQuoteRequest {
    source_chain_id: String,
    dest_chain_id: String,
    source_token: String,
    dest_token: String,
    amount: String,
    user_address: String,
    target_recipient: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    slippage: Option<f64>,
}

/// Orbiter quote response
#[derive(Debug, Deserialize)]
struct OrbiterQuoteResponse {
    #[serde(default)]
    status: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    result: Option<OrbiterQuoteResult>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OrbiterQuoteResult {
    #[serde(default)]
    fees: Option<OrbiterFees>,
    #[serde(default)]
    details: Option<OrbiterDetails>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrbiterFees {
    #[serde(default)]
    withholding_fee: Option<String>,
    #[serde(default)]
    withholding_fee_usd: Option<String>,
    #[serde(default)]
    swap_fee: Option<String>,
    #[serde(default)]
    swap_fee_usd: Option<String>,
    #[serde(default)]
    trade_fee: Option<String>,
    #[serde(default)]
    trade_fee_usd: Option<String>,
    #[serde(default)]
    total_fee: Option<String>,
    #[serde(default)]
    fee_symbol: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrbiterDetails {
    #[serde(default)]
    source_token_amount: Option<String>,
    #[serde(default)]
    dest_token_amount: Option<String>,
    #[serde(default)]
    min_dest_token_amount: Option<String>,
}

/// Map chain names to Orbiter chain IDs
fn map_chain_id(chain: &str) -> Result<String, BridgeError> {
    let chain_id = match chain.to_lowercase().as_str() {
        "ethereum" | "eth" | "mainnet" => "1",
        "arbitrum" | "arb" | "arbitrum-one" => "42161",
        "optimism" | "opt" => "10",
        "polygon" | "matic" => "137",
        "zksync" | "zksync-era" => "324",
        "zksync-lite" => "3",
        "starknet" => "SN_MAIN",
        "linea" => "59144",
        "base" => "8453",
        "scroll" => "534352",
        "zora" => "7777777",
        "manta" => "169",
        "mantle" => "5000",
        "loopring" => "LOOPRING",
        "immutable" | "immutablex" => "IMMUTABLE",
        "boba" => "288",
        "metis" => "1088",
        "mode" => "34443",
        "blast" => "81457",
        "lisk" => "1135",
        "redstone" => "690",
        _ => {
            return Err(BridgeError::UnsupportedRoute {
                from_chain: chain.to_string(),
                to_chain: "".to_string(),
            });
        }
    };
    Ok(chain_id.to_string())
}

/// Map chain names for Orbiter Finance (legacy)
fn map_chain_name(chain: &str) -> Result<String, BridgeError> {
    let orbiter_chain = match chain.to_lowercase().as_str() {
        "ethereum" | "eth" | "mainnet" => "ethereum",
        "arbitrum" | "arb" | "arbitrum-one" => "arbitrum",
        "optimism" | "opt" => "optimism",
        "polygon" | "matic" => "polygon",
        "zksync" | "zksync-era" => "zksync",
        "zksync-lite" => "zksync-lite",
        "starknet" => "starknet",
        "linea" => "linea",
        "base" => "base",
        "scroll" => "scroll",
        "zora" => "zora",
        "manta" => "manta",
        "mantle" => "mantle",
        "loopring" => "loopring",
        "immutable" | "immutablex" => "immutablex",
        "boba" => "boba",
        "metis" => "metis",
        "mode" => "mode",
        "blast" => "blast",
        "lisk" => "lisk",
        "redstone" => "redstone",
        _ => {
            return Err(BridgeError::UnsupportedRoute {
                from_chain: chain.to_string(),
                to_chain: "".to_string(),
            });
        }
    };
    Ok(orbiter_chain.to_string())
}

/// Map asset symbols
fn map_asset_symbol(asset: &str) -> Result<String, BridgeError> {
    match asset.to_uppercase().as_str() {
        "ETH" | "WETH" => Ok("ETH".to_string()),
        "USDC" => Ok("USDC".to_string()),
        "USDT" => Ok("USDT".to_string()),
        "DAI" => Ok("DAI".to_string()),
        _ => Err(BridgeError::UnsupportedAsset {
            asset: asset.to_string(),
        }),
    }
}

/// Get token address for Orbiter API
/// Native ETH uses 0x0000000000000000000000000000000000000000
fn get_token_address(asset: &str, _chain: &str) -> Result<String, BridgeError> {
    match asset.to_uppercase().as_str() {
        "ETH" | "WETH" => Ok("0x0000000000000000000000000000000000000000".to_string()),
        // For ERC20 tokens, use simplified addresses (Orbiter handles resolution)
        "USDC" => Ok("USDC".to_string()),
        "USDT" => Ok("USDT".to_string()),
        "DAI" => Ok("DAI".to_string()),
        _ => Err(BridgeError::UnsupportedAsset {
            asset: asset.to_string(),
        }),
    }
}

/// Get token decimals
fn get_token_decimals(token: &str) -> u32 {
    match token.to_uppercase().as_str() {
        "USDC" | "USDT" => 6,
        _ => 18,
    }
}

/// Get a quote from Orbiter Finance
pub async fn get_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    let cache_key = format!(
        "orbiter:{}:{}:{}:{}",
        request.asset,
        request.from_chain,
        request.to_chain,
        request.amount.as_deref().unwrap_or("1000000")
    );

    get_cached_quote(&cache_key, &config.cache, || {
        fetch_orbiter_quote(request, config)
    })
    .await
}

/// Fetch quote from Orbiter Finance
async fn fetch_orbiter_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    retry_request(
        || fetch_orbiter_quote_once(request, config),
        config.retries,
        "Orbiter API call",
    )
    .await
}

/// Single attempt to fetch Orbiter quote
async fn fetch_orbiter_quote_once(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching Orbiter quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Map chains to IDs
    let source_chain_id = map_chain_id(&request.from_chain)?;
    let dest_chain_id = map_chain_id(&request.to_chain)?;

    // Get token addresses
    let source_token = get_token_address(&request.asset, &request.from_chain)?;
    let dest_token = get_token_address(&request.asset, &request.to_chain)?;

    // Get amount with proper decimals
    let decimals = get_token_decimals(&request.asset);
    let amount = request
        .amount
        .clone()
        .unwrap_or_else(|| (10_u128.pow(decimals)).to_string());

    // Use placeholder addresses (Orbiter API requires wallet addresses)
    let user_address = "0xefc6089224068b20197156a91d50132b2a47b908";
    let target_recipient = user_address;

    // Build request payload
    let quote_request = OrbiterQuoteRequest {
        source_chain_id,
        dest_chain_id,
        source_token,
        dest_token,
        amount: amount.clone(),
        user_address: user_address.to_string(),
        target_recipient: target_recipient.to_string(),
        slippage: Some(request.slippage / 100.0), // Convert from percentage
    };

    let url = format!("{}/quote", ORBITER_API_BASE);
    info!("Requesting Orbiter quote from: {}", url);

    // Send POST request
    let response = config.client.post(&url).json(&quote_request).send().await;

    let resp = match response {
        Ok(r) => r,
        Err(e) => {
            info!("Orbiter API network error: {}, creating estimate", e);
            return create_orbiter_estimate(request);
        }
    };

    let status = resp.status();
    if !status.is_success() {
        warn!("Orbiter API returned {}, creating estimate", status);
        return create_orbiter_estimate(request);
    }

    let response_text = resp.text().await.map_err(BridgeError::from)?;
    debug!("Orbiter API response: {}", response_text);

    let quote_response: OrbiterQuoteResponse = match serde_json::from_str(&response_text) {
        Ok(data) => data,
        Err(e) => {
            info!("Failed to parse Orbiter response: {}, creating estimate", e);
            return create_orbiter_estimate(request);
        }
    };

    // Check response status
    if quote_response.status != "success" {
        warn!(
            "Orbiter API returned error: {}, creating estimate",
            quote_response.message
        );
        return create_orbiter_estimate(request);
    }

    let result = match quote_response.result {
        Some(r) => r,
        None => {
            warn!("Orbiter API returned no result, creating estimate");
            return create_orbiter_estimate(request);
        }
    };

    // Extract fees
    let fees = match result.fees {
        Some(f) => f,
        None => {
            warn!("Orbiter API returned no fees, creating estimate");
            return create_orbiter_estimate(request);
        }
    };

    // Calculate fee in token units
    let decimals = get_token_decimals(&request.asset);
    let divisor = 10_f64.powi(decimals as i32);

    // Use total_fee in USD or calculate from withholding fee
    let fee_readable = if let Some(total_fee_usd) = &fees.total_fee {
        total_fee_usd.parse::<f64>().unwrap_or(0.0)
    } else if let Some(withholding_fee) = &fees.withholding_fee {
        let fee_amount = withholding_fee.parse::<f64>().unwrap_or(0.0);
        fee_amount / divisor
    } else {
        0.0
    };

    // Orbiter is very fast for L2 transfers (typically 1-3 minutes)
    let est_time = estimate_orbiter_time(&request.from_chain, &request.to_chain);

    let metadata = serde_json::json!({
        "fees": fees,
        "details": result.details,
        "network": "Orbiter Finance",
        "architecture": "maker_taker_model",
        "security_model": "zkrollup_native_optimized",
        "note": "Real-time quote from Orbiter API",
        "specialization": "L2 rollup bridges"
    });

    let quote = BridgeQuote {
        bridge: "Orbiter".to_string(),
        fee: fee_readable,
        est_time,
        metadata: Some(metadata),
    };

    info!(
        "Orbiter quote retrieved: fee={:.6} {}, time={}s",
        quote.fee, request.asset, quote.est_time
    );

    Ok(quote)
}

/// Estimate transfer time based on chain pair
fn estimate_orbiter_time(from_chain: &str, to_chain: &str) -> u64 {
    let from = from_chain.to_lowercase();
    let to = to_chain.to_lowercase();

    // Check if either chain is L1 Ethereum
    let is_eth_from = from.contains("ethereum") || from.contains("eth") || from == "mainnet";
    let is_eth_to = to.contains("ethereum") || to.contains("eth") || to == "mainnet";

    match (is_eth_from, is_eth_to) {
        // L1 Ethereum involved: Slower (10-15 minutes)
        (true, _) | (_, true) => 900,
        // L2 to L2: Very fast (1-3 minutes) - Orbiter's specialty
        (false, false) => 120,
    }
}

/// Create estimated Orbiter quote
fn create_orbiter_estimate(request: &BridgeQuoteRequest) -> Result<BridgeQuote, BridgeError> {
    // Verify route is supported
    map_chain_id(&request.from_chain)?;
    map_chain_id(&request.to_chain)?;
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

    // Orbiter is known for very low fees (maker-taker model)
    // Withholding fee + trading fee (typically 0.05-0.10%)
    let (fee_percentage, base_gas_cost) = match request.asset.to_uppercase().as_str() {
        "USDC" | "USDT" => (0.0005, 0.05),  // 0.05% + ~$0.05 base
        "ETH" | "WETH" => (0.0005, 0.0001), // 0.05% + ~$0.30 gas (in ETH)
        "DAI" => (0.0006, 0.08),            // 0.06% + ~$0.08 base
        _ => (0.001, 0.5),                  // 0.1% + base estimate
    };

    let estimated_fee = (amount_readable * fee_percentage) + base_gas_cost;

    // Orbiter timing - specialized for fast L2 transfers
    let est_time = estimate_orbiter_time(&request.from_chain, &request.to_chain);

    let metadata = serde_json::json!({
        "estimated": true,
        "fee_percentage": fee_percentage,
        "base_gas_cost": base_gas_cost,
        "amount": amount_readable,
        "network": "Orbiter Finance",
        "architecture": "maker_taker_model",
        "security_model": "zkrollup_native_optimized",
        "supported_chains": ["ethereum", "arbitrum", "optimism", "polygon", "zksync", "starknet", "linea", "base", "scroll"],
        "note": "Estimated quote (API unavailable) - Orbiter specializes in fast L2 transfers",
        "route": format!("{} -> {}", request.from_chain, request.to_chain),
        "typical_time": format!("{}-{} minutes", est_time / 60 - 1, est_time / 60 + 1),
        "fee_formula": format!("{}% + {} {} base fee", fee_percentage * 100.0, base_gas_cost, request.asset),
        "specialization": "L2 rollup bridges"
    });

    let quote = BridgeQuote {
        bridge: "Orbiter".to_string(),
        fee: estimated_fee,
        est_time,
        metadata: Some(metadata),
    };

    info!(
        "Orbiter estimate created: fee={:.6} {}, time={}s",
        quote.fee, request.asset, quote.est_time
    );

    Ok(quote)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_mapping() {
        assert_eq!(map_chain_name("ethereum").unwrap(), "ethereum");
        assert_eq!(map_chain_name("arbitrum").unwrap(), "arbitrum");
        assert_eq!(map_chain_name("zksync").unwrap(), "zksync");
        assert_eq!(map_chain_name("starknet").unwrap(), "starknet");
        assert!(map_chain_name("invalid").is_err());
    }

    #[test]
    fn test_orbiter_estimate() {
        let request = BridgeQuoteRequest {
            asset: "ETH".to_string(),
            from_chain: "arbitrum".to_string(),
            to_chain: "optimism".to_string(),
            amount: Some("1000000000000000000".to_string()), // 1 ETH
            slippage: 0.5,
        };

        let quote = create_orbiter_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Orbiter");
        assert!(quote.fee > 0.0);
        // L2 to L2 should be very fast (120 seconds = 2 minutes)
        assert_eq!(quote.est_time, 120);
        // Fee should be very low for 1 ETH (0.05% + $0.30 = ~0.0006 ETH)
        assert!(quote.fee > 0.0001 && quote.fee < 0.01);
    }

    #[test]
    fn test_orbiter_time_estimates() {
        // Ethereum routes should be slower
        assert_eq!(estimate_orbiter_time("ethereum", "arbitrum"), 900);
        assert_eq!(estimate_orbiter_time("optimism", "ethereum"), 900);

        // L2 to L2 should be very fast
        assert_eq!(estimate_orbiter_time("arbitrum", "optimism"), 120);
        assert_eq!(estimate_orbiter_time("zksync", "linea"), 120);
    }

    #[test]
    fn test_chain_id_mapping() {
        assert_eq!(map_chain_id("ethereum").unwrap(), "1");
        assert_eq!(map_chain_id("arbitrum").unwrap(), "42161");
        assert_eq!(map_chain_id("optimism").unwrap(), "10");
        assert_eq!(map_chain_id("zksync").unwrap(), "324");
        assert!(map_chain_id("invalid").is_err());
    }

    #[test]
    fn test_token_decimals() {
        assert_eq!(get_token_decimals("USDC"), 6);
        assert_eq!(get_token_decimals("USDT"), 6);
        assert_eq!(get_token_decimals("ETH"), 18);
    }
}
