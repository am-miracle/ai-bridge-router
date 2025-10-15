use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::{get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

/// Synapse Protocol API base URL
const SYNAPSE_API_BASE: &str = "https://api.synapseprotocol.com";

/// Synapse bridge quote response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SynapseBridgeQuoteResponse {
    id: String,
    from_chain_id: u64,
    to_chain_id: u64,
    expected_to_amount: String,
    min_to_amount: String,
    router_address: String,
    estimated_time: u64,
    module_names: Vec<String>,
    gas_drop_amount: String,
    #[serde(default)]
    call_data: Option<SynapseCallData>,
}

#[derive(Debug, Deserialize, Serialize)]
struct SynapseCallData {
    to: String,
    data: String,
    value: String,
}

/// Map chain names to Synapse chain IDs
fn map_chain_name(chain: &str) -> Result<u64, BridgeError> {
    let chain_id = match chain.to_lowercase().as_str() {
        "ethereum" | "eth" | "mainnet" => 1,
        "arbitrum" | "arb" | "arbitrum-one" => 42161,
        "optimism" | "opt" => 10,
        "polygon" | "matic" => 137,
        "avalanche" | "avax" => 43114,
        "bsc" | "binance" | "bnb" => 56,
        "fantom" | "ftm" => 250,
        "aurora" => 1313161554,
        "harmony" => 1666600000,
        "boba" => 288,
        "moonbeam" => 1284,
        "moonriver" => 1285,
        "cronos" => 25,
        "metis" => 1088,
        "dfk" | "defikingdoms" => 53935,
        "klaytn" => 8217,
        "base" => 8453,
        "blast" => 81457,
        "scroll" => 534352,
        _ => {
            return Err(BridgeError::UnsupportedRoute {
                from_chain: chain.to_string(),
                to_chain: "".to_string(),
            });
        }
    };
    Ok(chain_id)
}

