use tracing::{error, info, warn};

pub mod axelar;
pub mod everclear;
pub mod hop;

use crate::cache::CacheClient;
use crate::models::bridge::{BridgeClientConfig, BridgeError, BridgeQuote, BridgeQuoteRequest};
use std::time::Duration;

/// Get quotes from all available bridges in parallel
use tokio::time::timeout;
pub struct BridgeQuoteWithError {
    pub bridge: String,
    pub quote: Option<BridgeQuote>,
    pub error: Option<String>,
}

pub async fn get_all_bridge_quotes(
    request: &BridgeQuoteRequest,
    config: &BridgeClientConfig,
) -> Vec<BridgeQuoteWithError> {
    info!(
        "Fetching bridge quotes for {} from {} to {}",
        request.asset, request.from_chain, request.to_chain
    );

    // Per-bridge timeouts (in seconds)
    let bridge_timeout = Duration::from_secs(3);

    let everclear_fut = timeout(bridge_timeout, everclear::get_quote(request, config));
    let hop_fut = timeout(bridge_timeout, hop::get_quote(request, config));
    let axelar_fut = timeout(bridge_timeout, axelar::get_quote(request, config));

    let (everclear_result, hop_result, axelar_result) =
        tokio::join!(everclear_fut, hop_fut, axelar_fut);

    let mut results = Vec::new();

    // Everclear
    match everclear_result {
        Ok(Ok(quote)) => {
            info!(
                "Successfully got Everclear quote: fee={}, time={}s",
                quote.fee, quote.est_time
            );
            results.push(BridgeQuoteWithError {
                bridge: "Everclear".to_string(),
                quote: Some(quote),
                error: None,
            });
        }
        Ok(Err(e)) => {
            warn!("Everclear quote failed: {}", e);
            results.push(BridgeQuoteWithError {
                bridge: "Everclear".to_string(),
                quote: None,
                error: Some(e.to_string()),
            });
        }
        Err(_) => {
            warn!(
                "Everclear quote timed out after {}s",
                bridge_timeout.as_secs()
            );
            results.push(BridgeQuoteWithError {
                bridge: "Everclear".to_string(),
                quote: None,
                error: Some(format!("Timeout after {}s", bridge_timeout.as_secs())),
            });
        }
    }

    // Hop
    match hop_result {
        Ok(Ok(quote)) => {
            info!(
                "Successfully got Hop quote: fee={}, time={}s",
                quote.fee, quote.est_time
            );
            results.push(BridgeQuoteWithError {
                bridge: "Hop".to_string(),
                quote: Some(quote),
                error: None,
            });
        }
        Ok(Err(e)) => {
            warn!("Hop quote failed: {}", e);
            results.push(BridgeQuoteWithError {
                bridge: "Hop".to_string(),
                quote: None,
                error: Some(e.to_string()),
            });
        }
        Err(_) => {
            warn!("Hop quote timed out after {}s", bridge_timeout.as_secs());
            results.push(BridgeQuoteWithError {
                bridge: "Hop".to_string(),
                quote: None,
                error: Some(format!("Timeout after {}s", bridge_timeout.as_secs())),
            });
        }
    }

    // Axelar
    match axelar_result {
        Ok(Ok(quote)) => {
            info!(
                "Successfully got Axelar quote: fee={}, time={}s",
                quote.fee, quote.est_time
            );
            results.push(BridgeQuoteWithError {
                bridge: "Axelar".to_string(),
                quote: Some(quote),
                error: None,
            });
        }
        Ok(Err(e)) => {
            warn!("Axelar quote failed: {}", e);
            results.push(BridgeQuoteWithError {
                bridge: "Axelar".to_string(),
                quote: None,
                error: Some(e.to_string()),
            });
        }
        Err(_) => {
            warn!("Axelar quote timed out after {}s", bridge_timeout.as_secs());
            results.push(BridgeQuoteWithError {
                bridge: "Axelar".to_string(),
                quote: None,
                error: Some(format!("Timeout after {}s", bridge_timeout.as_secs())),
            });
        }
    }

    if results.iter().all(|r| r.quote.is_none()) {
        error!("No bridge quotes were successfully retrieved");
    } else {
        info!(
            "Successfully retrieved {} bridge quotes",
            results.iter().filter(|r| r.quote.is_some()).count()
        );
    }

    results
}
async fn get_cached_quote<F, Fut>(
    cache_key: &str,
    cache: &Option<CacheClient>,
    fetcher: F,
) -> Result<BridgeQuote, BridgeError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<BridgeQuote, BridgeError>>,
{
    // Try cache first if available
    if let Some(cache_client) = cache {
        if let Ok(Some(cached_quote)) = cache_client.get_cache::<BridgeQuote>(cache_key).await {
            info!("Cache hit for bridge quote: {}", cache_key);
            return Ok(cached_quote);
        }
        info!("Cache MISS for bridge quote: {}", cache_key);
    }

    // Fetch fresh quote
    let quote = fetcher().await?;

    // Cache the result with dynamic TTL based on bridge response time
    if let Some(cache_client) = cache {
        let ttl = if quote.est_time < 60 {
            600 // 10 minutes for fast bridges
        } else if quote.est_time < 300 {
            300 // 5 minutes for medium bridges
        } else {
            120 // 2 minutes for slow bridges
        };

        if let Err(e) = cache_client.set_cache(cache_key, &quote, ttl).await {
            warn!("Failed to cache bridge quote: {}", e);
        } else {
            info!("Cached bridge quote with TTL {}s: {}", ttl, cache_key);
        }
    }

    Ok(quote)
}

