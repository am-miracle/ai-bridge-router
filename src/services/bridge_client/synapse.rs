use tracing::{debug, info};

use super::{get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

// const SYNAPSE_API_BASE: &str = "https://api.synapseprotocol.com/";

// /// Synapse quote request payload
// #[derive(Debug, Serialize)]
// #[serde(rename_all = "camelCase")]
// struct SynapseQuoteRequest {
//     from_chain_Id: String,
//     from_token: String,
//     from_amount: String,
//     from_sender: String,
//     to_chain_id: String,
//     to_token: String,
//     to_recipient: String,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     slippage: Option<f64>,
// }

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

/// Map asset symbols
fn map_asset_symbol(asset: &str) -> Result<String, BridgeError> {
    match asset.to_uppercase().as_str() {
        "USDC" => Ok("USDC".to_string()),
        "USDT" => Ok("USDT".to_string()),
        "DAI" => Ok("DAI".to_string()),
        "ETH" | "WETH" => Ok("ETH".to_string()),
        "WBTC" => Ok("WBTC".to_string()),
        "SYN" => Ok("SYN".to_string()),
        "NUSD" | "NETH" | "NBTC" => Ok(asset.to_uppercase()), // Synapse native assets
        _ => Err(BridgeError::UnsupportedAsset {
            asset: asset.to_string(),
        }),
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
    _config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching Synapse quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Validate chains and assets
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    map_asset_symbol(&request.asset)?;

    // Synapse API requires more integration
    // Return estimate for now
    info!("Synapse quote - using estimate");
    create_synapse_estimate(request)
}

/// Create estimated Synapse quote
fn create_synapse_estimate(request: &BridgeQuoteRequest) -> Result<BridgeQuote, BridgeError> {
    // Verify route is supported
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    map_asset_symbol(&request.asset)?;

    // Synapse fees vary by route and liquidity
    // Typical: 0.04-0.06% swap fee + bridge fee
    let estimated_fee = match request.asset.to_uppercase().as_str() {
        "USDC" | "USDT" => 0.15,           // ~$0.15
        "NUSD" => 0.10,                    // Native stablecoin
        "ETH" | "WETH" | "NETH" => 0.0003, // ~$0.90
        "DAI" => 0.18,
        "WBTC" | "NBTC" => 0.00001,
        "SYN" => 1.0, // Native token
        _ => 0.0005,
    };

    // Synapse is relatively fast
    let est_time = match (request.from_chain.as_str(), request.to_chain.as_str()) {
        ("ethereum", _) => 900, // L1 to L2: ~15 mins
        (_, "ethereum") => 900, // L2 to L1: ~15 mins
        _ => 300,               // L2 to L2: ~5 mins
    };

    let metadata = serde_json::json!({
        "estimated": true,
        "network": "Synapse Protocol",
        "architecture": "cross_chain_amm",
        "security_model": "canonical_bridges_plus_synapse_chain",
        "supported_chains": ["ethereum", "bsc", "polygon", "arbitrum", "optimism", "avalanche", "fantom", "base", "blast"],
        "note": "Estimated quote - Synapse uses cross-chain AMM with canonical bridges",
        "route": format!("{} -> {}", request.from_chain, request.to_chain),
        "native_assets": ["nUSD", "nETH", "nBTC"],
        "fee_structure": "Swap fee (0.04-0.06%) + bridge fee"
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
        assert!(map_chain_name("invalid").is_err());
    }

    #[test]
    fn test_asset_mapping() {
        assert_eq!(map_asset_symbol("USDC").unwrap(), "USDC");
        assert_eq!(map_asset_symbol("ETH").unwrap(), "ETH");
        assert_eq!(map_asset_symbol("SYN").unwrap(), "SYN");
        assert_eq!(map_asset_symbol("NUSD").unwrap(), "NUSD");
        assert!(map_asset_symbol("UNKNOWN").is_err());
    }

    #[test]
    fn test_synapse_estimate() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "ethereum".to_string(),
            to_chain: "arbitrum".to_string(),
            amount: Some("1000000".to_string()),
            slippage: 0.5,
        };

        let quote = create_synapse_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Synapse");
        assert!(quote.fee > 0.0);
    }
}
