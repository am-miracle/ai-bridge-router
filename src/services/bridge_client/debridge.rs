use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::{get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

/// deBridge DLN API base URL
const DEBRIDGE_API_BASE: &str = "https://dln.debridge.finance/v1.0/dln";

/// deBridge order creation response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DeBridgeOrderResponse {
    estimation: DeBridgeEstimation,
    tx: DeBridgeTransaction,
    order: DeBridgeOrder,
    #[serde(rename = "orderId")]
    order_id: String,
    #[serde(rename = "fixFee")]
    fix_fee: String,
    #[serde(rename = "prependedOperatingExpenseCost")]
    prepended_operating_expense_cost: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DeBridgeEstimation {
    #[serde(rename = "srcChainTokenIn")]
    src_chain_token_in: DeBridgeTokenInfo,
    #[serde(rename = "srcChainTokenOut")]
    src_chain_token_out: Option<DeBridgeTokenInfo>,
    #[serde(rename = "dstChainTokenOut")]
    dst_chain_token_out: DeBridgeTokenOutInfo,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
struct DeBridgeTokenInfo {
    address: String,
    #[serde(rename = "chainId")]
    chain_id: u64,
    decimals: u8,
    name: String,
    symbol: String,
    amount: String,
    #[serde(rename = "approximateOperatingExpense")]
    approximate_operating_expense: Option<String>,
    #[serde(rename = "approximateUsdValue")]
    approximate_usd_value: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DeBridgeTokenOutInfo {
    address: String,
    #[serde(rename = "chainId")]
    chain_id: u64,
    decimals: u8,
    name: String,
    symbol: String,
    amount: String,
    #[serde(rename = "recommendedAmount")]
    recommended_amount: String,
    #[serde(rename = "approximateUsdValue")]
    approximate_usd_value: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DeBridgeTransaction {
    data: String,
    to: String,
    value: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DeBridgeOrder {
    #[serde(rename = "approximateFulfillmentDelay")]
    approximate_fulfillment_delay: u64,
}

/// Map chain names to deBridge chain IDs
fn map_chain_name(chain: &str) -> Result<u64, BridgeError> {
    let chain_id = match chain.to_lowercase().as_str() {
        "ethereum" | "eth" | "mainnet" => 1,
        "bsc" | "binance" | "bnb" => 56,
        "polygon" | "matic" => 137,
        "arbitrum" | "arb" | "arbitrum-one" => 42161,
        "optimism" | "opt" => 10,
        "avalanche" | "avax" => 43114,
        "base" => 8453,
        "linea" => 59144,
        "solana" | "sol" => 7565164,
        _ => {
            return Err(BridgeError::UnsupportedRoute {
                from_chain: chain.to_string(),
                to_chain: "".to_string(),
            });
        }
    };
    Ok(chain_id)
}

/// Get token address for deBridge (uses contract addresses)
fn get_token_address(asset: &str, chain: &str) -> Result<String, BridgeError> {
    // deBridge token addresses
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

/// Get a quote from deBridge DLN
pub async fn get_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    let cache_key = format!(
        "debridge:{}:{}:{}:{}",
        request.asset,
        request.from_chain,
        request.to_chain,
        request.amount.as_deref().unwrap_or("1000000")
    );

    get_cached_quote(&cache_key, &config.cache, || {
        fetch_debridge_quote(request, config)
    })
    .await
}

/// Fetch quote from deBridge
async fn fetch_debridge_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    retry_request(
        || fetch_debridge_quote_once(request, config),
        config.retries,
        "deBridge API call",
    )
    .await
}

/// Single attempt to fetch deBridge quote
async fn fetch_debridge_quote_once(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching deBridge quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Map chains
    let src_chain_id = map_chain_name(&request.from_chain)?;
    let dst_chain_id = map_chain_name(&request.to_chain)?;

    // Get token addresses for both chains
    let src_chain_token_in = get_token_address(&request.asset, &request.from_chain)?;
    let dst_chain_token_out = get_token_address(&request.asset, &request.to_chain)?;

    // Get amount or use default
    let decimals = get_token_decimals(&request.asset);
    let src_chain_token_in_amount = request
        .amount
        .clone()
        .unwrap_or_else(|| (10_u128.pow(decimals)).to_string());

    // Build API URL with query parameters (estimation only mode - no wallet addresses)
    let url = format!(
        "{}/order/create-tx?srcChainId={}&srcChainTokenIn={}&srcChainTokenInAmount={}&dstChainId={}&dstChainTokenOut={}&dstChainTokenOutAmount=auto&prependOperatingExpense=true",
        DEBRIDGE_API_BASE,
        src_chain_id,
        src_chain_token_in,
        src_chain_token_in_amount,
        dst_chain_id,
        dst_chain_token_out
    );

    info!("Requesting deBridge quote from: {}", url);

    let response_result = config.client.get(&url).send().await;

    let response = match response_result {
        Ok(resp) => resp,
        Err(e) => {
            info!("deBridge API network error: {}, creating estimate", e);
            return create_debridge_estimate(request);
        }
    };

    let status = response.status();
    if !status.is_success() {
        warn!("deBridge API returned {}, creating estimate", status);
        return create_debridge_estimate(request);
    }

    let response_text = response.text().await.map_err(BridgeError::from)?;
    debug!("deBridge API response: {}", response_text);

    // Parse deBridge order response
    match serde_json::from_str::<DeBridgeOrderResponse>(&response_text) {
        Ok(debridge_response) => {
            // Calculate fee from input vs output amounts
            let divisor = 10_f64.powi(decimals as i32);

            let input_amount = src_chain_token_in_amount.parse::<f64>().unwrap_or(0.0);
            let output_amount = debridge_response
                .estimation
                .dst_chain_token_out
                .amount
                .parse::<f64>()
                .unwrap_or(0.0);

            let input_readable = input_amount / divisor;
            let output_readable = output_amount / divisor;
            let fee_readable = input_readable - output_readable;

            // Add operating expense if available
            let operating_expense = debridge_response
                .prepended_operating_expense_cost
                .as_ref()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0)
                / divisor;

            let total_fee = fee_readable + operating_expense;

            let metadata = serde_json::json!({
                "order_id": debridge_response.order_id,
                "src_chain_token_in": debridge_response.estimation.src_chain_token_in,
                "dst_chain_token_out": {
                    "amount": debridge_response.estimation.dst_chain_token_out.amount,
                    "recommended_amount": debridge_response.estimation.dst_chain_token_out.recommended_amount,
                    "approximate_usd_value": debridge_response.estimation.dst_chain_token_out.approximate_usd_value
                },
                "operating_expense": operating_expense,
                "fix_fee": debridge_response.fix_fee,
                "transaction_to": debridge_response.tx.to,
                "transaction_value": debridge_response.tx.value,
                "network": "deBridge DLN",
                "architecture": "deswap_liquidity_network",
                "security_model": "solver_based_market_making",
                "route": format!("{} -> {}", request.from_chain, request.to_chain),
                "note": "Real-time quote from deBridge DLN API"
            });

            let quote = BridgeQuote {
                bridge: "deBridge".to_string(),
                fee: total_fee,
                est_time: debridge_response.order.approximate_fulfillment_delay,
                metadata: Some(metadata),
            };

            info!(
                "deBridge quote retrieved: fee={:.6} {}, time={}s",
                quote.fee, request.asset, quote.est_time
            );

            Ok(quote)
        }
        Err(e) => {
            info!(
                "Failed to parse deBridge response: {}, creating estimate",
                e
            );
            create_debridge_estimate(request)
        }
    }
}

/// Create estimated deBridge quote
fn create_debridge_estimate(request: &BridgeQuoteRequest) -> Result<BridgeQuote, BridgeError> {
    // Verify route is supported
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    get_token_address(&request.asset, &request.from_chain)?;

    // Calculate fee based on amount (percentage-based for solver fees)
    let decimals = get_token_decimals(&request.asset);
    let divisor = 10_f64.powi(decimals as i32);

    let amount_f64 = request
        .amount
        .as_ref()
        .and_then(|a| a.parse::<f64>().ok())
        .unwrap_or(10_f64.powi(decimals as i32));

    let amount_readable = amount_f64 / divisor;

    // deBridge fees: solver margin + operating expenses
    let (solver_margin_pct, operating_expense) = match request.asset.to_uppercase().as_str() {
        "USDC" | "USDT" => (0.0005, 0.05),  // 0.05% + ~$0.05 operating
        "ETH" | "WETH" => (0.0004, 0.0001), // 0.04% + gas in ETH
        "DAI" => (0.0005, 0.06),            // 0.05% + ~$0.06
        "WBTC" => (0.0006, 0.000005),       // 0.06% + gas in BTC
        _ => (0.0007, 0.10),                // 0.07% + default
    };

    let estimated_fee = (amount_readable * solver_margin_pct) + operating_expense;

    info!(
        "deBridge estimate for {} {}: margin={}%, operating={}, total={}",
        amount_readable,
        request.asset,
        solver_margin_pct * 100.0,
        operating_expense,
        estimated_fee
    );

    // deBridge is very fast (solver-based)
    let est_time = match (request.from_chain.as_str(), request.to_chain.as_str()) {
        ("ethereum", _) => 300, // L1 source: ~5 mins
        (_, "ethereum") => 300, // L1 destination: ~5 mins
        _ => 120,               // L2 to L2: ~2 mins
    };

    let metadata = serde_json::json!({
        "estimated": true,
        "solver_margin_pct": solver_margin_pct,
        "operating_expense": operating_expense,
        "amount": amount_readable,
        "network": "deBridge DLN",
        "architecture": "deswap_liquidity_network",
        "security_model": "solver_based_market_making",
        "supported_chains": ["ethereum", "bsc", "polygon", "arbitrum", "optimism", "avalanche", "base", "linea", "solana"],
        "note": "Estimated quote (API unavailable) - deBridge uses solver-based market making",
        "route": format!("{} -> {}", request.from_chain, request.to_chain),
        "fee_structure": format!("{}% solver margin + {} operating expense", solver_margin_pct * 100.0, operating_expense)
    });

    let quote = BridgeQuote {
        bridge: "deBridge".to_string(),
        fee: estimated_fee,
        est_time,
        metadata: Some(metadata),
    };

    info!(
        "deBridge estimate created: fee={:.6} {}, time={}s",
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
        assert_eq!(map_chain_name("bsc").unwrap(), 56);
        assert_eq!(map_chain_name("polygon").unwrap(), 137);
        assert_eq!(map_chain_name("arbitrum").unwrap(), 42161);
        assert_eq!(map_chain_name("optimism").unwrap(), 10);
        assert_eq!(map_chain_name("avalanche").unwrap(), 43114);
        assert_eq!(map_chain_name("base").unwrap(), 8453);
        assert_eq!(map_chain_name("solana").unwrap(), 7565164);
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
    fn test_debridge_estimate() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "ethereum".to_string(),
            to_chain: "arbitrum".to_string(),
            amount: Some("1000000".to_string()), // 1 USDC
            slippage: 0.5,
        };

        let quote = create_debridge_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "deBridge");
        assert!(quote.fee > 0.0);
        // Fee should be reasonable for 1 USDC (0.05% + $0.05 operating)
        assert!(quote.fee > 0.045 && quote.fee < 0.10);
        // Ethereum to Arbitrum should be ~5 minutes (300s)
        assert_eq!(quote.est_time, 300);
    }

    #[test]
    fn test_debridge_estimate_l2_to_l2() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "arbitrum".to_string(),
            to_chain: "optimism".to_string(),
            amount: Some("10000000".to_string()), // 10 USDC
            slippage: 0.5,
        };

        let quote = create_debridge_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "deBridge");
        // L2 to L2 should be faster (~2 minutes)
        assert_eq!(quote.est_time, 120);
        // Fee should scale with amount
        assert!(quote.fee > 0.05);
    }
}
