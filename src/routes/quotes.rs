use axum::{
    Router,
    extract::{Query, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Json, Response},
    routing::get,
};
use std::sync::Arc;
use tokio::time::{Duration, timeout};
use tracing::{error, info, warn};

use crate::app_state::AppState;
use crate::db::SecurityRepository;
use crate::models::bridge::{BridgeClientConfig, BridgeQuote, BridgeQuoteRequest};
use crate::models::quote::{AggregatedQuotesResponse, ErrorResponse, QuoteParams, QuoteResponse};
use crate::services::bridge_client::hop::{HopConfig, HopNetwork};
use crate::services::{calculate_score, get_all_bridge_quotes};
use crate::utils::errors::AppResult;

/// Cache TTL for fresh quotes in seconds (15 seconds as per requirements)
const QUOTE_CACHE_TTL_SECONDS: u64 = 15;

/// Maximum age for stale cache in seconds (5 minutes)
const MAX_STALE_CACHE_AGE_SECONDS: u64 = 300;

/// Generate cache key for quote requests
fn generate_cache_key(params: &QuoteParams) -> String {
    format!(
        "quotes:{}:{}:{}:{}",
        params.from_chain.to_lowercase(),
        params.to_chain.to_lowercase(),
        params.token.to_uppercase(),
        params.amount
    )
}

/// Convert internal BridgeQuote to API QuoteResponse format with score
fn convert_bridge_quote_to_response_with_score(quote: &BridgeQuote, score: f64) -> QuoteResponse {
    QuoteResponse {
        bridge: quote.bridge.clone(),
        cost: quote.fee,
        est_time: quote.est_time,
        liquidity: quote.liquidity.clone(),
        score,
    }
}