/// Get token address for Synapse (uses contract addresses)
fn get_token_address(asset: &str, chain: &str) -> Result<String, BridgeError> {
    // Synapse uses specific token addresses per chain
    match (asset.to_uppercase().as_str(), chain.to_lowercase().as_str()) {
        // USDC addresses
        ("USDC", "ethereum") => Ok("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string()),
        ("USDC", "arbitrum") => Ok("0xaf88d065e77c8cC2239327C5EDb3A432268e5831".to_string()),
        ("USDC", "optimism") => Ok("0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85".to_string()),
        ("USDC", "polygon") => Ok("0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359".to_string()),
        ("USDC", "base") => Ok("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string()),
        ("USDC", "avalanche") => Ok("0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E".to_string()),
        ("USDC", "bsc") => Ok("0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d".to_string()),
        ("USDC", "blast") => Ok("0x4300000000000000000000000000000000000003".to_string()),

        // USDT addresses
        ("USDT", "ethereum") => Ok("0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string()),
        ("USDT", "arbitrum") => Ok("0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9".to_string()),
        ("USDT", "optimism") => Ok("0x94b008aA00579c1307B0EF2c499aD98a8ce58e58".to_string()),
        ("USDT", "polygon") => Ok("0xc2132D05D31c914a87C6611C10748AEb04B58e8F".to_string()),
        ("USDT", "bsc") => Ok("0x55d398326f99059fF775485246999027B3197955".to_string()),
        ("USDT", "avalanche") => Ok("0x9702230A8Ea53601f5cD2dc00fDBc13d4dF4A8c7".to_string()),

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
        ("ETH" | "WETH", "blast") => Ok("0x4300000000000000000000000000000000000004".to_string()),

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

/// Get a quote from Synapse Protocol
pub async fn get_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    let cache_key = format!(
        "synapse:{}:{}:{}:{}",
        request.asset,
        request.from_chain,
        request.to_chain,
        request.amount.as_deref().unwrap_or("1000000")
    );

    get_cached_quote(&cache_key, &config.cache, || {
        fetch_synapse_quote(request, config)
    })
    .await
}

/// Fetch quote from Synapse
async fn fetch_synapse_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    retry_request(
        || fetch_synapse_quote_once(request, config),
        config.retries,
        "Synapse API call",
    )
    .await
}

/// Single attempt to fetch Synapse quote
async fn fetch_synapse_quote_once(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching Synapse quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Map chains
    let from_chain_id = map_chain_name(&request.from_chain)?;
    let to_chain_id = map_chain_name(&request.to_chain)?;

    // Get token addresses for both chains
    let from_token = get_token_address(&request.asset, &request.from_chain)?;
    let to_token = get_token_address(&request.asset, &request.to_chain)?;

    // Get amount or use default
    let decimals = get_token_decimals(&request.asset);
    let from_amount = request
        .amount
        .clone()
        .unwrap_or_else(|| (10_u128.pow(decimals)).to_string());

    // Build API URL with query parameters
    let mut url = format!(
        "{}/bridge/v2?fromChainId={}&fromToken={}&fromAmount={}&toChainId={}&toToken={}",
        SYNAPSE_API_BASE, from_chain_id, from_token, from_amount, to_chain_id, to_token
    );

    // Add slippage if non-default (optional parameter)
    if request.slippage != 0.5 {
        url.push_str(&format!("&slippage={}", request.slippage));
    }

    info!("Requesting Synapse quote from: {}", url);

    let response_result = config.client.get(&url).send().await;

    let response = match response_result {
        Ok(resp) => resp,
        Err(e) => {
            info!("Synapse API network error: {}, creating estimate", e);
            return create_synapse_estimate(request);
        }
    };

    let status = response.status();
    if !status.is_success() {
        warn!("Synapse API returned {}, creating estimate", status);
        return create_synapse_estimate(request);
    }

    let response_text = response.text().await.map_err(BridgeError::from)?;
    debug!("Synapse API response: {}", response_text);

    // Parse as array of quotes
    match serde_json::from_str::<Vec<SynapseBridgeQuoteResponse>>(&response_text) {
        Ok(quotes) if !quotes.is_empty() => {
            // Take the first quote (best quote)
            let synapse_quote = &quotes[0];

            // Calculate fee from amounts
            let divisor = 10_f64.powi(decimals as i32);

            let input_amount = from_amount.parse::<f64>().unwrap_or(0.0);
            let expected_output = synapse_quote
                .expected_to_amount
                .parse::<f64>()
                .unwrap_or(0.0);

            let input_readable = input_amount / divisor;
            let output_readable = expected_output / divisor;
            let fee_readable = input_readable - output_readable;

            let metadata = serde_json::json!({
                "quote_id": synapse_quote.id,
                "from_chain_id": synapse_quote.from_chain_id,
                "to_chain_id": synapse_quote.to_chain_id,
                "expected_to_amount": synapse_quote.expected_to_amount,
                "min_to_amount": synapse_quote.min_to_amount,
                "router_address": synapse_quote.router_address,
                "module_names": synapse_quote.module_names,
                "gas_drop_amount": synapse_quote.gas_drop_amount,
                "call_data": synapse_quote.call_data,
                "network": "Synapse Protocol",
                "architecture": "cross_chain_amm",
                "security_model": "canonical_bridges_plus_synapse_chain",
                "route": format!("{} -> {}", request.from_chain, request.to_chain),
                "note": "Real-time quote from Synapse API"
            });

            let quote = BridgeQuote {
                bridge: "Synapse".to_string(),
                fee: fee_readable,
                est_time: synapse_quote.estimated_time,
                metadata: Some(metadata),
            };

            info!(
                "Synapse quote retrieved: fee={:.6} {}, time={}s",
                quote.fee, request.asset, quote.est_time
            );

            Ok(quote)
        }
        Ok(_) => {
            info!("Synapse API returned empty quotes, creating estimate");
            create_synapse_estimate(request)
        }
        Err(e) => {
            info!("Failed to parse Synapse response: {}, creating estimate", e);
            create_synapse_estimate(request)
        }
    }
}

/// Create estimated Synapse quote
fn create_synapse_estimate(request: &BridgeQuoteRequest) -> Result<BridgeQuote, BridgeError> {
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

    // Synapse fees: 0.04-0.06% swap fee + bridge fee + gas
    let (fee_percentage, base_gas_cost) = match request.asset.to_uppercase().as_str() {
        "USDC" | "USDT" => (0.0005, 0.15),  // 0.05% + ~$0.15 gas
        "ETH" | "WETH" => (0.0004, 0.0003), // 0.04% + ~$0.90 gas (in ETH)
        "DAI" => (0.0005, 0.18),            // 0.05% + ~$0.18 gas
        "WBTC" => (0.0006, 0.00001),        // 0.06% + gas (in BTC)
        _ => (0.0006, 1.0),
    };

    let estimated_fee = (amount_readable * fee_percentage) + base_gas_cost;

    // Synapse is relatively fast
    let est_time = match (request.from_chain.as_str(), request.to_chain.as_str()) {
        ("ethereum", _) => 900, // L1 to L2: ~15 mins
        (_, "ethereum") => 900, // L2 to L1: ~15 mins
        _ => 300,               // L2 to L2: ~5 mins
    };

    let metadata = serde_json::json!({
        "estimated": true,
        "fee_percentage": fee_percentage,
        "base_gas_cost": base_gas_cost,
        "amount": amount_readable,
        "network": "Synapse Protocol",
        "architecture": "cross_chain_amm",
        "security_model": "canonical_bridges_plus_synapse_chain",
        "supported_chains": ["ethereum", "bsc", "polygon", "arbitrum", "optimism", "avalanche", "fantom", "base", "blast"],
        "note": "Estimated quote (API unavailable) - Synapse uses cross-chain AMM with canonical bridges",
        "route": format!("{} -> {}", request.from_chain, request.to_chain),
        "fee_structure": format!("{}% + {} {} gas", fee_percentage * 100.0, base_gas_cost, request.asset)
    });

    let quote = BridgeQuote {
        bridge: "Synapse".to_string(),
        fee: estimated_fee,
        est_time,
        metadata: Some(metadata),
    };

    info!(
        "Synapse estimate created: fee={:.6} {}, time={}s",
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
        assert_eq!(map_chain_name("avalanche").unwrap(), 43114);
        assert_eq!(map_chain_name("base").unwrap(), 8453);
        assert_eq!(map_chain_name("blast").unwrap(), 81457);
        assert_eq!(map_chain_name("scroll").unwrap(), 534352);
        assert!(map_chain_name("invalid").is_err());
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
    fn test_synapse_estimate() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "ethereum".to_string(),
            to_chain: "arbitrum".to_string(),
            amount: Some("1000000".to_string()), // 1 USDC
            slippage: 0.5,
        };

        let quote = create_synapse_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Synapse");
        assert!(quote.fee > 0.0);
        // Fee should be reasonable for 1 USDC (0.05% + $0.15 = ~$0.155)
        assert!(quote.fee > 0.15 && quote.fee < 0.20);
        // Ethereum to Arbitrum should be ~15 minutes
        assert_eq!(quote.est_time, 900);
    }

    #[test]
    fn test_synapse_estimate_l2_to_l2() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "arbitrum".to_string(),
            to_chain: "optimism".to_string(),
            amount: Some("10000000".to_string()), // 10 USDC
            slippage: 0.5,
        };

        let quote = create_synapse_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Synapse");
        // L2 to L2 should be faster (~5 minutes)
        assert_eq!(quote.est_time, 300);
    }
}
