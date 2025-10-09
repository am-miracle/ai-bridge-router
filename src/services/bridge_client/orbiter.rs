use tracing::{debug, info};

use super::{get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

/// Map chain names for Orbiter Finance
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
    _config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching Orbiter quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Validate chains and assets
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    map_asset_symbol(&request.asset)?;

    // Orbiter API requires more integration
    // Return estimate for now
    info!("Orbiter quote - using estimate");
    create_orbiter_estimate(request)
}

/// Create estimated Orbiter quote
fn create_orbiter_estimate(request: &BridgeQuoteRequest) -> Result<BridgeQuote, BridgeError> {
    // Verify route is supported
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    map_asset_symbol(&request.asset)?;

    // Orbiter is known for low fees (maker-taker model)
    // Trading fee: 0-0.1% depending on market conditions
    let estimated_fee = match request.asset.to_uppercase().as_str() {
        "USDC" | "USDT" => 0.08,   // ~$0.08
        "ETH" | "WETH" => 0.00015, // ~$0.45
        "DAI" => 0.10,
        _ => 0.0003,
    };

    // Orbiter is very fast for L2<->L2 transfers
    let est_time = match (request.from_chain.as_str(), request.to_chain.as_str()) {
        ("ethereum", _) => 900, // L1 to L2: ~15 mins
        (_, "ethereum") => 900, // L2 to L1: ~15 mins
        _ => 120,               // L2 to L2: ~2 mins (fastest)
    };

    let metadata = serde_json::json!({
        "estimated": true,
        "network": "Orbiter Finance",
        "architecture": "maker_taker_model",
        "security_model": "zkrollup_native_optimized",
        "supported_chains": ["ethereum", "arbitrum", "optimism", "polygon", "zksync", "starknet", "linea", "base", "scroll"],
        "note": "Estimated quote - Orbiter specializes in fast L2 transfers with maker-taker model",
        "route": format!("{} -> {}", request.from_chain, request.to_chain),
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
            amount: Some("1000000000000000000".to_string()),
            slippage: 0.5,
        };

        let quote = create_orbiter_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Orbiter");
        assert!(quote.fee > 0.0);
        assert_eq!(quote.est_time, 120); // Fast L2 to L2
    }
}