/// Extract client IP from request headers and connection info
/// Handles common proxy headers like X-Forwarded-For, X-Real-IP
/// Falls back to localhost for local testing when ConnectInfo is not available
fn extract_client_ip(request: &Request, connect_info: Option<&std::net::SocketAddr>) -> String {
    // Try to get IP from common proxy headers first
    if let Some(forwarded_for) = request.headers().get("x-forwarded-for")
        && let Ok(forwarded_str) = forwarded_for.to_str()
    {
        // X-Forwarded-For can contain multiple IPs, take the first one
        if let Some(first_ip) = forwarded_str.split(',').next() {
            let ip = first_ip.trim();
            if !ip.is_empty() && ip != "unknown" {
                return ip.to_string();
            }
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = request.headers().get("x-real-ip")
        && let Ok(real_ip_str) = real_ip.to_str()
        && !real_ip_str.is_empty()
        && real_ip_str != "unknown"
    {
        return real_ip_str.to_string();
    }

    // Try CF-Connecting-IP (Cloudflare)
    if let Some(cf_ip) = request.headers().get("cf-connecting-ip")
        && let Ok(cf_ip_str) = cf_ip.to_str()
        && !cf_ip_str.is_empty()
        && cf_ip_str != "unknown"
    {
        return cf_ip_str.to_string();
    }

    // Fall back to connection info if available, otherwise use localhost for testing
    match connect_info {
        Some(addr) => addr.ip().to_string(),
        None => "127.0.0.1".to_string(), // Fallback for local testing
    }
}

/// Convert amount float to smallest unit string based on token
fn amount_to_smallest_unit(amount: f64, token: &str) -> String {
    match token.to_uppercase().as_str() {
        "USDC" | "USDT" => {
            // 6 decimals
            let smallest_unit = (amount * 1_000_000.0) as u64;
            smallest_unit.to_string()
        }
        "ETH" | "WETH" | "DAI" => {
            // 18 decimals
            let smallest_unit = (amount * 1_000_000_000_000_000_000.0) as u64;
            smallest_unit.to_string()
        }
        _ => {
            // Default to 18 decimals for unknown tokens
            let smallest_unit = (amount * 1_000_000_000_000_000_000.0) as u64;
            smallest_unit.to_string()
        }
    }
}

/// Process quotes with security metadata and scoring
async fn process_quotes_with_security(
    bridge_quotes: &[BridgeQuote],
    app_state: &Arc<AppState>,
) -> Vec<QuoteResponse> {
    // Get bridge names for security metadata lookup
    let bridge_names: Vec<String> = bridge_quotes.iter().map(|q| q.bridge.clone()).collect();

    // Fetch security metadata for all bridges in batch, with timeout
    info!(
        "Fetching security metadata for {} bridges",
        bridge_names.len()
    );
    let security_metadata = match timeout(
        Duration::from_secs(3),
        SecurityRepository::get_batch_security_metadata(app_state.db(), &bridge_names),
    )
    .await
    {
        Ok(Ok(metadata)) => metadata,
        Ok(Err(e)) => {
            error!("Failed to fetch security metadata: {}", e);
            vec![]
        }
        Err(_) => {
            error!("Timeout fetching security metadata");
            vec![]
        }
    };

    // Create a lookup map for security metadata
    let security_map: std::collections::HashMap<String, &crate::services::SecurityMetadata> =
        security_metadata
            .iter()
            .map(|m| (m.bridge.clone(), m))
            .collect();

    // Calculate scores and convert to API response format
    bridge_quotes
        .iter()
        .map(|quote| {
            let security = security_map.get(&quote.bridge);
            let has_audit = security.map(|s| s.has_audit).unwrap_or(false);
            let has_exploit = security.map(|s| s.has_exploit).unwrap_or(false);

            let score = calculate_score(quote.fee, quote.est_time, has_audit, has_exploit);

            info!(
                "Bridge {}: fee={:.6}, time={}s, audit={}, exploit={}, score={:.3}",
                quote.bridge, quote.fee, quote.est_time, has_audit, has_exploit, score
            );

            convert_bridge_quote_to_response_with_score(quote, score)
        })
        .collect()
}

/// GET /quotes endpoint - Quote Aggregation
pub async fn get_quotes(
    Query(params): Query<QuoteParams>,
    State(app_state): State<Arc<AppState>>,
    request: Request,
) -> Response {
    info!(
        "Quote aggregation request: {} {} from {} to {}",
        params.amount, params.token, params.from_chain, params.to_chain
    );

    // Rate limiting using Redis increment
    // Try to get ConnectInfo from request extensions, fallback to None for local testing
    let connect_info = request.extensions().get::<std::net::SocketAddr>();
    let client_ip = extract_client_ip(&request, connect_info);
    let rate_limit_key = format!("rate_limit:quotes:{}", client_ip);

    info!(
        "Client IP detected: {} (from headers/connection)",
        client_ip
    );

    tracing::info!(
        "Before Redis increment for rate limiting: key={}",
        rate_limit_key
    );
    let request_count = match app_state.cache().increment(&rate_limit_key, 1).await {
        Ok(count) => {
            tracing::info!(
                "After Redis increment for rate limiting: key={}, count={}",
                rate_limit_key,
                count
            );
            count
        }
        Err(e) => {
            tracing::error!(
                "Redis increment failed for rate limiting: key={}, error={}",
                rate_limit_key,
                e
            );
            0
        }
    };

    // Allow 100 requests per minute per IP
    if request_count > 100 {
        warn!(
            "Rate limit exceeded for IP {}: {} requests",
            client_ip, request_count
        );
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse {
                error: "Rate limit exceeded. Maximum 100 requests per minute.".to_string(),
            }),
        )
            .into_response();
    }

    // Set expiration for rate limit counter (only on first request)
    if request_count == 1 {
        tracing::info!(
            "Before Redis expire for rate limiting: key={}",
            rate_limit_key
        );
        let expire_result = app_state.cache().expire(&rate_limit_key, 60).await; // 1 minute
        match expire_result {
            Ok(_) => tracing::info!(
                "After Redis expire for rate limiting: key={}",
                rate_limit_key
            ),
            Err(e) => tracing::error!(
                "Redis expire failed for rate limiting: key={}, error={}",
                rate_limit_key,
                e
            ),
        }
    }

    // Validate input parameters
    if params.from_chain.is_empty() {
        warn!("Missing from_chain parameter");
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "from_chain parameter is required".to_string(),
            }),
        )
            .into_response();
    }

    if params.to_chain.is_empty() {
        warn!("Missing to_chain parameter");
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "to_chain parameter is required".to_string(),
            }),
        )
            .into_response();
    }

    if params.token.is_empty() {
        warn!("Missing token parameter");
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "token parameter is required".to_string(),
            }),
        )
            .into_response();
    }

    if params.amount <= 0.0 {
        warn!("Invalid amount: {}", params.amount);
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "amount must be greater than 0".to_string(),
            }),
        )
            .into_response();
    }

    if params.from_chain.eq_ignore_ascii_case(&params.to_chain) {
        warn!("Same source and destination chain: {}", params.from_chain);
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Source and destination chains must be different".to_string(),
            }),
        )
            .into_response();
    }

    // Generate cache key for this request
    let cache_key = generate_cache_key(&params);

    // Try to get fresh cache first (optional optimization)
    tracing::info!("Before Redis get_cache for fresh quotes: key={}", cache_key);
    match app_state
        .cache()
        .get_cache::<AggregatedQuotesResponse>(&cache_key)
        .await
    {
        Ok(Some(cached_response)) => {
            tracing::info!(
                "After Redis get_cache for fresh quotes: key={}, HIT",
                cache_key
            );
            let mut headers = HeaderMap::new();
            headers.insert(header::CACHE_CONTROL, "public, max-age=15".parse().unwrap());
            headers.insert("X-Cache", "HIT".parse().unwrap());
            return (StatusCode::OK, headers, Json(cached_response)).into_response();
        }
        Ok(None) => {
            tracing::info!(
                "After Redis get_cache for fresh quotes: key={}, MISS",
                cache_key
            );
        }
        Err(e) => {
            tracing::error!(
                "Redis get_cache failed for fresh quotes: key={}, error={}",
                cache_key,
                e
            );
        }
    }

    // Convert amount to smallest unit for bridge APIs
    let amount_smallest_unit = amount_to_smallest_unit(params.amount, &params.token);

    // Create bridge quote request
    let request = BridgeQuoteRequest {
        asset: params.token.clone(),
        from_chain: params.from_chain.clone(),
        to_chain: params.to_chain.clone(),
        amount: Some(amount_smallest_unit),
        // recipient: None,
    };

    // Create Hop configuration (mainnet by default, can be made configurable via env)
    let is_testnet = std::env::var("HOP_NETWORK")
        .unwrap_or_else(|_| "mainnet".to_string())
        .to_lowercase()
        == "goerli";

    let hop_network = HopNetwork {
        name: if is_testnet {
            "goerli".to_string()
        } else {
            "mainnet".to_string()
        },
        is_testnet,
    };

    let hop_config = HopConfig::new(hop_network);

    // Create bridge client configuration with 5s timeout as specified
    let config = BridgeClientConfig::new()
        .with_cache(app_state.cache().clone())
        .with_timeout(Duration::from_secs(5)) // 5s timeout as per requirements
        .with_retries(1) // Single retry for fast response
        .with_hop_config(hop_config);

    // Get quotes from all bridges concurrently
    info!("Fetching quotes from all bridges with 5s timeout");
    let bridge_results = get_all_bridge_quotes(&request, &config).await;

    // Separate successful quotes and errors
    let mut quotes = Vec::new();
    let mut errors = Vec::new();
    for result in &bridge_results {
        if let Some(quote) = &result.quote {
            quotes.push(quote.clone());
        }
        if let Some(err) = &result.error {
            errors.push(crate::models::quote::BridgeQuoteError {
                bridge: result.bridge.clone(),
                error: err.clone(),
            });
        }
    }

    // Check if no quotes were returned - try stale cache fallback
    if quotes.is_empty() {
        // Try to get stale cache with extended TTL check
        let stale_cache_key = format!("{}_stale", cache_key);
        tracing::info!(
            "Before Redis get_cache for stale quotes: key={}",
            stale_cache_key
        );
        match app_state
            .cache()
            .get_cache::<AggregatedQuotesResponse>(&stale_cache_key)
            .await
        {
            Ok(Some(stale_response)) => {
                tracing::info!(
                    "After Redis get_cache for stale quotes: key={}, HIT",
                    stale_cache_key
                );
                warn!("Returning stale cached quotes for key: {}", cache_key);
                let mut headers = HeaderMap::new();
                headers.insert(
                    header::CACHE_CONTROL,
                    "public, max-age=0, must-revalidate".parse().unwrap(),
                );
                headers.insert("X-Cache", "STALE".parse().unwrap());
                headers.insert(
                    header::WARNING,
                    "110 - \"Response is Stale\"".parse().unwrap(),
                );
                return (StatusCode::OK, headers, Json(stale_response)).into_response();
            }
            Ok(None) => {
                tracing::info!(
                    "After Redis get_cache for stale quotes: key={}, MISS",
                    stale_cache_key
                );
            }
            Err(e) => {
                tracing::error!(
                    "Redis get_cache failed for stale quotes: key={}, error={}",
                    stale_cache_key,
                    e
                );
            }
        }

        // No cache available, return error
        return (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: "No quotes available and no cached data found".to_string(),
            }),
        )
            .into_response();
    }

    // Process quotes with security scoring
    tracing::info!("Before DB call: process_quotes_with_security");
    let routes = process_quotes_with_security(&quotes, &app_state).await;
    tracing::info!("After DB call: process_quotes_with_security");

    info!(
        "Returning {} routes from bridges with calculated scores",
        routes.len()
    );

    // Log each route for debugging
    for route in &routes {
        info!(
            "Route: {} - cost: {}, time: {}s, liquidity: {}, score: {:.3}",
            route.bridge, route.cost, route.est_time, route.liquidity, route.score
        );
    }

    // Check routes emptiness before moving it
    let has_routes = !routes.is_empty();
    let response = AggregatedQuotesResponse {
        routes,
        // Only include errors if there are no successful routes
        errors: if !has_routes && !errors.is_empty() {
            errors
        } else {
            Vec::new()
        },
    };

    // Cache the response with 15s TTL for fresh cache
    tracing::info!("Before Redis set_cache for fresh quotes: key={}", cache_key);
    match app_state
        .cache()
        .set_cache(&cache_key, &response, QUOTE_CACHE_TTL_SECONDS)
        .await
    {
        Ok(_) => {
            info!(
                "Cached quote response with {}s TTL for key: {}",
                QUOTE_CACHE_TTL_SECONDS, cache_key
            );
            tracing::info!("After Redis set_cache for fresh quotes: key={}", cache_key);
        }
        Err(e) => {
            warn!("Failed to cache quote response: {}", e);
            tracing::error!(
                "Redis set_cache failed for fresh quotes: key={}, error={}",
                cache_key,
                e
            );
        }
    }

    // Also cache with longer TTL for stale fallback (5 minutes)
    let stale_cache_key = format!("{}_stale", cache_key);
    tracing::info!(
        "Before Redis set_cache for stale quotes: key={}",
        stale_cache_key
    );
    match app_state
        .cache()
        .set_cache(&stale_cache_key, &response, MAX_STALE_CACHE_AGE_SECONDS)
        .await
    {
        Ok(_) => {
            tracing::info!(
                "After Redis set_cache for stale quotes: key={}",
                stale_cache_key
            );
        }
        Err(e) => {
            warn!("Failed to cache stale quote response: {}", e);
            tracing::error!(
                "Redis set_cache failed for stale quotes: key={}, error={}",
                stale_cache_key,
                e
            );
        }
    }

    // Return fresh response with appropriate headers
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CACHE_CONTROL,
        format!("public, max-age={}", QUOTE_CACHE_TTL_SECONDS)
            .parse()
            .unwrap(),
    );
    headers.insert("X-Cache", "MISS".parse().unwrap());

    (StatusCode::OK, headers, Json(response)).into_response()
}

