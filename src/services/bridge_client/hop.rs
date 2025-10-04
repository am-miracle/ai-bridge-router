use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::{format_liquidity, get_cached_quote, retry_request};
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};

/// Hop API endpoint - using v1 API
/// Documentation: https://docs.hop.exchange/v1
const HOP_API_BASE: &str = "https://api.hop.exchange";

/// Network configuration for Hop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HopNetwork {
    pub name: String,
    pub is_testnet: bool,
}

/// Available route from Hop API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HopRoute {
    pub token: String,
    #[serde(rename = "sourceChainSlug")]
    pub source_chain: String,
    #[serde(rename = "sourceChainId")]
    pub source_chain_id: u64,
    #[serde(rename = "destinationChainSlug")]
    pub destination_chain: String,
    #[serde(rename = "destinationChainId")]
    pub destination_chain_id: u64,
}

/// Hop configuration with dynamic chain and token support
#[derive(Debug, Clone)]
pub struct HopConfig {
    pub network: HopNetwork,
    pub available_routes: Arc<RwLock<Vec<HopRoute>>>,
    pub supported_tokens: Arc<RwLock<Vec<String>>>,
    pub supported_chains: Arc<RwLock<Vec<String>>>,
    pub last_updated: Arc<RwLock<Option<std::time::SystemTime>>>,
}

/// Hop quote request structure
#[derive(Debug, Serialize)]
struct HopQuoteRequest {
    #[serde(rename = "fromChain")]
    from_chain: String,
    #[serde(rename = "toChain")]
    to_chain: String,
    token: String,
    amount: String,
    #[serde(rename = "slippage")]
    slippage: f64,
    network: Option<String>,
}

/// Hop API quote response structure
/// Based on Hop v1 API documentation: https://docs.hop.exchange/v1
///
/// Example response:
/// ```json
/// {
///   "amountIn": "1000000",
///   "slippage": 0.5,
///   "amountOutMin": "743633",
///   "destinationAmountOutMin": "742915",
///   "bonderFee": "250515",
///   "estimatedRecieved": "747908",
///   "deadline": 1679862208,
///   "destinationDeadline": 1679862208
/// }
/// ```
#[derive(Debug, Deserialize)]
struct HopQuoteResponse {
    #[serde(rename = "amountIn")]
    amount_in: String,
    slippage: f64,
    #[serde(rename = "amountOutMin")]
    amount_out_min: String,
    #[serde(rename = "destinationAmountOutMin")]
    destination_amount_out_min: Option<String>,
    #[serde(rename = "bonderFee")]
    bonder_fee: String,
    #[serde(rename = "estimatedRecieved")]
    estimated_received: String,
    deadline: u64,
    #[serde(rename = "destinationDeadline")]
    destination_deadline: Option<u64>,
}

impl HopConfig {
    /// Create a new Hop configuration
    pub fn new(network: HopNetwork) -> Self {
        Self {
            network,
            available_routes: Arc::new(RwLock::new(Vec::new())),
            supported_tokens: Arc::new(RwLock::new(Vec::new())),
            supported_chains: Arc::new(RwLock::new(Vec::new())),
            last_updated: Arc::new(RwLock::new(None)),
        }
    }

