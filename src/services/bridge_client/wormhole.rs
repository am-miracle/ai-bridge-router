use tracing::debug;

use super::{get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

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

/// Map asset symbols
fn map_asset_symbol(asset: &str) -> Result<String, BridgeError> {
    match asset.to_uppercase().as_str() {
        "USDC" => Ok("USDC".to_string()),
        "USDT" => Ok("USDT".to_string()),
        "ETH" | "WETH" => Ok("WETH".to_string()),
        "MATIC" | "WMATIC" => Ok("WMATIC".to_string()),
        "DAI" => Ok("DAI".to_string()),
        "WBTC" => Ok("WBTC".to_string()),
        "AVAX" | "WAVAX" => Ok("WAVAX".to_string()),
        "BNB" | "WBNB" => Ok("WBNB".to_string()),
        _ => Err(BridgeError::UnsupportedAsset {
            asset: asset.to_string(),
        }),
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
    _config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching Wormhole quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Validate chains and assets
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    map_asset_symbol(&request.asset)?;

    // Wormhole API doesn't have a public quote endpoint yet
    // Return estimate for now
    create_wormhole_estimate(request)
}

/// Create estimated Wormhole quote
fn create_wormhole_estimate(request: &BridgeQuoteRequest) -> Result<BridgeQuote, BridgeError> {
    // Verify route is supported
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    map_asset_symbol(&request.asset)?;

    // Wormhole fees: fixed relayer fee + gas costs
    let estimated_fee = match request.asset.to_uppercase().as_str() {
        "USDC" | "USDT" => 0.25,  // ~$0.25 relayer fee
        "WETH" | "ETH" => 0.0005, // ~$1.50 at $3k ETH
        "WBTC" => 0.00002,        // ~$1.00 at $50k BTC
        "WMATIC" => 5.0,          // ~$5 worth
        _ => 0.0015,              // 0.15%
    };

    // Wormhole uses guardian network - relatively fast
    // L1 to L1: 15-30 minutes (wait for finality)
    // L2 to L2: 5-10 minutes
    let est_time = match (request.from_chain.as_str(), request.to_chain.as_str()) {
        ("ethereum", "ethereum") => 0, // Same chain not supported
        ("ethereum", _) => 900,        // L1 to anywhere: ~15 mins
        (_, "ethereum") => 900,        // Anywhere to L1: ~15 mins
        _ => 600,                      // L2 to L2 or others: ~10 mins
    };

    let metadata = serde_json::json!({
        "estimated": true,
        "network": "Wormhole",
        "architecture": "guardian_network",
        "security_model": "19_guardian_multisig",
        "supported_chains": ["ethereum", "bsc", "polygon", "avalanche", "fantom", "arbitrum", "optimism", "base", "scroll"],
        "note": "Estimated quote - Wormhole uses guardian network for cross-chain messaging",
        "route": format!("{} -> {}", request.from_chain, request.to_chain),
        "tvl": "Multi-billion dollar bridge",
        "exploit_history": "Major exploit in 2022 ($325M, recovered)"
    });

    let quote = BridgeQuote {
        bridge: "Wormhole".to_string(),
        fee: estimated_fee,
        est_time,
        metadata: Some(metadata),
    };

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
        assert!(map_chain_name("invalid-chain").is_err());
    }

    #[test]
    fn test_asset_mapping() {
        assert_eq!(map_asset_symbol("USDC").unwrap(), "USDC");
        assert_eq!(map_asset_symbol("ETH").unwrap(), "WETH");
        assert_eq!(map_asset_symbol("WETH").unwrap(), "WETH");
        assert!(map_asset_symbol("UNKNOWN").is_err());
    }

    #[test]
    fn test_wormhole_estimate() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "ethereum".to_string(),
            to_chain: "polygon".to_string(),
            amount: Some("1000000".to_string()),
            slippage: 0.5,
        };

        let quote = create_wormhole_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Wormhole");
        assert!(quote.fee > 0.0);
        assert!(quote.est_time > 0);
    }
}