/// Retry wrapper for bridge API calls
async fn retry_request<F, Fut, T>(
    operation: F,
    retries: u32,
    operation_name: &str,
) -> Result<T, BridgeError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, BridgeError>>,
{
    let mut last_error = None;

    for attempt in 0..=retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(err) => {
                if attempt < retries {
                    // Don't retry on certain errors
                    match &err {
                        BridgeError::UnsupportedAsset { .. }
                        | BridgeError::UnsupportedRoute { .. }
                        | BridgeError::JsonParsing { .. } => {
                            return Err(err);
                        }
                        _ => {
                            let delay = Duration::from_millis(100 * (2_u64.pow(attempt)));
                            warn!(
                                "{} attempt {} failed: {}. Retrying in {:?}",
                                operation_name,
                                attempt + 1,
                                err,
                                delay
                            );
                            tokio::time::sleep(delay).await;
                        }
                    }
                }
                last_error = Some(err);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| BridgeError::Internal {
        message: "Retry loop completed without error".to_string(),
    }))
}

/// Helper function to format large numbers with commas
pub fn format_liquidity(amount: f64, symbol: &str) -> String {
    if amount >= 1_000_000.0 {
        format!("{:.1}M {}", amount / 1_000_000.0, symbol)
    } else if amount >= 1_000.0 {
        format!("{:.0}K {}", amount / 1_000.0, symbol)
    } else {
        format!("{:.0} {}", amount, symbol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_liquidity() {
        assert_eq!(format_liquidity(1_500_000.0, "USDC"), "1.5M USDC");
        assert_eq!(format_liquidity(750_000.0, "USDT"), "750K USDT");
        assert_eq!(format_liquidity(500.0, "ETH"), "500 ETH");
    }

    #[test]
    fn test_bridge_quote_serialization() {
        let quote = BridgeQuote {
            bridge: "Test".to_string(),
            fee: 0.002,
            est_time: 120,
            liquidity: "1M USDC".to_string(),
            score: None,
            metadata: None,
        };

        let json = serde_json::to_string(&quote).unwrap();
        let deserialized: BridgeQuote = serde_json::from_str(&json).unwrap();

        assert_eq!(quote.bridge, deserialized.bridge);
        assert_eq!(quote.fee, deserialized.fee);
    }
}