    /// Fetch available routes from Hop API
    pub async fn fetch_available_routes(
        &self,
        config: &BridgeClientConfig,
    ) -> Result<(), BridgeError> {
        let cache_key = format!("hop:routes:{}", self.network.name);

        // Check if we have cached routes that are still fresh (cache for 1 hour)
        if let Some(cache_client) = &config.cache
            && let Ok(Some(cached_routes)) =
                cache_client.get_cache::<Vec<HopRoute>>(&cache_key).await
        {
            let mut routes = self.available_routes.write().await;
            *routes = cached_routes;

            // Update supported tokens and chains
            self.update_supported_lists().await;

            let mut last_updated = self.last_updated.write().await;
            *last_updated = Some(std::time::SystemTime::now());

            info!("Loaded {} Hop routes from cache", routes.len());
            return Ok(());
        }

        // Fetch fresh routes from API
        let url = format!("{}/v1/available-routes", HOP_API_BASE);
        let mut query_params = vec![];

        if self.network.is_testnet {
            query_params.push(("network", "goerli"));
        }

        let response = tokio::time::timeout(
            config.timeout,
            config.client.get(&url).query(&query_params).send(),
        )
        .await
        .map_err(|_| BridgeError::Timeout {
            timeout_ms: config.timeout.as_millis() as u64,
        })?
        .map_err(BridgeError::from)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BridgeError::BadResponse {
                message: format!("HTTP {}: {}", status, error_text),
            });
        }

        let response_text = response.text().await.map_err(BridgeError::from)?;
        let routes: Vec<HopRoute> =
            serde_json::from_str(&response_text).map_err(|e| BridgeError::BadResponse {
                message: format!(
                    "Failed to parse Hop routes response: {}. Response: {}",
                    e, response_text
                ),
            })?;

        // Update internal state
        {
            let mut available_routes = self.available_routes.write().await;
            *available_routes = routes.clone();
        }

        self.update_supported_lists().await;

        let mut last_updated = self.last_updated.write().await;
        *last_updated = Some(std::time::SystemTime::now());

        // Cache the routes
        if let Some(cache_client) = &config.cache {
            if let Err(e) = cache_client.set_cache(&cache_key, &routes, 3600).await {
                warn!("Failed to cache Hop routes: {}", e);
            } else {
                info!("Cached {} Hop routes for 1 hour", routes.len());
            }
        }

        info!("Fetched {} available routes from Hop API", routes.len());
        Ok(())
    }

    /// Update supported tokens and chains from available routes
    async fn update_supported_lists(&self) {
        let routes = self.available_routes.read().await;

        let mut tokens = std::collections::HashSet::new();
        let mut chains = std::collections::HashSet::new();

        for route in routes.iter() {
            tokens.insert(route.token.clone());
            chains.insert(route.source_chain.clone());
            chains.insert(route.destination_chain.clone());
        }

        {
            let mut supported_tokens = self.supported_tokens.write().await;
            *supported_tokens = tokens.into_iter().collect();
            supported_tokens.sort();
        }

        {
            let mut supported_chains = self.supported_chains.write().await;
            *supported_chains = chains.into_iter().collect();
            supported_chains.sort();
        }
    }

    /// Check if a route is supported
    pub async fn is_route_supported(&self, from_chain: &str, to_chain: &str, token: &str) -> bool {
        let routes = self.available_routes.read().await;
        routes.iter().any(|route| {
            route.source_chain == from_chain
                && route.destination_chain == to_chain
                && route.token == token
        })
    }

    /// Get supported tokens
    #[allow(dead_code)]
    pub async fn get_supported_tokens(&self) -> Vec<String> {
        self.supported_tokens.read().await.clone()
    }

    /// Get supported chains
    #[allow(dead_code)]
    pub async fn get_supported_chains(&self) -> Vec<String> {
        self.supported_chains.read().await.clone()
    }

    /// Check if routes need refresh (older than 1 hour)
    pub async fn needs_refresh(&self) -> bool {
        let last_updated = self.last_updated.read().await;
        match *last_updated {
            Some(time) => {
                match time.elapsed() {
                    Ok(elapsed) => elapsed.as_secs() > 3600, // 1 hour
                    Err(_) => true, // If we can't get elapsed time, refresh
                }
            }
            None => true, // Never updated, needs refresh
        }
    }
}

/// Chain name normalization for Hop API
/// Maps common chain aliases to Hop's expected chain identifiers
/// Based on Hop's supported chains: https://docs.hop.exchange/v2
fn normalize_chain_name(chain: &str) -> String {
    match chain.to_lowercase().as_str() {
        "eth" | "mainnet" | "ethereum" => "ethereum".to_string(),
        "matic" | "polygon" => "polygon".to_string(),
        "arbitrum-one" | "arbitrum" => "arbitrum".to_string(),
        "opt" | "optimism" => "optimism".to_string(),
        "xdai" | "gnosis" => "gnosis".to_string(),
        "bsc" | "binance" | "binance-smart-chain" => "bsc".to_string(),
        "avalanche" | "avax" => "avalanche".to_string(),
        "base" => "base".to_string(),
        "linea" => "linea".to_string(),
        "scroll" => "scroll".to_string(),
        _ => chain.to_lowercase(),
    }
}

/// Token symbol normalization for Hop API
/// Maps wrapped tokens to their canonical forms and ensures proper casing
fn normalize_token_symbol(asset: &str) -> String {
    match asset.to_uppercase().as_str() {
        "WETH" => "ETH".to_string(),
        "WMATIC" => "MATIC".to_string(),
        "WAVAX" => "AVAX".to_string(),
        "WBNB" => "BNB".to_string(),
        "WFTM" => "FTM".to_string(),
        "USDC.E" => "USDC".to_string(), // Avalanche USDC
        "USDT.E" => "USDT".to_string(), // Avalanche USDT
        _ => asset.to_uppercase(),
    }
}

