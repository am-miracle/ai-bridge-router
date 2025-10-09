use tracing::{error, info, warn};

pub mod across;
pub mod axelar;
pub mod cbridge;
pub mod everclear;
pub mod hop;
pub mod layerzero;
pub mod orbiter;
pub mod stargate;
pub mod synapse;
pub mod wormhole;

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

    // Per-bridge timeouts from config
    let bridge_timeout = config.timeout;

    let everclear_fut = timeout(bridge_timeout, everclear::get_quote(request, config));
    let hop_fut = timeout(bridge_timeout, hop::get_quote(request, config));
    let axelar_fut = timeout(bridge_timeout, axelar::get_quote(request, config));
    let across_fut = timeout(bridge_timeout, across::get_quote(request, config));
    let stargate_fut = timeout(bridge_timeout, stargate::get_quote(request, config));
    let wormhole_fut = timeout(bridge_timeout, wormhole::get_quote(request, config));
    let layerzero_fut = timeout(bridge_timeout, layerzero::get_quote(request, config));
    let orbiter_fut = timeout(bridge_timeout, orbiter::get_quote(request, config));
    let cbridge_fut = timeout(bridge_timeout, cbridge::get_quote(request, config));
    let synapse_fut = timeout(bridge_timeout, synapse::get_quote(request, config));

    let (
        everclear_result,
        hop_result,
        axelar_result,
        across_result,
        stargate_result,
        wormhole_result,
        layerzero_result,
        orbiter_result,
        cbridge_result,
        synapse_result,
    ) = tokio::join!(
        everclear_fut,
        hop_fut,
        axelar_fut,
        across_fut,
        stargate_fut,
        wormhole_fut,
        layerzero_fut,
        orbiter_fut,
        cbridge_fut,
        synapse_fut
    );

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

    // Across
    match across_result {
        Ok(Ok(quote)) => {
            info!(
                "Successfully got Across quote: fee={}, time={}s",
                quote.fee, quote.est_time
            );
            results.push(BridgeQuoteWithError {
                bridge: "Across".to_string(),
                quote: Some(quote),
                error: None,
            });
        }
        Ok(Err(e)) => {
            warn!("Across quote failed: {}", e);
            results.push(BridgeQuoteWithError {
                bridge: "Across".to_string(),
                quote: None,
                error: Some(e.to_string()),
            });
        }
        Err(_) => {
            warn!("Across quote timed out after {}s", bridge_timeout.as_secs());
            results.push(BridgeQuoteWithError {
                bridge: "Across".to_string(),
                quote: None,
                error: Some(format!("Timeout after {}s", bridge_timeout.as_secs())),
            });
        }
    }

    // Stargate
    match stargate_result {
        Ok(Ok(quote)) => {
            info!(
                "Successfully got Stargate quote: fee={}, time={}s",
                quote.fee, quote.est_time
            );
            results.push(BridgeQuoteWithError {
                bridge: "Stargate".to_string(),
                quote: Some(quote),
                error: None,
            });
        }
        Ok(Err(e)) => {
            warn!("Stargate quote failed: {}", e);
            results.push(BridgeQuoteWithError {
                bridge: "Stargate".to_string(),
                quote: None,
                error: Some(e.to_string()),
            });
        }
        Err(_) => {
            warn!(
                "Stargate quote timed out after {}s",
                bridge_timeout.as_secs()
            );
            results.push(BridgeQuoteWithError {
                bridge: "Stargate".to_string(),
                quote: None,
                error: Some(format!("Timeout after {}s", bridge_timeout.as_secs())),
            });
        }
    }

    // Wormhole
    match wormhole_result {
        Ok(Ok(quote)) => {
            info!(
                "Successfully got Wormhole quote: fee={}, time={}s",
                quote.fee, quote.est_time
            );
            results.push(BridgeQuoteWithError {
                bridge: "Wormhole".to_string(),
                quote: Some(quote),
                error: None,
            });
        }
        Ok(Err(e)) => {
            warn!("Wormhole quote failed: {}", e);
            results.push(BridgeQuoteWithError {
                bridge: "Wormhole".to_string(),
                quote: None,
                error: Some(e.to_string()),
            });
        }
        Err(_) => {
            warn!(
                "Wormhole quote timed out after {}s",
                bridge_timeout.as_secs()
            );
            results.push(BridgeQuoteWithError {
                bridge: "Wormhole".to_string(),
                quote: None,
                error: Some(format!("Timeout after {}s", bridge_timeout.as_secs())),
            });
        }
    }

    // LayerZero
    match layerzero_result {
        Ok(Ok(quote)) => {
            info!(
                "Successfully got LayerZero quote: fee={}, time={}s",
                quote.fee, quote.est_time
            );
            results.push(BridgeQuoteWithError {
                bridge: "LayerZero".to_string(),
                quote: Some(quote),
                error: None,
            });
        }
        Ok(Err(e)) => {
            warn!("LayerZero quote failed: {}", e);
            results.push(BridgeQuoteWithError {
                bridge: "LayerZero".to_string(),
                quote: None,
                error: Some(e.to_string()),
            });
        }
        Err(_) => {
            warn!(
                "LayerZero quote timed out after {}s",
                bridge_timeout.as_secs()
            );
            results.push(BridgeQuoteWithError {
                bridge: "LayerZero".to_string(),
                quote: None,
                error: Some(format!("Timeout after {}s", bridge_timeout.as_secs())),
            });
        }
    }

    // Orbiter
    match orbiter_result {
        Ok(Ok(quote)) => {
            info!(
                "Successfully got Orbiter quote: fee={}, time={}s",
                quote.fee, quote.est_time
            );
            results.push(BridgeQuoteWithError {
                bridge: "Orbiter".to_string(),
                quote: Some(quote),
                error: None,
            });
        }
        Ok(Err(e)) => {
            warn!("Orbiter quote failed: {}", e);
            results.push(BridgeQuoteWithError {
                bridge: "Orbiter".to_string(),
                quote: None,
                error: Some(e.to_string()),
            });
        }
        Err(_) => {
            warn!(
                "Orbiter quote timed out after {}s",
                bridge_timeout.as_secs()
            );
            results.push(BridgeQuoteWithError {
                bridge: "Orbiter".to_string(),
                quote: None,
                error: Some(format!("Timeout after {}s", bridge_timeout.as_secs())),
            });
        }
    }

    // cBridge
    match cbridge_result {
        Ok(Ok(quote)) => {
            info!(
                "Successfully got cBridge quote: fee={}, time={}s",
                quote.fee, quote.est_time
            );
            results.push(BridgeQuoteWithError {
                bridge: "cBridge".to_string(),
                quote: Some(quote),
                error: None,
            });
        }
        Ok(Err(e)) => {
            warn!("cBridge quote failed: {}", e);
            results.push(BridgeQuoteWithError {
                bridge: "cBridge".to_string(),
                quote: None,
                error: Some(e.to_string()),
            });
        }
        Err(_) => {
            warn!(
                "cBridge quote timed out after {}s",
                bridge_timeout.as_secs()
            );
            results.push(BridgeQuoteWithError {
                bridge: "cBridge".to_string(),
                quote: None,
                error: Some(format!("Timeout after {}s", bridge_timeout.as_secs())),
            });
        }
    }

    // Synapse
    match synapse_result {
        Ok(Ok(quote)) => {
            info!(
                "Successfully got Synapse quote: fee={}, time={}s",
                quote.fee, quote.est_time
            );
            results.push(BridgeQuoteWithError {
                bridge: "Synapse".to_string(),
                quote: Some(quote),
                error: None,
            });
        }
        Ok(Err(e)) => {
            warn!("Synapse quote failed: {}", e);
            results.push(BridgeQuoteWithError {
                bridge: "Synapse".to_string(),
                quote: None,
                error: Some(e.to_string()),
            });
        }
        Err(_) => {
            warn!(
                "Synapse quote timed out after {}s",
                bridge_timeout.as_secs()
            );
            results.push(BridgeQuoteWithError {
                bridge: "Synapse".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_quote_serialization() {
        let quote = BridgeQuote {
            bridge: "Test".to_string(),
            fee: 0.002,
            est_time: 120,
            metadata: None,
        };

        let json = serde_json::to_string(&quote).unwrap();
        let deserialized: BridgeQuote = serde_json::from_str(&json).unwrap();

        assert_eq!(quote.bridge, deserialized.bridge);
        assert_eq!(quote.fee, deserialized.fee);
    }
}
