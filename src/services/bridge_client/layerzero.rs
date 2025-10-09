use tracing::{debug, info};

use super::{get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

/// Map chain names to Superbridge/LayerZero chain IDs
fn map_chain_name(chain: &str) -> Result<u32, BridgeError> {
    let chain_id = match chain.to_lowercase().as_str() {
        "ethereum" | "eth" | "mainnet" => 1,
        "optimism" | "opt" => 10,
        "bsc" | "binance" | "bnb" | "bnb-smart-chain" => 56,
        "polygon" | "matic" => 137,
        "fantom" | "ftm" => 250,
        "zksync" | "zksync-era" => 324,
        "redstone" => 690,
        "metis" => 1088,
        "lisk" => 1135,
        "mantle" => 5000,
        "base" => 8453,
        "mode" => 34443,
        "avalanche" | "avax" => 43114,
        "arbitrum" | "arb" | "arbitrum-one" => 42161,
        "linea" => 59144,
        "blast" => 81457,
        "scroll" => 534352,
        "zora" => 7777777,
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
        "ETH" | "WETH" => Ok("ETH".to_string()),
        "USDC" => Ok("USDC".to_string()),
        "USDT" => Ok("USDT".to_string()),
        "DAI" => Ok("DAI".to_string()),
        "WBTC" => Ok("WBTC".to_string()),
        _ => Err(BridgeError::UnsupportedAsset {
            asset: asset.to_string(),
        }),
    }
}

/// Get a quote from LayerZero via Superbridge
pub async fn get_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    let cache_key = format!(
        "layerzero:{}:{}:{}:{}",
        request.asset,
        request.from_chain,
        request.to_chain,
        request.amount.as_deref().unwrap_or("1000000")
    );

    get_cached_quote(&cache_key, &config.cache, || {
        fetch_layerzero_quote(request, config)
    })
    .await
}

/// Fetch quote from LayerZero
async fn fetch_layerzero_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    retry_request(
        || fetch_layerzero_quote_once(request, config),
        config.retries,
        "LayerZero API call",
    )
    .await
}

/// Single attempt to fetch LayerZero quote
async fn fetch_layerzero_quote_once(
    request: &BridgeQuoteRequest,
    _config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching LayerZero quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Validate chains and assets
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    map_asset_symbol(&request.asset)?;

    // Superbridge API requires more integration
    // Return estimate for now
    info!("LayerZero quote - using estimate");
    create_layerzero_estimate(request)
}

/// Create estimated LayerZero quote
fn create_layerzero_estimate(request: &BridgeQuoteRequest) -> Result<BridgeQuote, BridgeError> {
    // Verify route is supported
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    map_asset_symbol(&request.asset)?;

    // LayerZero fees are similar to Stargate (it's the underlying protocol)
    let estimated_fee = match request.asset.to_uppercase().as_str() {
        "USDC" | "USDT" => 0.12,  // ~$0.12
        "ETH" | "WETH" => 0.0002, // ~$0.60
        "DAI" => 0.15,
        "WBTC" => 0.000008,
        _ => 0.0005,
    };

    // LayerZero is fast (message-based)
    let est_time = match (request.from_chain.as_str(), request.to_chain.as_str()) {
        ("ethereum", _) => 600, // L1 needs finality: ~10 mins
        (_, "ethereum") => 600,
        _ => 240, // L2 to L2: ~4 mins
    };

    let metadata = serde_json::json!({
        "estimated": true,
        "network": "LayerZero",
        "architecture": "omnichain_messaging_protocol",
        "security_model": "oracle_relayer",
        "supported_chains": ["ethereum", "bsc", "avalanche", "polygon", "arbitrum", "optimism", "fantom", "base", "linea", "zksync", "scroll"],
        "note": "Estimated quote - LayerZero enables omnichain applications via Superbridge",
        "route": format!("{} -> {}", request.from_chain, request.to_chain),
        "via": "Superbridge"
    });

    let quote = BridgeQuote {
        bridge: "LayerZero".to_string(),
        fee: estimated_fee,
        est_time,
        metadata: Some(metadata),
    };

    info!(
        "LayerZero estimate created: fee={:.6} {}, time={}s",
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
        assert_eq!(map_chain_name("arbitrum").unwrap(), 42161);
        assert_eq!(map_chain_name("base").unwrap(), 8453);
        assert!(map_chain_name("invalid-chain").is_err());
    }

    #[test]
    fn test_layerzero_estimate() {
        let request = BridgeQuoteRequest {
            asset: "ETH".to_string(),
            from_chain: "ethereum".to_string(),
            to_chain: "optimism".to_string(),
            amount: Some("1000000000000000000".to_string()),
            slippage: 0.5,
        };

        let quote = create_layerzero_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "LayerZero");
        assert!(quote.fee > 0.0);
    }
}
