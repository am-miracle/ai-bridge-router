use tracing::{debug, info};

use super::{get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

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
    _config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching cBridge quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Validate chains and assets
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    map_asset_symbol(&request.asset)?;

    // cBridge API requires complex integration with transfer configs
    // Return estimate for now
    info!("cBridge quote - using estimate");
    create_cbridge_estimate(request)
}

/// Create estimated cBridge quote
fn create_cbridge_estimate(request: &BridgeQuoteRequest) -> Result<BridgeQuote, BridgeError> {
    // Verify route is supported
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    map_asset_symbol(&request.asset)?;

    // cBridge fees: 0.04% base fee + liquidity provider fee
    let estimated_fee = match request.asset.to_uppercase().as_str() {
        "USDC" | "USDT" => 0.10,   // ~$0.10
        "ETH" | "WETH" => 0.00025, // ~$0.75
        "DAI" | "BUSD" => 0.12,
        "WBTC" => 0.00001,
        "CELR" => 10.0, // Native token
        _ => 0.0004,
    };

    // cBridge is relatively fast (State Guardian Network)
    let est_time = match (request.from_chain.as_str(), request.to_chain.as_str()) {
        ("ethereum", _) => 1200, // L1 needs finality: ~20 mins
        (_, "ethereum") => 1200,
        _ => 300, // L2 to L2 or others: ~5 mins
    };

    let metadata = serde_json::json!({
        "estimated": true,
        "network": "Celer cBridge",
        "architecture": "state_guardian_network",
        "security_model": "sgn_pos_with_optimistic_rollup",
        "supported_chains": ["ethereum", "bsc", "arbitrum", "optimism", "polygon", "avalanche", "fantom", "base", "scroll"],
        "note": "Estimated quote - cBridge uses State Guardian Network for cross-chain transfers",
        "route": format!("{} -> {}", request.from_chain, request.to_chain),
        "base_fee": "0.04%",
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
            amount: Some("1000000".to_string()),
            slippage: 0.5,
        };

        let quote = create_cbridge_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Celer cBridge");
        assert!(quote.fee > 0.0);
    }
}
