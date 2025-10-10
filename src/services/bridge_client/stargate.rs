use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::{get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

/// Stargate API base URL
const STARGATE_API_BASE: &str = "https://stargate.finance/api/v1";

/// Stargate quotes response
#[derive(Debug, Deserialize)]
struct StargateQuotesResponse {
    #[serde(default)]
    quotes: Vec<StargateQuote>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StargateQuote {
    #[serde(default)]
    route: Option<String>,
    #[serde(default)]
    error: Option<String>,
    /// Source amount
    #[serde(default)]
    src_amount: Option<String>,
    /// Destination amount after fees
    #[serde(default)]
    dst_amount: Option<String>,
    /// Duration estimation
    #[serde(default)]
    duration: Option<StargateDuration>,
    /// Fees breakdown
    #[serde(default)]
    fees: Vec<StargateFee>,
}

#[derive(Debug, Deserialize, Serialize)]
struct StargateDuration {
    /// Estimated duration in seconds
    #[serde(default)]
    estimated: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StargateFee {
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    chain_key: Option<String>,
    #[serde(default)]
    amount: Option<String>,
    #[serde(default)]
    r#type: Option<String>,
}

/// Map chain names to Stargate API chain keys
fn map_chain_key(chain: &str) -> Result<&'static str, BridgeError> {
    match chain.to_lowercase().as_str() {
        "ethereum" | "eth" | "mainnet" => Ok("ethereum"),
        "bsc" | "binance" | "bnb" | "bnb-smart-chain" => Ok("bsc"),
        "avalanche" | "avax" => Ok("avalanche"),
        "polygon" | "matic" => Ok("polygon"),
        "arbitrum" | "arb" | "arbitrum-one" => Ok("arbitrum"),
        "optimism" | "opt" => Ok("optimism"),
        "fantom" | "ftm" => Ok("fantom"),
        "metis" => Ok("metis"),
        "kava" => Ok("kava"),
        "mantle" => Ok("mantle"),
        "linea" => Ok("linea"),
        "base" => Ok("base"),
        "scroll" => Ok("scroll"),
        "zksync" | "zksync-era" => Ok("zksync"),
        _ => Err(BridgeError::UnsupportedRoute {
            from_chain: chain.to_string(),
            to_chain: "".to_string(),
        }),
    }
}

/// Map asset symbols
fn map_asset_symbol(asset: &str) -> Result<String, BridgeError> {
    match asset.to_uppercase().as_str() {
        "USDC" => Ok("USDC".to_string()),
        "USDT" => Ok("USDT".to_string()),
        "ETH" | "WETH" => Ok("ETH".to_string()),
        "USDD" => Ok("USDD".to_string()),
        "DAI" => Ok("DAI".to_string()),
        "FRAX" => Ok("FRAX".to_string()),
        "MAI" => Ok("MAI".to_string()),
        "LUSD" => Ok("LUSD".to_string()),
        "METIS" => Ok("METIS".to_string()),
        _ => Err(BridgeError::UnsupportedAsset {
            asset: asset.to_string(),
        }),
    }
}

/// Get token address for Stargate API
/// Use special address 0xEeee... for native ETH
fn get_token_address(asset: &str, _chain: &str) -> Result<String, BridgeError> {
    match asset.to_uppercase().as_str() {
        "ETH" | "WETH" => Ok("0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE".to_string()),
        // For ERC20 tokens, would need chain-specific addresses
        // Simplified for now - Stargate API will handle token resolution
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

/// Estimate transfer time based on chain pair
/// Stargate timing depends on finality requirements
fn estimate_stargate_time(from_chain: &str, to_chain: &str) -> u64 {
    let from = from_chain.to_lowercase();
    let to = to_chain.to_lowercase();

    // Ethereum requires significant finality
    let is_eth_from = from.contains("ethereum") || from.contains("eth") || from == "mainnet";
    let is_eth_to = to.contains("ethereum") || to.contains("eth") || to == "mainnet";

    match (is_eth_from, is_eth_to) {
        // Ethereum involved: Slower (8-12 minutes)
        (true, _) | (_, true) => 600,
        // L2 to L2 or fast chains: Fast (2-4 minutes)
        (false, false) => 180,
    }
}

/// Get a quote from Stargate
pub async fn get_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    let cache_key = format!(
        "stargate:{}:{}:{}:{}",
        request.asset,
        request.from_chain,
        request.to_chain,
        request.amount.as_deref().unwrap_or("1000000")
    );

    get_cached_quote(&cache_key, &config.cache, || {
        fetch_stargate_quote(request, config)
    })
    .await
}

/// Fetch quote from Stargate API
async fn fetch_stargate_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    retry_request(
        || fetch_stargate_quote_once(request, config),
        config.retries,
        "Stargate API call",
    )
    .await
}

/// Single attempt to fetch Stargate quote
async fn fetch_stargate_quote_once(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching Stargate quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Map chains to API keys
    let src_chain_key = map_chain_key(&request.from_chain)?;
    let dst_chain_key = map_chain_key(&request.to_chain)?;

    // Get token addresses
    let src_token = get_token_address(&request.asset, &request.from_chain)?;
    let dst_token = get_token_address(&request.asset, &request.to_chain)?;

    // Get amount with proper decimals
    let decimals = get_token_decimals(&request.asset);
    let src_amount = request
        .amount
        .clone()
        .unwrap_or_else(|| (10_u128.pow(decimals)).to_string());

    // Calculate minimum destination amount (apply slippage)
    let src_amount_f64 = src_amount
        .parse::<f64>()
        .unwrap_or(10_f64.powi(decimals as i32));
    let dst_amount_min = (src_amount_f64 * (1.0 - request.slippage / 100.0)) as u128;

    // Use placeholder addresses (Stargate API requires wallet addresses)
    let src_address = "0x1234567890abcdef1234567890abcdef12345678";
    let dst_address = "0xabcdef1234567890abcdef1234567890abcdef12";

    // Build API URL
    let url = format!(
        "{}/quotes?srcToken={}&srcChainKey={}&dstToken={}&dstChainKey={}&srcAddress={}&dstAddress={}&srcAmount={}&dstAmountMin={}",
        STARGATE_API_BASE,
        src_token,
        src_chain_key,
        dst_token,
        dst_chain_key,
        src_address,
        dst_address,
        src_amount,
        dst_amount_min
    );

    info!("Requesting Stargate quote from: {}", url);

    // Fetch quotes
    let response = config.client.get(&url).send().await;

    let resp = match response {
        Ok(r) => r,
        Err(e) => {
            info!("Stargate API network error: {}, creating estimate", e);
            return create_stargate_estimate(request);
        }
    };

    let status = resp.status();
    if !status.is_success() {
        warn!("Stargate API returned {}, creating estimate", status);
        return create_stargate_estimate(request);
    }

    let response_text = resp.text().await.map_err(BridgeError::from)?;
    debug!("Stargate API response: {}", response_text);

    let quotes_response: StargateQuotesResponse = match serde_json::from_str(&response_text) {
        Ok(data) => data,
        Err(e) => {
            info!(
                "Failed to parse Stargate response: {}, creating estimate",
                e
            );
            return create_stargate_estimate(request);
        }
    };

    // Get the best quote (usually "taxi" route is fastest)
    let best_quote = quotes_response
        .quotes
        .into_iter()
        .filter(|q| q.error.is_none())
        .min_by(|a, b| {
            let a_time = a
                .duration
                .as_ref()
                .and_then(|d| d.estimated)
                .unwrap_or(f64::MAX);
            let b_time = b
                .duration
                .as_ref()
                .and_then(|d| d.estimated)
                .unwrap_or(f64::MAX);
            a_time.partial_cmp(&b_time).unwrap()
        });

    let quote_data = match best_quote {
        Some(q) => q,
        None => {
            warn!("No valid quotes from Stargate API, creating estimate");
            return create_stargate_estimate(request);
        }
    };

    // Calculate fee from src and dst amounts
    let divisor = 10_f64.powi(decimals as i32);
    let src_readable = src_amount_f64 / divisor;

    let dst_amount_str = quote_data.dst_amount.as_deref().unwrap_or(&src_amount);
    let dst_amount_f64 = dst_amount_str.parse::<f64>().unwrap_or(src_amount_f64);
    let dst_readable = dst_amount_f64 / divisor;

    let fee_readable = src_readable - dst_readable;

    // Get estimated time
    let est_time = quote_data
        .duration
        .as_ref()
        .and_then(|d| d.estimated)
        .unwrap_or(300.0) as u64;

    let metadata = serde_json::json!({
        "route": quote_data.route,
        "src_amount": quote_data.src_amount,
        "dst_amount": quote_data.dst_amount,
        "duration_seconds": est_time,
        "fees": quote_data.fees,
        "network": "Stargate Finance",
        "architecture": "layerzero_v2_omnichain",
        "security_model": "unified_liquidity_pools",
        "route_type": quote_data.route.as_deref().unwrap_or("unknown"),
        "note": "Real-time quote from Stargate API"
    });

    let quote = BridgeQuote {
        bridge: "Stargate".to_string(),
        fee: fee_readable,
        est_time,
        metadata: Some(metadata),
    };

    info!(
        "Stargate quote retrieved: fee={:.6} {}, time={}s, route={}",
        quote.fee,
        request.asset,
        quote.est_time,
        quote_data.route.as_deref().unwrap_or("unknown")
    );

    Ok(quote)
}

/// Create estimated Stargate quote
fn create_stargate_estimate(request: &BridgeQuoteRequest) -> Result<BridgeQuote, BridgeError> {
    // Verify route is supported
    map_chain_key(&request.from_chain)?;
    map_chain_key(&request.to_chain)?;
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

    // Stargate uses unified liquidity pools
    // Fees: 0.06% transfer fee + LayerZero messaging fee
    let (fee_percentage, messaging_cost) = match request.asset.to_uppercase().as_str() {
        "USDC" | "USDT" => (0.0006, 0.12),  // 0.06% + ~$0.12 LayerZero fee
        "ETH" | "WETH" => (0.0006, 0.0003), // 0.06% + ~$0.90 LayerZero fee (in ETH)
        "DAI" | "FRAX" | "LUSD" | "MAI" => (0.0008, 0.15), // 0.08% + ~$0.15
        _ => (0.001, 0.5),                  // 0.1% + base estimate
    };

    let estimated_fee = (amount_readable * fee_percentage) + messaging_cost;

    // Stargate timing depends on chain finality (LayerZero V2)
    let est_time = estimate_stargate_time(&request.from_chain, &request.to_chain);

    let metadata = serde_json::json!({
        "estimated": true,
        "fee_percentage": fee_percentage,
        "messaging_cost": messaging_cost,
        "amount": amount_readable,
        "network": "Stargate Finance",
        "architecture": "layerzero_v2_omnichain",
        "security_model": "unified_liquidity_pools",
        "supported_chains": ["ethereum", "bsc", "avalanche", "polygon", "arbitrum", "optimism", "fantom", "base", "linea", "metis"],
        "note": "Estimated quote (API unavailable) - Calculated using typical Stargate fees",
        "route": format!("{} -> {}", request.from_chain, request.to_chain),
        "typical_time": format!("{}-{} minutes", est_time / 60 - 2, est_time / 60 + 2),
        "fee_formula": format!("{}% + {} {} LayerZero fee", fee_percentage * 100.0, messaging_cost, request.asset),
        "tvl": "~$300M+ liquidity"
    });

    let quote = BridgeQuote {
        bridge: "Stargate".to_string(),
        fee: estimated_fee,
        est_time,
        metadata: Some(metadata),
    };

    info!(
        "Stargate estimate created: fee={:.6} {}, time={}s",
        quote.fee, request.asset, quote.est_time
    );

    Ok(quote)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_mapping() {
        assert_eq!(map_asset_symbol("USDC").unwrap(), "USDC");
        assert_eq!(map_asset_symbol("ETH").unwrap(), "ETH");
        assert_eq!(map_asset_symbol("USDT").unwrap(), "USDT");
        assert!(map_asset_symbol("UNKNOWN").is_err());
    }

    #[test]
    fn test_stargate_estimate() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "ethereum".to_string(),
            to_chain: "polygon".to_string(),
            amount: Some("1000000".to_string()), // 1 USDC
            slippage: 0.5,
        };

        let quote = create_stargate_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Stargate");
        assert!(quote.fee > 0.0);
        // Ethereum to L2 should be slower (600 seconds = 10 minutes)
        assert_eq!(quote.est_time, 600);
        // Fee should be reasonable for 1 USDC (0.06% + $0.12 = ~$0.1206)
        assert!(quote.fee > 0.12 && quote.fee < 0.20);
    }

    #[test]
    fn test_stargate_time_estimates() {
        // Ethereum routes should be slower
        assert_eq!(estimate_stargate_time("ethereum", "polygon"), 600);
        assert_eq!(estimate_stargate_time("arbitrum", "ethereum"), 600);

        // L2 to L2 should be faster
        assert_eq!(estimate_stargate_time("arbitrum", "polygon"), 180);
        assert_eq!(estimate_stargate_time("optimism", "base"), 180);
    }

    #[test]
    fn test_token_decimals() {
        assert_eq!(get_token_decimals("USDC"), 6);
        assert_eq!(get_token_decimals("USDT"), 6);
        assert_eq!(get_token_decimals("ETH"), 18);
    }

    #[test]
    fn test_chain_keys() {
        assert_eq!(map_chain_key("ethereum").unwrap(), "ethereum");
        assert_eq!(map_chain_key("arbitrum").unwrap(), "arbitrum");
        assert_eq!(map_chain_key("polygon").unwrap(), "polygon");
        assert!(map_chain_key("invalid-chain").is_err());
    }
}
