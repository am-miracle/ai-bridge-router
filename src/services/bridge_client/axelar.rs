use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::{get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

/// Axelar API endpoint for GMP and token transfers
/// Axelar provides General Message Passing (GMP) and Interchain Token Service (ITS)
const AXELAR_API_BASE: &str = "https://api.axelarscan.io";

/// Axelar gas fee estimation request structure
#[derive(Debug, Serialize)]
struct AxelarGasFeeRequest {
    #[serde(rename = "sourceChain")]
    source_chain: String,
    #[serde(rename = "destinationChain")]
    destination_chain: String,
    #[serde(rename = "sourceTokenAddress")]
    source_token_address: String,
    #[serde(rename = "gasMultiplier")]
    gas_multiplier: String,
}

/// Axelar gas fee response structure
#[derive(Debug, Deserialize)]
struct AxelarGasFeeResponse {
    #[serde(rename = "totalFee")]
    total_fee: String,
    #[serde(rename = "isExpressSupported")]
    is_express_supported: bool,
    #[serde(rename = "baseFee")]
    base_fee: String,
    // #[serde(rename = "expressFee")]
    // express_fee: String,
    #[serde(rename = "executionFee")]
    execution_fee: String,
    // #[serde(rename = "executionFeeWithMultiplier")]
    // execution_fee_with_multiplier: String,
    // #[serde(rename = "gasLimit")]
    // gas_limit: String,
    // #[serde(rename = "gasLimitWithL1Fee")]
    // gas_limit_with_l1_fee: String,
    #[serde(rename = "gasMultiplier")]
    gas_multiplier: f64,
    // #[serde(rename = "minGasPrice")]
    // min_gas_price: String,
}

// Note: Asset and Chain information structures are available for future use
// when implementing dynamic asset/chain discovery from the API endpoints:
// - /api/getAssets
// - /api/getChains
// - /api/getITSAssets

/// Chain name mapping for Axelar's extensive multi-ecosystem support
/// Axelar connects EVM, Cosmos, Sui, Stellar, XRPL and other blockchains
fn map_chain_name(chain: &str) -> Result<String, BridgeError> {
    let axelar_chain = match chain.to_lowercase().as_str() {
        // EVM Chains
        "ethereum" | "eth" | "mainnet" => "ethereum",
        "polygon" | "matic" => "polygon",
        "arbitrum" | "arbitrum-one" => "arbitrum",
        "optimism" | "opt" => "optimism",
        "avalanche" | "avax" => "avalanche",
        "fantom" | "ftm" => "fantom",
        "moonbeam" | "glmr" => "moonbeam",
        "bnb" | "bsc" | "binance" => "binance",
        "base" => "base",
        "linea" => "linea",
        "mantle" => "mantle",
        "celo" => "celo",
        "kava" => "kava",
        "filecoin" => "filecoin",
        "blast" => "blast",
        "fraxtal" => "fraxtal",

        // Cosmos Ecosystem
        "cosmos" | "atom" => "cosmoshub",
        "osmosis" | "osmo" => "osmosis",
        "juno" => "juno",
        "crescent" => "crescent",
        "kujira" => "kujira",
        "neutron" => "neutron",
        "injective" => "injective",
        "secret" => "secret-snip",
        "terra" => "terra",
        "terra2" => "terra-2",
        "umee" => "umee",
        "carbon" => "carbon",
        "ojo" => "ojo",

        // Other ecosystems
        "sui" => "sui",
        "stellar" => "stellar",
        "xrpl" => "xrpl",
        "saga" => "saga",

        _ => {
            return Err(BridgeError::UnsupportedRoute {
                from_chain: chain.to_string(),
                to_chain: "".to_string(),
            });
        }
    };
    Ok(axelar_chain.to_string())
}

/// Asset symbol mapping for Axelar's Interchain Token Service (ITS)
/// Supports both Gateway tokens and Interchain tokens
fn map_asset_symbol(asset: &str) -> Result<String, BridgeError> {
    match asset.to_uppercase().as_str() {
        // Gateway Tokens (axl prefixed)
        "USDC" => Ok("uusdc".to_string()),
        "USDT" => Ok("uusdt".to_string()),
        "DAI" => Ok("dai-wei".to_string()),
        "ETH" | "WETH" => Ok("eth-wei".to_string()),
        "WBTC" => Ok("wbtc-satoshi".to_string()),
        "FRAX" => Ok("frax-wei".to_string()),
        "MATIC" => Ok("matic-wei".to_string()),
        "DOT" => Ok("dot-planck".to_string()),
        "AVAX" => Ok("avax-wei".to_string()),
        "FTM" => Ok("ftm-wei".to_string()),
        "GLMR" => Ok("glmr-wei".to_string()),
        "BNB" => Ok("bnb-wei".to_string()),
        "USDE" => Ok("usde-wei".to_string()),
        "USYC" => Ok("uusyc".to_string()),
        "YUSD" => Ok("yusd-wei".to_string()),
        "DEUSD" => Ok("deusd-wei".to_string()),
        "SDAI" => Ok("sdai-wei".to_string()),
        "USN" => Ok("usn-wei".to_string()),
        "VYUSD" => Ok("vyusd-wei".to_string()),
        "SUSN" => Ok("susn-wei".to_string()),

        // Native Axelar token
        "AXL" => Ok("uaxl".to_string()),

        // Cosmos ecosystem tokens
        "ATOM" => Ok("ibc/...".to_string()), // IBC denomination
        "OSMO" => Ok("uosmo".to_string()),

        _ => {
            // For unknown assets, try to use the original symbol
            // This allows for future expansion without hardcoding
            warn!("Unknown asset symbol: {}, using as-is", asset);
            Ok(asset.to_lowercase())
        }
    }
}

/// Convert Axelar amount to human readable
fn parse_axelar_amount(amount_str: &str, asset: &str) -> Result<f64, BridgeError> {
    let amount = amount_str
        .parse::<u128>()
        .map_err(|_| BridgeError::BadResponse {
            message: format!("Invalid amount: {}", amount_str),
        })?;

    // Determine decimals based on asset
    let divisor = match asset.to_uppercase().as_str() {
        "USDC" | "USDT" | "AXL" | "USYC" => 1_000_000u128, // 6 decimals
        "ETH" | "WETH" | "DAI" | "FRAX" | "MATIC" | "AVAX" | "FTM" | "GLMR" | "BNB" | "USDE"
        | "YUSD" | "DEUSD" | "SDAI" | "USN" | "VYUSD" | "SUSN" => 1_000_000_000_000_000_000u128, // 18 decimals
        "WBTC" => 100_000_000u128,          // 8 decimals (satoshi)
        "DOT" => 10_000_000_000u128,        // 10 decimals (planck)
        _ => 1_000_000_000_000_000_000u128, // Default to 18 decimals
    };

    Ok(amount as f64 / divisor as f64)
}

/// Get a quote from Axelar bridge
pub async fn get_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    let cache_key = format!(
        "axelar:{}:{}:{}:{}",
        request.asset,
        request.from_chain,
        request.to_chain,
        request.amount.as_deref().unwrap_or("1000000") // Default amount
    );

    get_cached_quote(&cache_key, &config.cache, || {
        fetch_axelar_quote(request, config)
    })
    .await
}