/// Health check specifically for bridge services
pub async fn bridge_health() -> AppResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "status": "ok",
        "bridges": ["Everclear", "Hop", "Axelar"],
        "available": true
    })))
}

/// Create quotes router
pub fn quotes_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/quotes", get(get_quotes))
        .route("/quotes/health", get(bridge_health))
}

#[allow(dead_code)]
/// Convert internal BridgeQuote to API QuoteResponse format (legacy - for tests)
fn convert_bridge_quote_to_response(quote: &BridgeQuote) -> QuoteResponse {
    QuoteResponse {
        bridge: quote.bridge.clone(),
        cost: quote.fee,
        est_time: quote.est_time,
        liquidity: quote.liquidity.clone(),
        score: 0.0, // Default score for legacy compatibility
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::quote::BridgeQuoteError;

    #[test]
    fn test_quote_params_validation() {
        let params = QuoteParams {
            from_chain: "ethereum".to_string(),
            to_chain: "polygon".to_string(),
            token: "USDC".to_string(),
            amount: 1.5,
        };

        assert_eq!(params.from_chain, "ethereum");
        assert_eq!(params.to_chain, "polygon");
        assert_eq!(params.token, "USDC");
        assert_eq!(params.amount, 1.5);
    }

    #[test]
    fn test_quote_response_serialization() {
        let response = QuoteResponse {
            bridge: "Connext".to_string(),
            cost: 0.002,
            est_time: 120,
            liquidity: "1,000,000 USDC".to_string(),
            score: 0.85,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Connext"));
        assert!(json.contains("0.002"));
        assert!(json.contains("120"));
    }

    #[test]
    fn test_aggregated_response_serialization() {
        let routes = vec![
            QuoteResponse {
                bridge: "Connext".to_string(),
                cost: 0.002,
                est_time: 120,
                score: 0.0,
                liquidity: "1,000,000 USDC".to_string(),
            },
            QuoteResponse {
                bridge: "Hop".to_string(),
                cost: 0.0015,
                est_time: 180,
                score: 0.0,
                liquidity: "500,000 USDC".to_string(),
            },
        ];

        let errors = vec![BridgeQuoteError {
            bridge: "Axelar".to_string(),
            error: "Timeout after 3s".to_string(),
        }];

        // Test case 1: Successful routes only
        let response = AggregatedQuotesResponse {
            routes: routes.clone(),
            errors: Vec::new(),
        };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("routes"));
        assert!(json.contains("Connext"));
        assert!(json.contains("Hop"));
        assert!(!json.contains("\"errors\"")); // Field should be omitted when empty

        // Test case 2: Mixed success and errors (struct should serialize both)
        let response = AggregatedQuotesResponse {
            routes: routes.clone(),
            errors: errors.clone(),
        };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("routes"));
        assert!(json.contains("Connext"));
        assert!(json.contains("Hop"));
        assert!(json.contains("\"errors\"")); // The struct itself will serialize non-empty errors.

        // Test case 3: Only errors (should include errors)
        let response = AggregatedQuotesResponse {
            routes: vec![],
            errors,
        };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"errors\""));
        assert!(json.contains("Axelar"));
        assert!(json.contains("Timeout after 3s"));
        assert!(json.contains("routes"));
        assert_eq!(
            serde_json::from_str::<AggregatedQuotesResponse>(&json)
                .unwrap()
                .routes
                .len(),
            0
        );
    }
    #[test]
    fn test_amount_to_smallest_unit() {
        // Test USDC (6 decimals)
        assert_eq!(amount_to_smallest_unit(1.5, "USDC"), "1500000");
        assert_eq!(amount_to_smallest_unit(0.000001, "USDC"), "1");

        // Test ETH (18 decimals)
        assert_eq!(amount_to_smallest_unit(1.0, "ETH"), "1000000000000000000");
        assert_eq!(amount_to_smallest_unit(0.001, "ETH"), "1000000000000000");

        // Test unknown token defaults to 18 decimals
        assert_eq!(
            amount_to_smallest_unit(1.0, "UNKNOWN"),
            "1000000000000000000"
        );
    }

    #[test]
    fn test_convert_bridge_quote_to_response() {
        let bridge_quote = BridgeQuote {
            bridge: "Test Bridge".to_string(),
            fee: 0.005,
            est_time: 300,
            liquidity: "2M USDC".to_string(),
            score: None,
            metadata: None,
        };

        let response = convert_bridge_quote_to_response(&bridge_quote);

        assert_eq!(response.bridge, "Test Bridge");
        assert_eq!(response.cost, 0.005);
        assert_eq!(response.est_time, 300);
        assert_eq!(response.liquidity, "2M USDC");
    }

    #[test]
    fn test_error_response_serialization() {
        let error_response = ErrorResponse {
            error: "No quotes available".to_string(),
        };

        let json = serde_json::to_string(&error_response).unwrap();
        assert!(json.contains("No quotes available"));
    }

    #[test]
    fn test_generate_cache_key() {
        let params = QuoteParams {
            from_chain: "Ethereum".to_string(),
            to_chain: "Polygon".to_string(),
            token: "usdc".to_string(),
            amount: 100.5,
        };

        let key = generate_cache_key(&params);
        assert_eq!(key, "quotes:ethereum:polygon:USDC:100.5");
    }

    #[test]
    fn test_cache_constants() {
        assert_eq!(QUOTE_CACHE_TTL_SECONDS, 15);
        assert_eq!(MAX_STALE_CACHE_AGE_SECONDS, 300);
    }
}
