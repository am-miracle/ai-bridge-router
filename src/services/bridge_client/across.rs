use serde::Deserialize;
use tracing::{debug, info, warn};

use super::{get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

/// Across Protocol API
const ACROSS_API_BASE: &str = "https://app.across.to/api";

/// Across suggested fees response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AcrossSuggestedFeesResponse {
    /// Total relay fee (LP fee + relayer fee)
    total_relay_fee: AcrossFee,
    /// Capital fee charged by LPs
    capital_fee_percent: String,
    /// Relayer gas fee
    #[allow(dead_code)]
    relayer_gas_fee: AcrossFee,
    /// Timestamp when quote expires
    #[serde(default)]
    #[allow(dead_code)]
    timestamp: Option<u64>,
    /// Whether the route is supported
    #[serde(default)]
    is_amount_too_low: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AcrossFee {
    /// Fee amount as string (in wei or smallest unit)
    #[serde(default)]
    #[allow(dead_code)]
    total: Option<String>,
    /// Percentage as string
    #[serde(default)]
    pct: Option<String>,
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

    // Get token address
    let token = get_token_address(&request.asset, &request.from_chain)?;

    // Get amount or use default
    let amount = request.amount.clone().unwrap_or_else(|| {
        let decimals = get_token_decimals(&request.asset);
        (10_u128.pow(decimals)).to_string()
    });

    // Build API URL for suggested fees
    let url = format!(
        "{}/suggested-fees?originChainId={}&destinationChainId={}&token={}&amount={}",
        ACROSS_API_BASE, origin_chain_id, destination_chain_id, token, amount
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
            if across_response.is_amount_too_low.unwrap_or(false) {
                return Err(BridgeError::BadResponse {
                    message: "Amount too low for Across Protocol".to_string(),
                });
            }

            // Parse total relay fee
            let fee_pct_str = across_response
                .total_relay_fee
                .pct
                .unwrap_or_else(|| "0.1".to_string());
            let fee_pct: f64 = fee_pct_str.parse().unwrap_or(0.001);

            // Convert to actual fee based on amount
            let amount_f64 = request
                .amount
                .as_ref()
                .and_then(|a| a.parse::<f64>().ok())
                .unwrap_or(1_000_000.0);

            let decimals = get_token_decimals(&request.asset);
            let divisor = 10_f64.powi(decimals as i32);
            let amount_readable = amount_f64 / divisor;

            let total_fee = amount_readable * fee_pct;

            // Across is typically very fast (optimistic approach)
            // Origin to destination: usually 1-4 minutes
            let est_time = 240; // ~4 minutes average

            let metadata = serde_json::json!({
                "total_relay_fee_pct": fee_pct,
                "capital_fee_percent": across_response.capital_fee_percent,
                "network": "Across Protocol",
                "architecture": "optimistic_bridging_intent_based",
                "security_model": "uma_optimistic_oracle",
                "route": format!("{} -> {}", request.from_chain, request.to_chain),
                "note": "Across uses optimistic validation for fast transfers"
            });

            let quote = BridgeQuote {
                bridge: "Across".to_string(),
                fee: total_fee,
                est_time,
                metadata: Some(metadata),
            };

            info!(
                "Across quote retrieved: fee={:.6} {} ({:.4}%), time={}s",
                quote.fee,
                request.asset,
                fee_pct * 100.0,
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

    // Across typical fees: 0.05-0.15% + gas
    let estimated_fee = match request.asset.to_uppercase().as_str() {
        "USDC" | "USDT" => 0.2,   // ~$0.20
        "ETH" | "WETH" => 0.0004, // ~$1.20
        "DAI" => 0.25,            // ~$0.25
        "WBTC" => 0.000015,       // ~$0.75
        _ => 0.001,               // 0.1%
    };

    // Across is one of the fastest bridges (optimistic)
    let est_time = 240; // ~4 minutes

    let metadata = serde_json::json!({
        "estimated": true,
        "network": "Across Protocol",
        "architecture": "optimistic_bridging_intent_based",
        "security_model": "uma_optimistic_oracle",
        "supported_chains": ["ethereum", "optimism", "polygon", "arbitrum", "base", "linea", "blast"],
        "note": "Estimated quote - Across uses optimistic validation for fast bridging",
        "route": format!("{} -> {}", request.from_chain, request.to_chain),
        "typical_time": "1-4 minutes",
        "fees": "0.05-0.15% + gas"
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
            amount: Some("1000000".to_string()),
            slippage: 0.5,
        };

        let quote = create_across_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Across");
        assert!(quote.fee > 0.0);
        assert_eq!(quote.est_time, 240);
    }
}