/// Fetch quote directly from Axelar API
async fn fetch_axelar_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    retry_request(
        || fetch_axelar_quote_once(request, config),
        config.retries,
        "Axelar API call",
    )
    .await
}

/// Single attempt to fetch Axelar quote using the GMP gas fee estimation endpoint
async fn fetch_axelar_quote_once(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching Axelar quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Map chain names
    let source_chain = map_chain_name(&request.from_chain)?;
    let destination_chain = map_chain_name(&request.to_chain)?;

    // Use the GMP gas fee estimation endpoint
    let url = format!("{}/gmp/estimateGasFee", AXELAR_API_BASE);

    info!("Requesting Axelar gas fee estimation from: {}", url);

    let gas_fee_request = AxelarGasFeeRequest {
        source_chain: source_chain.clone(),
        destination_chain: destination_chain.clone(),
        source_token_address: "0x0000000000000000000000000000000000000000".to_string(), // Native token
        gas_multiplier: "auto".to_string(),
    };

    let response_result = config.client.post(&url).json(&gas_fee_request).send().await;

    // Handle potential timeout or network error
    let response = match response_result {
        Ok(resp) => resp,
        Err(e) => {
            // Network error - create estimate
            info!("Axelar API network error: {}, creating estimate", e);
            return create_axelar_estimate(request);
        }
    };

    if !response.status().is_success() {
        // If API is not available, create estimate
        info!(
            "Axelar API returned {}, creating estimate",
            response.status()
        );
        return create_axelar_estimate(request);
    }

    let response_text = response.text().await.map_err(BridgeError::from)?;
    debug!("Axelar API response: {}", response_text);

    // Try to parse the response, but fall back to estimate if parsing fails
    match serde_json::from_str::<AxelarGasFeeResponse>(&response_text) {
        Ok(axelar_response) => {
            // Parse the total fee (this is in wei or smallest unit)
            let fee = parse_axelar_amount(&axelar_response.total_fee, &request.asset)?;

            // Axelar typically takes 5-20 minutes depending on the route
            let est_time = match (request.from_chain.as_str(), request.to_chain.as_str()) {
                // Cosmos-based chains are faster
                ("cosmos", _) | (_, "cosmos") | ("osmosis", _) | (_, "osmosis") => 300,
                // Cross-ecosystem transfers take longer
                (from, to)
                    if (from == "sui" || from == "stellar") || (to == "sui" || to == "stellar") =>
                {
                    1200
                }
                // Standard EVM-to-EVM transfers
                _ => 900,
            };

            let metadata = serde_json::json!({
                "total_fee": axelar_response.total_fee,
                "base_fee": axelar_response.base_fee,
                "execution_fee": axelar_response.execution_fee,
                "express_supported": axelar_response.is_express_supported,
                "gas_multiplier": axelar_response.gas_multiplier,
                "network": "Axelar",
                "architecture": "proof_of_stake_validator_network",
                "capabilities": ["GMP", "ITS", "multi_ecosystem"],
                "security_model": "validator_consensus",
                "route": format!("{} -> {}", request.from_chain, request.to_chain)
            });

            let quote = BridgeQuote {
                bridge: "Axelar".to_string(),
                fee,
                est_time,
                metadata: Some(metadata),
            };

            info!(
                "Axelar GMP quote retrieved: fee={:.6} {}, time={}s, ecosystems=multi",
                quote.fee, request.asset, quote.est_time
            );

            Ok(quote)
        }
        Err(_) => {
            info!("Failed to parse Axelar response, creating estimate");
            create_axelar_estimate(request)
        }
    }
}

