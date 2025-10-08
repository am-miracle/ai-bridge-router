use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{error, info, warn};

/// Token price information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    /// Token symbol (e.g., "USDC", "ETH")
    pub symbol: String,
    /// Token ID on CoinGecko (e.g., "usd-coin", "ethereum")
    pub coingecko_id: String,
    /// Price in USD
    pub usd_price: f64,
    /// 24h price change percentage
    pub price_change_24h: Option<f64>,
    /// Last updated timestamp
    pub last_updated: Option<u64>,
}

/// CoinGecko API response for simple price endpoint
#[derive(Debug, Deserialize)]
struct CoinGeckoSimplePriceResponse {
    #[serde(flatten)]
    prices: HashMap<String, CoinGeckoPriceData>,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoPriceData {
    usd: f64,
    #[serde(default)]
    usd_24h_change: Option<f64>,
}

/// Token price service for fetching real-time token prices
pub struct TokenPriceService {
    client: Client,
    coingecko_api_key: Option<String>,
}

impl TokenPriceService {
    /// Create a new token price service
    pub fn new(coingecko_api_key: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("bridge-router/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            coingecko_api_key,
        }
    }

    /// Get token ID mapping for common tokens
    fn get_coingecko_id(token: &str) -> Option<&'static str> {
        match token.to_uppercase().as_str() {
            "ETH" | "WETH" => Some("ethereum"),
            "USDC" => Some("usd-coin"),
            "USDT" => Some("tether"),
            "DAI" => Some("dai"),
            "WBTC" => Some("wrapped-bitcoin"),
            "MATIC" => Some("matic-network"),
            "ARB" => Some("arbitrum"),
            "OP" => Some("optimism"),
            "AVAX" => Some("avalanche-2"),
            "BNB" => Some("binancecoin"),
            _ => None,
        }
    }

    /// Fetch price for a single token
    pub async fn get_token_price(&self, token: &str) -> Result<TokenPrice, String> {
        let coingecko_id =
            Self::get_coingecko_id(token).ok_or_else(|| format!("Unsupported token: {}", token))?;

        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&include_24hr_change=true",
            coingecko_id
        );

        let mut request = self.client.get(&url);

        if let Some(api_key) = &self.coingecko_api_key {
            request = request.header("x-cg-demo-api-key", api_key);
        }

