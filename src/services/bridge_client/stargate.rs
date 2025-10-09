use tracing::{debug, info};

use super::{get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

/// Map chain names to Stargate chain IDs
fn map_chain_name(chain: &str) -> Result<u16, BridgeError> {
    let chain_id = match chain.to_lowercase().as_str() {
        "ethereum" | "eth" | "mainnet" => 101,
        "bsc" | "binance" | "bnb" | "bnb-smart-chain" => 102,
        "avalanche" | "avax" => 106,
        "polygon" | "matic" => 109,
        "arbitrum" | "arb" => 110,
        "optimism" | "opt" => 111,
        "fantom" | "ftm" => 112,
        "metis" => 151,
        "kava" => 177,
        "mantle" => 181,
        "linea" => 183,
        "base" => 184,
        "scroll" => 214,
        "zksync" | "zksync-era" => 165,
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
    _config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching Stargate quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Validate chains and assets
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    map_asset_symbol(&request.asset)?;

    // Stargate API requires more complex integration
    // Return estimate for now
    info!("Stargate quote - using estimate");
    create_stargate_estimate(request)
}

/// Create estimated Stargate quote
fn create_stargate_estimate(request: &BridgeQuoteRequest) -> Result<BridgeQuote, BridgeError> {
    // Verify route is supported
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    map_asset_symbol(&request.asset)?;

    // Stargate uses unified liquidity pools
    // Fees: 0.06% of transfer amount + LayerZero messaging fee
    let estimated_fee = match request.asset.to_uppercase().as_str() {
        "USDC" | "USDT" => 0.15,                 // ~$0.15 (0.06% + messaging)
        "ETH" | "WETH" => 0.0003,                // ~$0.90
        "DAI" | "FRAX" | "LUSD" | "MAI" => 0.18, // Slightly higher for less liquid
        _ => 0.0006,                             // 0.06%
    };

    // Stargate is fast (LayerZero V1)
    // Typical time: 1-5 minutes depending on finality
    let est_time = match (request.from_chain.as_str(), request.to_chain.as_str()) {
        ("ethereum", _) => 600, // L1 needs finality: ~10 mins
        (_, "ethereum") => 600, // To L1: ~10 mins
        _ => 180,               // L2 to L2: ~3 mins
    };

    let metadata = serde_json::json!({
        "estimated": true,
        "network": "Stargate Finance",
        "architecture": "layerzero_v1_omnichain",
        "security_model": "unified_liquidity_pools",
        "supported_chains": ["ethereum", "bsc", "avalanche", "polygon", "arbitrum", "optimism", "fantom", "base", "linea", "metis"],
        "note": "Estimated quote - Stargate uses LayerZero for omnichain transfers",
        "route": format!("{} -> {}", request.from_chain, request.to_chain),
        "tvl": "~$300M+ liquidity",
        "fees": "0.06% + LayerZero message fee"
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
    fn test_chain_mapping() {
        assert_eq!(map_chain_name("ethereum").unwrap(), 101);
        assert_eq!(map_chain_name("polygon").unwrap(), 109);
        assert_eq!(map_chain_name("arbitrum").unwrap(), 110);
        assert_eq!(map_chain_name("optimism").unwrap(), 111);
        assert_eq!(map_chain_name("base").unwrap(), 184);
        assert!(map_chain_name("invalid-chain").is_err());
    }

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
            amount: Some("1000000".to_string()),
            slippage: 0.5,
        };

        let quote = create_stargate_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Stargate");
        assert!(quote.fee > 0.0);
        assert!(quote.est_time > 0);
    }
}