/// Convert string amount to float with proper decimal handling
/// Handles amounts in smallest units (e.g., wei, satoshis) and converts to human-readable format
fn parse_amount_to_float(amount_str: &str, decimals: u8) -> Result<f64, BridgeError> {
    let amount = amount_str
        .parse::<u128>()
        .map_err(|_| BridgeError::BadResponse {
            message: format!("Invalid amount format: {}", amount_str),
        })?;

    let divisor = 10_u128.pow(decimals as u32);
    let result = amount as f64 / divisor as f64;

    // Validate the result is reasonable (not NaN or infinite)
    if result.is_nan() || result.is_infinite() {
        return Err(BridgeError::BadResponse {
            message: format!("Invalid amount calculation result: {}", result),
        });
    }

    Ok(result)
}

/// Get a quote from Hop bridge
///
/// This function fetches a quote from Hop's API for cross-chain transfers.
/// It handles route validation, caching, and error recovery.
///
/// # Arguments
/// * `request` - Bridge quote request with asset, chains, and amount
/// * `config` - Bridge client configuration with timeout and retry settings
///
/// # Returns
/// * `Ok(BridgeQuote)` - Successfully retrieved quote with fee, time, and liquidity
/// * `Err(BridgeError)` - Various error conditions (timeout, unsupported route, etc.)
///
/// # Example
/// ```rust
/// use bridge_router::models::bridge::{BridgeQuoteRequest, BridgeClientConfig};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let request = BridgeQuoteRequest {
///     asset: "USDC".to_string(),
///     from_chain: "ethereum".to_string(),
///     to_chain: "polygon".to_string(),
///     amount: Some("1000000".to_string()), // 1 USDC
/// };
///
/// let config = BridgeClientConfig::new();
/// let quote = bridge_router::services::bridge_client::hop::get_quote(&request, &config).await?;
/// // Returns: BridgeQuote { bridge: "Hop", fee: 0.002, est_time: 300, ... }
/// # Ok(())
/// # }
/// ```
pub async fn get_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    // Normalize chain and token names
    let from_chain = normalize_chain_name(&request.from_chain);
    let to_chain = normalize_chain_name(&request.to_chain);
    let token = normalize_token_symbol(&request.asset);

    // Check if we have Hop configuration and if route is supported
    if let Some(hop_config) = &config.hop_config {
        // Ensure we have fresh route data
        if hop_config.needs_refresh().await
            && let Err(e) = hop_config.fetch_available_routes(config).await
        {
            warn!("Failed to fetch Hop routes: {}", e);
            // Continue with quote attempt even if route fetch fails
        }

        // Check if route is supported
        if !hop_config
            .is_route_supported(&from_chain, &to_chain, &token)
            .await
        {
            return Err(BridgeError::UnsupportedRoute {
                from_chain: request.from_chain.clone(),
                to_chain: request.to_chain.clone(),
            });
        }
    }

    let cache_key = format!(
        "hop:{}:{}:{}:{}",
        token,
        from_chain,
        to_chain,
        request.amount.as_deref().unwrap_or("1000000") // Default amount
    );

    get_cached_quote(&cache_key, &config.cache, || {
        fetch_hop_quote(request, config)
    })
    .await
}

/// Fetch quote directly from Hop API
async fn fetch_hop_quote(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    retry_request(
        || fetch_hop_quote_once(request, config),
        config.retries,
        "Hop API call",
    )
    .await
}

