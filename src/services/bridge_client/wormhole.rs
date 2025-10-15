use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::{get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

/// Wormholescan API base URLs
const WORMHOLESCAN_API_BASE: &str = "https://api.wormholescan.io/api/v1";

/// NTT Token response from Wormholescan API
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NTTToken {
    symbol: String,
    coingecko_id: String,
    price: Option<String>,
    volume_24h: Option<String>,
    platforms: std::collections::HashMap<String, String>,
}

/// NTT Token detail response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NTTTokenDetail {
    home: NTTHome,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NTTHome {
    blockchain: String,
    #[serde(rename = "wormholeChainId")]
    wormhole_chain_id: u16,
    mode: String,
    token: NTTTokenInfo,
    manager: NTTManager,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NTTTokenInfo {
    address: String,
    name: String,
    symbol: String,
    decimals: u8,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NTTManager {
    address: String,
    version: String,
}

/// Transfer operation from Wormholescan
#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
struct Operation {
    id: String,
    #[serde(rename = "emitterChain")]
    emitter_chain: u16,
    content: OperationContent,
    #[serde(rename = "sourceChain")]
    source_chain: Option<ChainInfo>,
    #[serde(rename = "targetChain")]
    target_chain: Option<ChainInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
struct OperationContent {
    #[serde(rename = "standarizedProperties")]
    standardized_properties: StandardizedProperties,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
struct StandardizedProperties {
    #[serde(rename = "fromChain")]
    from_chain: u16,
    #[serde(rename = "toChain")]
    to_chain: u16,
    amount: String,
    fee: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
struct ChainInfo {
    #[serde(rename = "chainId")]
    chain_id: u16,
    timestamp: String,
    status: String,
    fee: Option<String>,
    #[serde(rename = "feeUSD")]
    fee_usd: Option<String>,
}

/// Map chain names to Wormhole chain IDs
fn map_chain_name(chain: &str) -> Result<u16, BridgeError> {
    let chain_id = match chain.to_lowercase().as_str() {
        "ethereum" | "eth" | "mainnet" => 2,
        "bsc" | "binance" | "bnb" => 4,
        "polygon" | "matic" => 5,
        "avalanche" | "avax" => 6,
        "fantom" | "ftm" => 10,
        "celo" => 14,
        "moonbeam" | "glmr" => 16,
        "arbitrum" | "arb" => 23,
        "optimism" | "opt" => 24,
        "base" => 30,
        "sei" => 32,
        "scroll" => 34,
        _ => {
            return Err(BridgeError::UnsupportedRoute {
                from_chain: chain.to_string(),
                to_chain: "".to_string(),
            });
        }
    };
    Ok(chain_id)
}

/// Get token address for Wormhole (uses contract addresses)
fn get_token_address(asset: &str, chain: &str) -> Result<String, BridgeError> {
    // Wormhole token addresses
    match (asset.to_uppercase().as_str(), chain.to_lowercase().as_str()) {
        // USDC addresses
        ("USDC", "ethereum") => Ok("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string()),
        ("USDC", "arbitrum") => Ok("0xaf88d065e77c8cC2239327C5EDb3A432268e5831".to_string()),
        ("USDC", "optimism") => Ok("0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85".to_string()),
        ("USDC", "polygon") => Ok("0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359".to_string()),
        ("USDC", "base") => Ok("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string()),
        ("USDC", "avalanche") => Ok("0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E".to_string()),
        ("USDC", "bsc") => Ok("0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d".to_string()),

        // USDT addresses
        ("USDT", "ethereum") => Ok("0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string()),
        ("USDT", "arbitrum") => Ok("0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9".to_string()),
        ("USDT", "bsc") => Ok("0x55d398326f99059fF775485246999027B3197955".to_string()),
        ("USDT", "polygon") => Ok("0xc2132D05D31c914a87C6611C10748AEb04B58e8F".to_string()),

        // ETH/WETH addresses
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

        // DAI addresses
        ("DAI", "ethereum") => Ok("0x6B175474E89094C44Da98b954EedeAC495271d0F".to_string()),
        ("DAI", "arbitrum") => Ok("0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1".to_string()),
        ("DAI", "optimism") => Ok("0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1".to_string()),
        ("DAI", "polygon") => Ok("0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063".to_string()),

        // WBTC addresses
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

/// Get a quote from Wormhole
pub async fn get_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    let cache_key = format!(
        "wormhole:{}:{}:{}:{}",
        request.asset,
        request.from_chain,
        request.to_chain,
        request.amount.as_deref().unwrap_or("1000000")
    );

    get_cached_quote(&cache_key, &config.cache, || {
        fetch_wormhole_quote(request, config)
    })
    .await
}

/// Fetch quote from Wormhole API
async fn fetch_wormhole_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    retry_request(
        || fetch_wormhole_quote_once(request, config),
        config.retries,
        "Wormhole API call",
    )
    .await
}

/// Single attempt to fetch Wormhole quote
async fn fetch_wormhole_quote_once(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching Wormhole quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Map chains
    let from_chain_id = map_chain_name(&request.from_chain)?;
    let to_chain_id = map_chain_name(&request.to_chain)?;

    // Get token address
    let token_address = get_token_address(&request.asset, &request.from_chain)?;

    // Try to fetch NTT token information from Wormholescan
    let url = format!(
        "{}/ntt/token/{}/{}",
        WORMHOLESCAN_API_BASE, from_chain_id, token_address
    );

    info!("Requesting Wormhole NTT info from: {}", url);

    let response_result = config.client.get(&url).send().await;

    let response = match response_result {
        Ok(resp) => resp,
        Err(e) => {
            info!("Wormholescan API network error: {}, creating estimate", e);
            return create_wormhole_estimate(request);
        }
    };

    let status = response.status();
    if !status.is_success() {
        warn!("Wormholescan API returned {}, creating estimate", status);
        return create_wormhole_estimate(request);
    }

    let response_text = response.text().await.map_err(BridgeError::from)?;
    debug!("Wormholescan API response: {}", response_text);

    // Parse NTT token detail
    match serde_json::from_str::<NTTTokenDetail>(&response_text) {
        Ok(ntt_detail) => {
            info!(
                "Found NTT token: {} ({}) on {}",
                ntt_detail.home.token.name,
                ntt_detail.home.token.symbol,
                ntt_detail.home.blockchain
            );

            // Try to get recent operations to estimate fees
            let operations_url = format!(
                "{}/operations?fromChain={}&toChain={}&pageSize=5",
                WORMHOLESCAN_API_BASE, from_chain_id, to_chain_id
            );

            info!("Fetching recent operations from: {}", operations_url);

            let avg_fee = match config.client.get(&operations_url).send().await {
                Ok(ops_resp) if ops_resp.status().is_success() => {
                    let ops_text = ops_resp.text().await.ok();
                    if let Some(text) = ops_text {
                        // Parse operations and calculate average fee
                        if let Ok(ops) = serde_json::from_str::<serde_json::Value>(&text) {
                            if let Some(operations) =
                                ops.get("operations").and_then(|v| v.as_array())
                            {
                                let fees: Vec<f64> = operations
                                    .iter()
                                    .filter_map(|op| {
                                        op.get("sourceChain")
                                            .and_then(|sc| sc.get("feeUSD"))
                                            .and_then(|f| f.as_str())
                                            .and_then(|s| s.parse::<f64>().ok())
                                    })
                                    .collect();

                                if !fees.is_empty() {
                                    Some(fees.iter().sum::<f64>() / fees.len() as f64)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            };

            let fee = avg_fee.unwrap_or_else(|| {
                // Fallback to estimate
                match request.asset.to_uppercase().as_str() {
                    "USDC" | "USDT" => 0.25,
                    "WETH" | "ETH" => 0.0005,
                    "WBTC" => 0.00002,
                    _ => 0.0015,
                }
            });

            // Estimate time based on chain pair
            let est_time = match (request.from_chain.as_str(), request.to_chain.as_str()) {
                ("ethereum", _) => 900, // L1 to anywhere: ~15 mins
                (_, "ethereum") => 900, // Anywhere to L1: ~15 mins
                _ => 600,               // L2 to L2: ~10 mins
            };

            let metadata = serde_json::json!({
                "ntt_token": ntt_detail.home.token.name,
                "ntt_mode": ntt_detail.home.mode,
                "manager_address": ntt_detail.home.manager.address,
                "manager_version": ntt_detail.home.manager.version,
                "decimals": ntt_detail.home.token.decimals,
                "wormhole_chain_id": ntt_detail.home.wormhole_chain_id,
                "network": "Wormhole",
                "architecture": "guardian_network_with_ntt",
                "security_model": "19_guardian_multisig",
                "route": format!("{} -> {}", request.from_chain, request.to_chain),
                "note": "Real-time data from Wormholescan API",
                "avg_fee_from_recent_ops": avg_fee.is_some()
            });

            let quote = BridgeQuote {
                bridge: "Wormhole".to_string(),
                fee,
                est_time,
                metadata: Some(metadata),
            };

            info!(
                "Wormhole quote retrieved: fee={:.6} {}, time={}s",
                quote.fee, request.asset, quote.est_time
            );

            Ok(quote)
        }
        Err(e) => {
            info!(
                "Failed to parse Wormholescan response: {}, creating estimate",
                e
            );
            create_wormhole_estimate(request)
        }
    }
}

/// Create estimated Wormhole quote
fn create_wormhole_estimate(request: &BridgeQuoteRequest) -> Result<BridgeQuote, BridgeError> {
    // Verify route is supported
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    get_token_address(&request.asset, &request.from_chain)?;

    // Calculate fee based on amount (fixed relayer fee model)
    let decimals = get_token_decimals(&request.asset);
    let divisor = 10_f64.powi(decimals as i32);

    let amount_f64 = request
        .amount
        .as_ref()
        .and_then(|a| a.parse::<f64>().ok())
        .unwrap_or(10_f64.powi(decimals as i32));

    let amount_readable = amount_f64 / divisor;

    // Wormhole fees: fixed relayer fee + gas costs (not percentage-based)
    // Fees are typically fixed per transaction
    let (base_fee, gas_cost) = match request.asset.to_uppercase().as_str() {
        "USDC" | "USDT" => (0.15, 0.10),    // ~$0.15 base + $0.10 gas
        "WETH" | "ETH" => (0.0002, 0.0003), // Fixed in ETH terms
        "WBTC" => (0.000015, 0.000005),     // Fixed in BTC terms
        _ => (0.20, 0.15),                  // Default fixed fees
    };

    let estimated_fee = base_fee + gas_cost;

    info!(
        "Wormhole estimate for {} {}: base_fee={}, gas={}, total={}",
        amount_readable, request.asset, base_fee, gas_cost, estimated_fee
    );

    // Wormhole uses guardian network - relatively fast
    // L1 to L1: 15-30 minutes (wait for finality)
    // L2 to L2: 5-10 minutes
    let est_time = match (request.from_chain.as_str(), request.to_chain.as_str()) {
        ("ethereum", _) => 900, // L1 to anywhere: ~15 mins
        (_, "ethereum") => 900, // Anywhere to L1: ~15 mins
        _ => 600,               // L2 to L2 or others: ~10 mins
    };

    let metadata = serde_json::json!({
        "estimated": true,
        "base_fee": base_fee,
        "gas_cost": gas_cost,
        "amount": amount_readable,
        "network": "Wormhole",
        "architecture": "guardian_network",
        "security_model": "19_guardian_multisig",
        "supported_chains": ["ethereum", "bsc", "polygon", "avalanche", "fantom", "arbitrum", "optimism", "base", "scroll"],
        "note": "Estimated quote (API unavailable) - Wormhole uses guardian network for cross-chain messaging",
        "route": format!("{} -> {}", request.from_chain, request.to_chain),
        "tvl": "Multi-billion dollar bridge",
        "fee_structure": format!("Fixed: {} + {} gas", base_fee, gas_cost)
    });

    let quote = BridgeQuote {
        bridge: "Wormhole".to_string(),
        fee: estimated_fee,
        est_time,
        metadata: Some(metadata),
    };

    info!(
        "Wormhole estimate created: fee={:.6} {}, time={}s",
        quote.fee, request.asset, quote.est_time
    );

    Ok(quote)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_mapping() {
        assert_eq!(map_chain_name("ethereum").unwrap(), 2);
        assert_eq!(map_chain_name("polygon").unwrap(), 5);
        assert_eq!(map_chain_name("arbitrum").unwrap(), 23);
        assert_eq!(map_chain_name("optimism").unwrap(), 24);
        assert_eq!(map_chain_name("base").unwrap(), 30);
        assert_eq!(map_chain_name("scroll").unwrap(), 34);
        assert!(map_chain_name("invalid-chain").is_err());
    }

    #[test]
    fn test_token_address_mapping() {
        // USDC addresses
        assert_eq!(
            get_token_address("USDC", "ethereum").unwrap(),
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
        );
        assert_eq!(
            get_token_address("USDC", "arbitrum").unwrap(),
            "0xaf88d065e77c8cC2239327C5EDb3A432268e5831"
        );

        // ETH addresses
        assert_eq!(
            get_token_address("ETH", "ethereum").unwrap(),
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
        );

        // Unsupported combinations
        assert!(get_token_address("UNKNOWN", "ethereum").is_err());
    }

    #[test]
    fn test_token_decimals() {
        assert_eq!(get_token_decimals("USDC"), 6);
        assert_eq!(get_token_decimals("USDT"), 6);
        assert_eq!(get_token_decimals("WBTC"), 8);
        assert_eq!(get_token_decimals("ETH"), 18);
        assert_eq!(get_token_decimals("DAI"), 18);
    }

    #[test]
    fn test_wormhole_estimate() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "ethereum".to_string(),
            to_chain: "polygon".to_string(),
            amount: Some("1000000".to_string()), // 1 USDC
            slippage: 0.5,
        };

        let quote = create_wormhole_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Wormhole");
        assert!(quote.fee > 0.0);
        // Fee should be ~$0.25 (0.15 base + 0.10 gas)
        assert!(quote.fee >= 0.20 && quote.fee <= 0.30);
        // Ethereum to Polygon should be ~15 minutes (900s)
        assert_eq!(quote.est_time, 900);
    }

    #[test]
    fn test_wormhole_estimate_l2_to_l2() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "arbitrum".to_string(),
            to_chain: "optimism".to_string(),
            amount: Some("10000000".to_string()), // 10 USDC
            slippage: 0.5,
        };

        let quote = create_wormhole_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Wormhole");
        // L2 to L2 should be faster (~10 minutes)
        assert_eq!(quote.est_time, 600);
    }
}