        match request.send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    return Err(format!("CoinGecko API error {}: {}", status, body));
                }

                match response.json::<CoinGeckoSimplePriceResponse>().await {
                    Ok(data) => {
                        if let Some(price_data) = data.prices.get(coingecko_id) {
                            info!(
                                "Fetched {} price: ${:.4} (24h change: {:+.2}%)",
                                token,
                                price_data.usd,
                                price_data.usd_24h_change.unwrap_or(0.0)
                            );

                            Ok(TokenPrice {
                                symbol: token.to_uppercase(),
                                coingecko_id: coingecko_id.to_string(),
                                usd_price: price_data.usd,
                                price_change_24h: price_data.usd_24h_change,
                                last_updated: Some(
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs(),
                                ),
                            })
                        } else {
                            Err(format!("No price data found for {}", token))
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse CoinGecko response: {}", e);
                        Err(format!("Failed to parse CoinGecko response: {}", e))
                    }
                }
            }
            Err(e) => {
                error!("Failed to fetch token price for {}: {}", token, e);
                Err(format!("Failed to fetch token price: {}", e))
            }
        }
    }

    /// Fetch prices for multiple tokens in a single API call
    #[allow(dead_code)]
    pub async fn get_multiple_prices(
        &self,
        tokens: &[&str],
    ) -> HashMap<String, Result<TokenPrice, String>> {
        let mut results = HashMap::new();

        // Get CoinGecko IDs for all tokens
        let mut valid_tokens = Vec::new();
        let mut coingecko_ids = Vec::new();

        for token in tokens {
            if let Some(id) = Self::get_coingecko_id(token) {
                valid_tokens.push(token.to_uppercase());
                coingecko_ids.push(id);
            } else {
                results.insert(
                    token.to_uppercase(),
                    Err(format!("Unsupported token: {}", token)),
                );
            }
        }

        if coingecko_ids.is_empty() {
            return results;
        }

        // Fetch all prices in one API call
        let ids_str = coingecko_ids.join(",");
        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&include_24hr_change=true",
            ids_str
        );

        let mut request = self.client.get(&url);

        if let Some(api_key) = &self.coingecko_api_key {
            request = request.header("x-cg-demo-api-key", api_key);
        }

        match request.send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    let status = response.status();
                    let error_msg = format!("CoinGecko API error: {}", status);
                    for token in &valid_tokens {
                        results.insert(token.clone(), Err(error_msg.clone()));
                    }
                    return results;
                }

                match response.json::<CoinGeckoSimplePriceResponse>().await {
                    Ok(data) => {
                        for (i, token) in valid_tokens.iter().enumerate() {
                            let coingecko_id = coingecko_ids[i];
                            if let Some(price_data) = data.prices.get(coingecko_id) {
                                results.insert(
                                    token.clone(),
                                    Ok(TokenPrice {
                                        symbol: token.clone(),
                                        coingecko_id: coingecko_id.to_string(),
                                        usd_price: price_data.usd,
                                        price_change_24h: price_data.usd_24h_change,
                                        last_updated: Some(
                                            std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap()
                                                .as_secs(),
                                        ),
                                    }),
                                );
                            } else {
                                results.insert(
                                    token.clone(),
                                    Err(format!("No price data for {}", token)),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to parse response: {}", e);
                        for token in &valid_tokens {
                            results.insert(token.clone(), Err(error_msg.clone()));
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to fetch prices: {}", e);
                for token in &valid_tokens {
                    results.insert(token.clone(), Err(error_msg.clone()));
                }
            }
        }

        results
    }

    /// Get fallback price for a token (used when API is unavailable)
    #[allow(dead_code)]
    pub fn get_fallback_price(&self, token: &str) -> TokenPrice {
        let (coingecko_id, usd_price) = match token.to_uppercase().as_str() {
            "ETH" | "WETH" => ("ethereum", 3000.0),
            "USDC" | "USDT" | "DAI" => ("usd-coin", 1.0),
            "WBTC" => ("wrapped-bitcoin", 60000.0),
            "MATIC" => ("matic-network", 0.8),
            "ARB" => ("arbitrum", 1.2),
            "OP" => ("optimism", 2.5),
            "AVAX" => ("avalanche-2", 35.0),
            "BNB" => ("binancecoin", 500.0),
            _ => ("unknown", 1.0),
        };

        warn!("Using fallback price for {}: ${:.2}", token, usd_price);

        TokenPrice {
            symbol: token.to_uppercase(),
            coingecko_id: coingecko_id.to_string(),
            usd_price,
            price_change_24h: None,
            last_updated: None,
        }
    }
}

/// Convert token amount to USD
pub fn convert_to_usd(token_amount: f64, token_price: &TokenPrice) -> f64 {
    token_amount * token_price.usd_price
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_coingecko_id() {
        assert_eq!(TokenPriceService::get_coingecko_id("ETH"), Some("ethereum"));
        assert_eq!(
            TokenPriceService::get_coingecko_id("USDC"),
            Some("usd-coin")
        );
        assert_eq!(
            TokenPriceService::get_coingecko_id("WETH"),
            Some("ethereum")
        );
        assert_eq!(TokenPriceService::get_coingecko_id("UNKNOWN"), None);
    }

    #[test]
    fn test_convert_to_usd() {
        let price = TokenPrice {
            symbol: "ETH".to_string(),
            coingecko_id: "ethereum".to_string(),
            usd_price: 3000.0,
            price_change_24h: None,
            last_updated: None,
        };

        assert_eq!(convert_to_usd(1.0, &price), 3000.0);
        assert_eq!(convert_to_usd(0.5, &price), 1500.0);
        assert!((convert_to_usd(0.001, &price) - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_fallback_prices() {
        let service = TokenPriceService::new(None);

        let eth_price = service.get_fallback_price("ETH");
        assert_eq!(eth_price.symbol, "ETH");
        assert!(eth_price.usd_price > 0.0);

        let usdc_price = service.get_fallback_price("USDC");
        assert_eq!(usdc_price.symbol, "USDC");
        assert_eq!(usdc_price.usd_price, 1.0);
    }
}
