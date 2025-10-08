use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, warn};

/// Gas price data for a specific chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasPrice {
    /// Chain identifier (ethereum, arbitrum, polygon, etc.)
    pub chain: String,
    /// Safe gas price in Gwei
    pub safe_gas_price: f64,
    /// Propose gas price in Gwei
    pub propose_gas_price: f64,
    /// Fast gas price in Gwei
    pub fast_gas_price: f64,
    /// Base fee in Gwei (EIP-1559)
    pub base_fee: Option<f64>,
    /// Priority fee in Gwei (EIP-1559)
    pub priority_fee: Option<f64>,
    /// ETH price in USD
    pub eth_price_usd: f64,
}

/// Etherscan V2 API response for gas oracle (gastracker module)
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum EtherscanV2Response {
    Success {
        status: String,
        result: EtherscanGasResult,
    },
    Error {
        message: String,
        result: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct EtherscanGasResult {
    safe_gas_price: String,
    propose_gas_price: String,
    fast_gas_price: String,
    #[serde(rename = "suggestBaseFee")]
    suggest_base_fee: Option<String>,
    #[serde(rename = "UsdPrice")]
    usd_price: Option<String>,
}

/// Etherscan V2 API response for proxy module (eth_gasPrice)
#[derive(Debug, Deserialize)]
struct EtherscanProxyResponse {
    result: String,
}

/// Chain name to chain ID mapping for Etherscan V2 API
fn get_chain_id(chain: &str) -> Option<u64> {
    match chain.to_lowercase().as_str() {
        // Ethereum
        "ethereum" | "eth" => Some(1),
        "sepolia" => Some(11155111),
        "holesky" => Some(17000),
        "hoodi" => Some(560048),

        // Abstract
        "abstract" => Some(2741),
        "abstract_sepolia" => Some(11124),

        // ApeChain
        "apechain" => Some(33139),
        "apechain_curtis" => Some(33111),

        // Arbitrum
        "arbitrum" | "arb" | "arbitrum_one" => Some(42161),
        "arbitrum_nova" => Some(42170),
        "arbitrum_sepolia" => Some(421614),

        // Avalanche
        "avalanche" | "avax" => Some(43114),
        "avalanche_fuji" | "fuji" => Some(43113),

        // Base
        "base" => Some(8453),
        "base_sepolia" => Some(84532),

        // Berachain
        "berachain" => Some(80094),
        "berachain_bepolia" => Some(80069),

        // BitTorrent Chain
        "bittorrent" | "btt" => Some(199),
        "bittorrent_testnet" => Some(1029),

        // Blast
        "blast" => Some(81457),
        "blast_sepolia" => Some(168587773),

        // BNB Smart Chain
        "bsc" | "bnb" | "binance" => Some(56),
        "bsc_testnet" | "bnb_testnet" => Some(97),

        // Celo
        "celo" => Some(42220),
        "celo_alfajores" => Some(44787),

        // Cronos
        "cronos" => Some(25),

        // Fraxtal
        "fraxtal" => Some(252),
        "fraxtal_testnet" => Some(2522),

        // Gnosis
        "gnosis" => Some(100),

        // HyperEVM
        "hyperevm" => Some(999),

        // Linea
        "linea" => Some(59144),
        "linea_sepolia" => Some(59141),

        // Mantle
        "mantle" => Some(5000),
        "mantle_sepolia" => Some(5003),

        // Memecore
        "memecore_testnet" => Some(43521),

        // Moonbeam
        "moonbeam" => Some(1284),
        "moonriver" => Some(1285),
        "moonbase" | "moonbase_alpha" => Some(1287),

        // Monad
        "monad_testnet" => Some(10143),

        // Optimism
        "optimism" | "op" => Some(10),
        "optimism_sepolia" | "op_sepolia" => Some(11155420),

        // Polygon
        "polygon" | "matic" => Some(137),
        "polygon_amoy" | "amoy" => Some(80002),

        // Katana
        "katana" => Some(747474),
        "katana_bokuto" => Some(737373),

        // Sei
        "sei" => Some(1329),
        "sei_testnet" => Some(1328),

        // Scroll
        "scroll" => Some(534352),
        "scroll_sepolia" => Some(534351),

        // Sonic
        "sonic" => Some(146),
        "sonic_testnet" => Some(14601),

        // Sophon
        "sophon" => Some(50104),
        "sophon_sepolia" => Some(531050104),

        // Swellchain
        "swellchain" => Some(1923),
        "swellchain_testnet" => Some(1924),

        // Taiko
        "taiko" => Some(167000),
        "taiko_hoodi" => Some(167012),

        // Unichain
        "unichain" => Some(130),
        "unichain_sepolia" => Some(1301),

        // World
        "world" => Some(480),
        "world_sepolia" => Some(4801),

        // XDC
        "xdc" => Some(50),
        "xdc_apothem" => Some(51),

        // zkSync
        "zksync" => Some(324),
        "zksync_sepolia" => Some(300),

        // opBNB
        "opbnb" => Some(204),
        "opbnb_testnet" => Some(5611),

        _ => None,
    }
}

/// Gas price service for fetching real-time gas prices using Etherscan V2 API
pub struct GasPriceService {
    client: Client,
    etherscan_v2_api_key: Option<String>,
}

impl GasPriceService {
    /// Create a new gas price service with Etherscan V2 API key
    pub fn new(etherscan_v2_api_key: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            etherscan_v2_api_key,
        }
    }

    /// Get gas price for any supported chain using Etherscan V2 API
    pub async fn get_gas_price(&self, chain: &str) -> Result<GasPrice, String> {
        let chain_id =
            get_chain_id(chain).ok_or_else(|| format!("Unsupported chain: {}", chain))?;

        // Try gastracker module first (available on some chains like Ethereum)
        if let Ok(gas_price) = self.try_gastracker_module(chain, chain_id).await {
            return Ok(gas_price);
        }

        // Fall back to proxy module (eth_gasPrice) - available on all chains
        self.try_proxy_module(chain, chain_id).await
    }

    /// Try to get gas price using gastracker module
    async fn try_gastracker_module(&self, chain: &str, chain_id: u64) -> Result<GasPrice, String> {
        let url = if let Some(api_key) = &self.etherscan_v2_api_key {
            format!(
                "https://api.etherscan.io/v2/api?chainid={}&module=gastracker&action=gasoracle&apikey={}",
                chain_id, api_key
            )
        } else {
            format!(
                "https://api.etherscan.io/v2/api?chainid={}&module=gastracker&action=gasoracle",
                chain_id
            )
        };

        match self.client.get(&url).send().await {
            Ok(response) => {
                let status = response.status();
                let response_text = response
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read response body: {}", e))?;

                if !status.is_success() {
                    error!(
                        "Etherscan API returned status {}: {}",
                        status, response_text
                    );
                    return Err(format!(
                        "Etherscan API error: {} - {}",
                        status, response_text
                    ));
                }

                match serde_json::from_str::<EtherscanV2Response>(&response_text) {
                    Ok(response) => match response {
                        EtherscanV2Response::Success { status, result, .. } => {
                            if status != "1" {
                                return Err(
                                    "Gastracker module returned non-success status".to_string()
                                );
                            }

                            let safe_gas_price = result
                                .safe_gas_price
                                .parse::<f64>()
                                .unwrap_or_else(|_| self.get_fallback_gas_price_value(chain));
                            let propose_gas_price =
                                result.propose_gas_price.parse::<f64>().unwrap_or_else(|_| {
                                    self.get_fallback_gas_price_value(chain) * 1.25
                                });
                            let fast_gas_price = result
                                .fast_gas_price
                                .parse::<f64>()
                                .unwrap_or_else(|_| self.get_fallback_gas_price_value(chain) * 1.5);
                            let base_fee =
                                result.suggest_base_fee.and_then(|s| s.parse::<f64>().ok());
                            let eth_price_usd = result
                                .usd_price
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(3000.0);

                            Ok(GasPrice {
                                chain: chain.to_lowercase(),
                                safe_gas_price,
                                propose_gas_price,
                                fast_gas_price,
                                base_fee,
                                priority_fee: None,
                                eth_price_usd,
                            })
                        }
                        EtherscanV2Response::Error {
                            message, result, ..
                        } => Err(format!("Gastracker module error: {} - {}", message, result)),
                    },
                    Err(e) => {
                        error!(
                            "Failed to parse Etherscan V2 response for {}: {}. Response: {}",
                            chain, e, response_text
                        );
                        Err(format!("Failed to parse Etherscan V2 response: {}", e))
                    }
                }
            }
            Err(e) => {
                error!("Failed to fetch gas price for {}: {}", chain, e);
                Err(format!("Failed to fetch gas price: {}", e))
            }
        }
    }

    /// Try to get gas price using proxy module (eth_gasPrice)
    async fn try_proxy_module(&self, chain: &str, chain_id: u64) -> Result<GasPrice, String> {
        let url = if let Some(api_key) = &self.etherscan_v2_api_key {
            format!(
                "https://api.etherscan.io/v2/api?chainid={}&module=proxy&action=eth_gasPrice&apikey={}",
                chain_id, api_key
            )
        } else {
            format!(
                "https://api.etherscan.io/v2/api?chainid={}&module=proxy&action=eth_gasPrice",
                chain_id
            )
        };

        match self.client.get(&url).send().await {
            Ok(response) => {
                let status = response.status();
                let response_text = response
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read response body: {}", e))?;

                if !status.is_success() {
                    error!(
                        "Etherscan proxy API returned status {}: {}",
                        status, response_text
                    );
                    return Err(format!(
                        "Etherscan API error: {} - {}",
                        status, response_text
                    ));
                }

                match serde_json::from_str::<EtherscanProxyResponse>(&response_text) {
                    Ok(data) => {
                        // Parse hex gas price (in wei) to gwei
                        let gas_price_wei =
                            u64::from_str_radix(data.result.trim_start_matches("0x"), 16)
                                .map_err(|e| format!("Failed to parse hex gas price: {}", e))?;

                        // Convert wei to gwei (1 gwei = 1e9 wei)
                        let gas_price_gwei = gas_price_wei as f64 / 1_000_000_000.0;

                        Ok(GasPrice {
                            chain: chain.to_lowercase(),
                            safe_gas_price: gas_price_gwei,
                            propose_gas_price: gas_price_gwei * 1.1,
                            fast_gas_price: gas_price_gwei * 1.2,
                            base_fee: Some(gas_price_gwei * 0.9),
                            priority_fee: Some(gas_price_gwei * 0.1),
                            eth_price_usd: 3000.0, // Default, will be overridden by token price service
                        })
                    }
                    Err(e) => {
                        error!(
                            "Failed to parse Etherscan proxy response for {}: {}. Response: {}",
                            chain, e, response_text
                        );
                        Err(format!("Failed to parse Etherscan proxy response: {}", e))
                    }
                }
            }
            Err(e) => {
                error!("Failed to fetch gas price via proxy for {}: {}", chain, e);
                Err(format!("Failed to fetch gas price: {}", e))
            }
        }
    }

    /// Get fallback gas price value for a chain (in Gwei)
    fn get_fallback_gas_price_value(&self, chain: &str) -> f64 {
        match chain.to_lowercase().as_str() {
            // Ethereum and testnets
            "ethereum" | "eth" | "sepolia" | "holesky" | "hoodi" => 20.0,

            // Layer 2s - typically very low gas
            "arbitrum" | "arb" | "arbitrum_one" | "arbitrum_nova" | "arbitrum_sepolia" => 0.1,
            "optimism" | "op" | "optimism_sepolia" | "op_sepolia" => 0.001,
            "base" | "base_sepolia" => 0.001,
            "blast" | "blast_sepolia" => 0.001,
            "scroll" | "scroll_sepolia" => 0.001,
            "zksync" | "zksync_sepolia" => 0.1,
            "linea" | "linea_sepolia" => 0.5,
            "mantle" | "mantle_sepolia" => 0.02,
            "opbnb" | "opbnb_testnet" => 0.001,

            // Alt Layer 1s
            "polygon" | "matic" | "polygon_amoy" | "amoy" => 30.0,
            "avalanche" | "avax" | "avalanche_fuji" | "fuji" => 25.0,
            "bsc" | "bnb" | "binance" | "bsc_testnet" | "bnb_testnet" => 3.0,
            "celo" | "celo_alfajores" => 5.0,
            "gnosis" => 2.0,
            "moonbeam" | "moonriver" | "moonbase" | "moonbase_alpha" => 100.0,
            "cronos" => 2000.0,

            // Newer chains
            "abstract" | "abstract_sepolia" => 0.001,
            "apechain" | "apechain_curtis" => 0.1,
            "berachain" | "berachain_bepolia" => 0.1,
            "bittorrent" | "btt" | "bittorrent_testnet" => 50.0,
            "fraxtal" | "fraxtal_testnet" => 0.001,
            "hyperevm" => 0.1,
            "katana" | "katana_bokuto" => 0.1,
            "memecore_testnet" => 1.0,
            "monad_testnet" => 0.1,
            "sei" | "sei_testnet" => 0.1,
            "sonic" | "sonic_testnet" => 0.1,
            "sophon" | "sophon_sepolia" => 0.1,
            "swellchain" | "swellchain_testnet" => 0.1,
            "taiko" | "taiko_hoodi" => 0.01,
            "unichain" | "unichain_sepolia" => 0.001,
            "world" | "world_sepolia" => 0.1,
            "xdc" | "xdc_apothem" => 0.1,

            // Default for unknown chains
            _ => 20.0,
        }
    }

    /// Get fallback gas price when API is unavailable
    #[allow(dead_code)]
    fn get_fallback_gas_price(&self, chain: &str) -> GasPrice {
        let base_gas = self.get_fallback_gas_price_value(chain);

        warn!("Using fallback gas price for {}: {} Gwei", chain, base_gas);

        GasPrice {
            chain: chain.to_lowercase(),
            safe_gas_price: base_gas,
            propose_gas_price: base_gas * 1.25,
            fast_gas_price: base_gas * 1.5,
            base_fee: Some(base_gas * 0.9),
            priority_fee: Some(base_gas * 0.1),
            eth_price_usd: 3000.0,
        }
    }
}

/// Estimate gas cost in USD for a transaction
pub fn estimate_gas_cost_usd(gas_price: &GasPrice, gas_limit: u64, use_fast: bool) -> f64 {
    let gas_price_gwei = if use_fast {
        gas_price.fast_gas_price
    } else {
        gas_price.propose_gas_price
    };

    // Convert Gwei to ETH: 1 ETH = 1,000,000,000 Gwei
    let gas_price_eth = gas_price_gwei / 1_000_000_000.0;

    // Calculate total gas cost in ETH
    let gas_cost_eth = gas_price_eth * gas_limit as f64;

    // Convert to USD
    gas_cost_eth * gas_price.eth_price_usd
}

/// Standard gas limits for common bridge operations
pub mod gas_limits {
    /// Bridge deposit gas limit (source chain)
    pub const BRIDGE_DEPOSIT: u64 = 150_000;

    /// Bridge withdrawal gas limit (destination chain)
    pub const BRIDGE_WITHDRAWAL: u64 = 100_000;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_gas_cost_usd() {
        let gas_price = GasPrice {
            chain: "ethereum".to_string(),
            safe_gas_price: 20.0,
            propose_gas_price: 25.0,
            fast_gas_price: 30.0,
            base_fee: Some(18.0),
            priority_fee: Some(2.0),
            eth_price_usd: 3000.0,
        };

        // Test with propose gas price
        let cost = estimate_gas_cost_usd(&gas_price, 150_000, false);
        // 25 Gwei * 150,000 gas = 3,750,000 Gwei = 0.00375 ETH
        // 0.00375 ETH * $3000 = $11.25
        assert!((cost - 11.25).abs() < 0.01);

        // Test with fast gas price
        let cost_fast = estimate_gas_cost_usd(&gas_price, 150_000, true);
        // 30 Gwei * 150,000 gas = 4,500,000 Gwei = 0.0045 ETH
        // 0.0045 ETH * $3000 = $13.50
        assert!((cost_fast - 13.5).abs() < 0.01);
    }

    #[test]
    fn test_fallback_gas_prices() {
        let service = GasPriceService::new(None);

        let eth_gas = service.get_fallback_gas_price("ethereum");
        assert_eq!(eth_gas.chain, "ethereum");
        assert!(eth_gas.safe_gas_price > 0.0);

        let arb_gas = service.get_fallback_gas_price("arbitrum");
        assert_eq!(arb_gas.chain, "arbitrum");
        assert!(arb_gas.safe_gas_price > 0.0);
    }

    #[test]
    fn test_chain_id_mapping() {
        assert_eq!(get_chain_id("ethereum"), Some(1));
        assert_eq!(get_chain_id("arbitrum"), Some(42161));
        assert_eq!(get_chain_id("optimism"), Some(10));
        assert_eq!(get_chain_id("polygon"), Some(137));
        assert_eq!(get_chain_id("base"), Some(8453));
        assert_eq!(get_chain_id("scroll"), Some(534352));
        assert_eq!(get_chain_id("blast"), Some(81457));
    }
}