/// Single attempt to fetch Hop quote
async fn fetch_hop_quote_once(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Result<BridgeQuote, BridgeError> {
    debug!(
        "Fetching Hop quote for {}/{} -> {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Normalize chain names and token symbol
    let from_chain = normalize_chain_name(&request.from_chain);
    let to_chain = normalize_chain_name(&request.to_chain);
    let token = normalize_token_symbol(&request.asset);

    // Default amount based on token type (in smallest units)
    let amount = request.amount.clone().unwrap_or_else(|| {
        match token.as_str() {
            "USDC" | "USDT" => "1000000".to_string(), // 1 USDC (6 decimals)
            "DAI" | "ETH" | "MATIC" => "1000000000000000000".to_string(), // 1 token (18 decimals)
            _ => "1000000000000000000".to_string(),   // Default to 18 decimals
        }
    });

    // Determine network parameter
    let network = config
        .hop_config
        .as_ref()
        .map(|hop_config| {
            if hop_config.network.is_testnet {
                "goerli"
            } else {
                "mainnet"
            }
        })
        .map(|s| s.to_string());

    let hop_request = HopQuoteRequest {
        from_chain: from_chain.clone(),
        to_chain: to_chain.clone(),
        token: token.clone(),
        amount: amount.clone(),
        slippage: 0.5, // Default 0.5% slippage as per API docs
        network,
    };

    // Build URL with query parameters
    let url = format!("{}/v1/quote", HOP_API_BASE);

    info!("Requesting Hop quote from: {}", url);

    let slippage_str = hop_request.slippage.to_string();
    let mut query_params = vec![
        ("fromChain", hop_request.from_chain.as_str()),
        ("toChain", hop_request.to_chain.as_str()),
        ("token", hop_request.token.as_str()),
        ("amount", hop_request.amount.as_str()),
        ("slippage", &slippage_str),
    ];

    // Add network parameter if specified
    if let Some(network) = &hop_request.network {
        query_params.push(("network", network));
    }

    let response = tokio::time::timeout(
        config.timeout,
        config.client.get(&url).query(&query_params).send(),
    )
    .await
    .map_err(|_| BridgeError::Timeout {
        timeout_ms: config.timeout.as_millis() as u64,
    })?
    .map_err(BridgeError::from)?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(BridgeError::BadResponse {
            message: format!("HTTP {}: {}", status, error_text),
        });
    }

    let response_text = response.text().await.map_err(BridgeError::from)?;
    debug!("Hop API response: {}", response_text);

    let hop_response: HopQuoteResponse =
        serde_json::from_str(&response_text).map_err(|e| BridgeError::BadResponse {
            message: format!(
                "Failed to parse Hop response: {}. Response: {}",
                e, response_text
            ),
        })?;

    // Determine token decimals based on token type
    let decimals = match token.as_str() {
        "USDC" | "USDT" => 6,
        "DAI" | "ETH" | "MATIC" | "AVAX" | "BNB" => 18,
        _ => 18, // Default to 18 decimals for most ERC-20 tokens
    };

    // Parse bonder fee (this is the total fee for the transfer in token units)
    let total_fee = parse_amount_to_float(&hop_response.bonder_fee, decimals)?;

    // Estimate time based on chain combinations and Hop's typical settlement times
    let est_time = match (from_chain.as_str(), to_chain.as_str()) {
        ("ethereum", _) | (_, "ethereum") => 900, // 15 minutes to/from mainnet (L1)
        ("polygon", "arbitrum") | ("arbitrum", "polygon") => 300, // 5 minutes between L2s
        ("optimism", "arbitrum") | ("arbitrum", "optimism") => 300, // 5 minutes between L2s
        ("base", "arbitrum") | ("arbitrum", "base") => 300, // 5 minutes between L2s
        _ => 600,                                 // 10 minutes default for other combinations
    };

    // Parse estimated received amount for liquidity calculation
    let estimated_received = parse_amount_to_float(&hop_response.estimated_received, decimals)
        .map_err(|e| {
            error!("Failed to parse estimated received amount: {}", e);
            e
        })?;

    // Calculate liquidity based on estimated received amount
    // Use a conservative multiplier to estimate available liquidity
    let liquidity_multiplier = match token.as_str() {
        "USDC" | "USDT" => 10000.0, // Stablecoins typically have high liquidity
        "ETH" => 5000.0,            // ETH has good liquidity
        _ => 1000.0,                // Default multiplier for other tokens
    };

    let liquidity_amount = estimated_received * liquidity_multiplier;
    let liquidity = format_liquidity(liquidity_amount, &token);

    // Create metadata with Hop-specific information for debugging and transparency
    let metadata = serde_json::json!({
        "amount_in": hop_response.amount_in,
        "amount_out_min": hop_response.amount_out_min,
        "destination_amount_out_min": hop_response.destination_amount_out_min.unwrap_or_else(|| "null".to_string()),
        "estimated_received": hop_response.estimated_received,
        "slippage": hop_response.slippage,
        "deadline": hop_response.deadline,
        "destination_deadline": hop_response.destination_deadline.unwrap_or(0),
        "route": format!("{} -> {}", from_chain, to_chain),
        "network": hop_request.network.unwrap_or_else(|| "mainnet".to_string()),
        "token_decimals": decimals,
        "from_chain": from_chain,
        "to_chain": to_chain
    });

    let quote = BridgeQuote {
        bridge: "Hop".to_string(),
        fee: total_fee,
        est_time,
        liquidity,
        score: None, // Will be calculated later in the quotes endpoint
        metadata: Some(metadata),
    };

    info!(
        "Hop quote retrieved successfully: {} {} from {} to {} - fee={:.6} {}, time={}s, liquidity={}",
        request.amount.as_deref().unwrap_or("default"),
        token,
        from_chain,
        to_chain,
        quote.fee,
        token,
        quote.est_time,
        quote.liquidity
    );

    Ok(quote)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hop_config_creation() {
        let network = HopNetwork {
            name: "mainnet".to_string(),
            is_testnet: false,
        };
        let config = HopConfig::new(network);

        assert_eq!(config.network.name, "mainnet");
        assert!(!config.network.is_testnet);
        assert!(config.needs_refresh().await);
    }

    #[tokio::test]
    async fn test_hop_routes_fetch() {
        // Test the HopConfig creation and basic functionality
        let network = HopNetwork {
            name: "mainnet".to_string(),
            is_testnet: false,
        };
        let config = HopConfig::new(network);

        assert_eq!(config.network.name, "mainnet");
        assert!(!config.network.is_testnet);
        assert!(config.needs_refresh().await);

        // Test that we can get empty supported lists initially
        let tokens = config.get_supported_tokens().await;
        let chains = config.get_supported_chains().await;

        assert!(tokens.is_empty());
        assert!(chains.is_empty());
    }

    #[test]
    fn test_chain_normalization() {
        // Ethereum variations
        assert_eq!(normalize_chain_name("ethereum"), "ethereum");
        assert_eq!(normalize_chain_name("ETH"), "ethereum");
        assert_eq!(normalize_chain_name("mainnet"), "ethereum");

        // Polygon variations
        assert_eq!(normalize_chain_name("polygon"), "polygon");
        assert_eq!(normalize_chain_name("MATIC"), "polygon");

        // Arbitrum variations
        assert_eq!(normalize_chain_name("arbitrum-one"), "arbitrum");
        assert_eq!(normalize_chain_name("arbitrum"), "arbitrum");

        // Optimism variations
        assert_eq!(normalize_chain_name("optimism"), "optimism");
        assert_eq!(normalize_chain_name("OPT"), "optimism");

        // Gnosis variations
        assert_eq!(normalize_chain_name("gnosis"), "gnosis");
        assert_eq!(normalize_chain_name("XDAI"), "gnosis");

        // New chains
        assert_eq!(normalize_chain_name("base"), "base");
        assert_eq!(normalize_chain_name("linea"), "linea");
        assert_eq!(normalize_chain_name("scroll"), "scroll");
        assert_eq!(normalize_chain_name("avalanche"), "avalanche");
        assert_eq!(normalize_chain_name("AVAX"), "avalanche");

        // Case insensitive
        assert_eq!(normalize_chain_name("ETHEREUM"), "ethereum");
        assert_eq!(normalize_chain_name("Polygon"), "polygon");
    }

    #[test]
    fn test_token_normalization() {
        // Stablecoins
        assert_eq!(normalize_token_symbol("USDC"), "USDC");
        assert_eq!(normalize_token_symbol("usdc"), "USDC");
        assert_eq!(normalize_token_symbol("USDT"), "USDT");
        assert_eq!(normalize_token_symbol("DAI"), "DAI");

        // Wrapped tokens
        assert_eq!(normalize_token_symbol("WETH"), "ETH");
        assert_eq!(normalize_token_symbol("weth"), "ETH");
        assert_eq!(normalize_token_symbol("WMATIC"), "MATIC");
        assert_eq!(normalize_token_symbol("wmatic"), "MATIC");
        assert_eq!(normalize_token_symbol("WAVAX"), "AVAX");
        assert_eq!(normalize_token_symbol("WBNB"), "BNB");
        assert_eq!(normalize_token_symbol("WFTM"), "FTM");

        // Avalanche-specific tokens
        assert_eq!(normalize_token_symbol("USDC.E"), "USDC");
        assert_eq!(normalize_token_symbol("USDT.E"), "USDT");

        // Case insensitive
        assert_eq!(normalize_token_symbol("usdc"), "USDC");
        assert_eq!(normalize_token_symbol("Usdc"), "USDC");
    }

    #[test]
    fn test_amount_parsing() {
        // USDC/USDT (6 decimals)
        assert_eq!(parse_amount_to_float("1000000", 6).unwrap(), 1.0);
        assert_eq!(parse_amount_to_float("500000", 6).unwrap(), 0.5);
        assert_eq!(parse_amount_to_float("2500000", 6).unwrap(), 2.5);

        // ETH/DAI (18 decimals)
        assert_eq!(
            parse_amount_to_float("1000000000000000000", 18).unwrap(),
            1.0
        );
        assert_eq!(
            parse_amount_to_float("500000000000000000", 18).unwrap(),
            0.5
        );
        assert_eq!(
            parse_amount_to_float("2500000000000000000", 18).unwrap(),
            2.5
        );

        // Edge cases
        assert_eq!(parse_amount_to_float("0", 6).unwrap(), 0.0);
        assert_eq!(parse_amount_to_float("1", 6).unwrap(), 0.000001);

        // Error cases
        assert!(parse_amount_to_float("invalid", 18).is_err());
        assert!(parse_amount_to_float("", 18).is_err());
        assert!(parse_amount_to_float("-1", 18).is_err());
    }

    #[tokio::test]
    async fn test_hop_quote_response_parsing() {
        let response_json = r#"{
            "amountIn": "1000000",
            "slippage": 0.5,
            "amountOutMin": "743633",
            "destinationAmountOutMin": "742915",
            "bonderFee": "250515",
            "estimatedRecieved": "747908",
            "deadline": 1679862208,
            "destinationDeadline": 1679862208
        }"#;

        let response: HopQuoteResponse = serde_json::from_str(response_json).unwrap();

        // Test all fields are parsed correctly
        assert_eq!(response.amount_in, "1000000");
        assert_eq!(response.slippage, 0.5);
        assert_eq!(response.amount_out_min, "743633");
        assert_eq!(
            response.destination_amount_out_min,
            Some("742915".to_string())
        );
        assert_eq!(response.bonder_fee, "250515");
        assert_eq!(response.estimated_received, "747908");
        assert_eq!(response.deadline, 1679862208);
        assert_eq!(response.destination_deadline, Some(1679862208));

        // Test fee calculation (250515 / 10^6 = 0.250515 USDC)
        let fee = parse_amount_to_float(&response.bonder_fee, 6).unwrap();
        assert_eq!(fee, 0.250515);

        // Test estimated received calculation (747908 / 10^6 = 0.747908 USDC)
        let received = parse_amount_to_float(&response.estimated_received, 6).unwrap();
        assert_eq!(received, 0.747908);
    }

    #[tokio::test]
    async fn test_hop_quote_response_parsing_with_nulls() {
        // Test parsing response with null values (real API scenario)
        let response_json = r#"{
            "amountIn": "100000000",
            "slippage": 0.5,
            "amountOutMin": "99490050",
            "destinationAmountOutMin": null,
            "bonderFee": "10000",
            "estimatedRecieved": "99990000",
            "deadline": 1760176068,
            "destinationDeadline": null
        }"#;

        let response: HopQuoteResponse = serde_json::from_str(response_json).unwrap();

        // Test all fields are parsed correctly
        assert_eq!(response.amount_in, "100000000");
        assert_eq!(response.slippage, 0.5);
        assert_eq!(response.amount_out_min, "99490050");
        assert_eq!(response.destination_amount_out_min, None);
        assert_eq!(response.bonder_fee, "10000");
        assert_eq!(response.estimated_received, "99990000");
        assert_eq!(response.deadline, 1760176068);
        assert_eq!(response.destination_deadline, None);

        // Test fee calculation (10000 / 10^8 = 0.0001 for 8 decimal token)
        let fee = parse_amount_to_float(&response.bonder_fee, 8).unwrap();
        assert_eq!(fee, 0.0001);

        // Test estimated received calculation
        let received = parse_amount_to_float(&response.estimated_received, 8).unwrap();
        assert_eq!(received, 0.9999);
    }

    #[test]
    fn test_hop_network_serialization() {
        let network = HopNetwork {
            name: "goerli".to_string(),
            is_testnet: true,
        };

        let json = serde_json::to_string(&network).unwrap();
        let deserialized: HopNetwork = serde_json::from_str(&json).unwrap();

        assert_eq!(network.name, deserialized.name);
        assert_eq!(network.is_testnet, deserialized.is_testnet);
    }
}