/// Create an estimated Axelar quote when API is unavailable
/// Reflects Axelar's GMP and ITS capabilities
fn create_axelar_estimate(request: &BridgeQuoteRequest) -> Result<BridgeQuote, BridgeError> {
    // Verify we support this route
    map_chain_name(&request.from_chain)?;
    map_chain_name(&request.to_chain)?;
    map_asset_symbol(&request.asset)?;

    // Estimate fees based on asset type and cross-chain complexity
    let estimated_fee = match request.asset.to_uppercase().as_str() {
        // Gateway tokens (axl prefixed)
        "USDC" | "USDT" | "DAI" => 0.5, // $0.50 equivalent for axlUSDC transfers
        "ETH" | "WETH" => 0.001,        // ~$2-3 for axlWETH
        "WBTC" => 0.0001,               // ~$5-10 for axlWBTC
        "AXL" => 1.0,                   // 1 AXL token for gas
        _ => 0.002,                     // 0.2% of transfer for other tokens
    };

    // Estimate time based on ecosystem and chain combinations
    let est_time = match (request.from_chain.as_str(), request.to_chain.as_str()) {
        // Cosmos-to-Cosmos via IBC is fastest
        ("cosmos", "osmosis") | ("osmosis", "cosmos") => 60, // 1 minute
        // Cosmos ecosystem generally faster
        (from, to)
            if from.contains("cosmos")
                || from.contains("osmosis")
                || to.contains("cosmos")
                || to.contains("osmosis") =>
        {
            300
        } // 5 minutes
        // Cross-ecosystem transfers (EVM <-> Cosmos <-> Sui/Stellar)
        (from, to) if (from == "sui" || from == "stellar") || (to == "sui" || to == "stellar") => {
            1200
        } // 20 minutes
        // Standard EVM-to-EVM transfers
        _ => 900, // 15 minutes
    };

    let metadata = serde_json::json!({
        "estimated": true,
        "network": "Axelar",
        "architecture": "proof_of_stake_validator_network",
        "capabilities": ["GMP", "ITS", "multi_ecosystem"],
        "security_model": "validator_consensus",
        "supported_ecosystems": ["EVM", "Cosmos", "Sui", "Stellar", "XRPL"],
        "note": "Estimated quote - supports both Gateway tokens and Interchain tokens",
        "route": format!("{} -> {}", request.from_chain, request.to_chain)
    });

    let quote = BridgeQuote {
        bridge: "Axelar".to_string(),
        fee: estimated_fee,
        est_time,
        metadata: Some(metadata),
    };

    info!(
        "Axelar GMP/ITS estimate created: fee={:.6} {}, time={}s, ecosystems=multi",
        quote.fee, request.asset, quote.est_time
    );

    Ok(quote)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_axelar_quote_success() {
        // Test helper functions
        assert_eq!(map_chain_name("ethereum").unwrap(), "ethereum");
        assert_eq!(map_chain_name("cosmos").unwrap(), "cosmoshub");
        assert!(map_chain_name("invalid-chain").is_err());

        assert_eq!(map_asset_symbol("USDC").unwrap(), "uusdc");
        assert_eq!(map_asset_symbol("ETH").unwrap(), "eth-wei");
        assert_eq!(map_asset_symbol("AXL").unwrap(), "uaxl");

        assert_eq!(parse_axelar_amount("1000000", "USDC").unwrap(), 1.0);
        assert_eq!(parse_axelar_amount("500000", "USDC").unwrap(), 0.5);
    }

    #[test]
    fn test_axelar_estimate() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "ethereum".to_string(),
            to_chain: "cosmos".to_string(),
            amount: Some("1000000".to_string()),
            // recipient: None,
            slippage: 0.5,
        };

        let quote = create_axelar_estimate(&request).unwrap();
        assert_eq!(quote.bridge, "Axelar");
        assert!(quote.fee > 0.0);
        assert!(quote.est_time > 0);
    }

    #[test]
    fn test_chain_mapping() {
        assert!(map_chain_name("ethereum").is_ok());
        assert!(map_chain_name("cosmos").is_ok());
        assert!(map_chain_name("avalanche").is_ok());
        assert!(map_chain_name("blast").is_ok());
        assert!(map_chain_name("fraxtal").is_ok());
        assert!(map_chain_name("invalid-chain").is_err());
    }

    #[test]
    fn test_asset_mapping() {
        assert!(map_asset_symbol("USDC").is_ok());
        assert!(map_asset_symbol("ETH").is_ok());
        assert!(map_asset_symbol("AXL").is_ok());
        assert!(map_asset_symbol("USDE").is_ok());
        assert!(map_asset_symbol("SDAI").is_ok());
        // Unknown assets should not error, just use as-is
        assert!(map_asset_symbol("UNKNOWN").is_ok());
    }

    #[test]
    fn test_amount_parsing() {
        assert_eq!(parse_axelar_amount("1000000", "USDC").unwrap(), 1.0);
        assert_eq!(
            parse_axelar_amount("1000000000000000000", "ETH").unwrap(),
            1.0
        );
        assert_eq!(parse_axelar_amount("100000000", "WBTC").unwrap(), 1.0);
        assert!(parse_axelar_amount("invalid", "USDC").is_err());
    }
}
