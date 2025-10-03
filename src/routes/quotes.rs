use axum::{
    Router,
    extract::{ConnectInfo, Query, Request, State},
    http::StatusCode,
    response::Json,
    routing::get,
};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::app_state::AppState;
use crate::db::SecurityRepository;
use crate::models::bridge::{BridgeClientConfig, BridgeQuote, BridgeQuoteRequest};
use crate::models::quote::{AggregatedQuotesResponse, ErrorResponse, QuoteParams, QuoteResponse};
use crate::services::bridge_client::hop::{HopConfig, HopNetwork};
use crate::services::{calculate_score, get_all_bridge_quotes};
use crate::utils::errors::AppResult;

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
fn extract_client_ip(request: &Request, connect_info: &std::net::SocketAddr) -> String {
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

    // Fall back to connection info
    connect_info.ip().to_string()
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

/// GET /quotes endpoint - Quote Aggregation
pub async fn get_quotes(
    Query(params): Query<QuoteParams>,
    State(app_state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    request: Request,
) -> Result<Json<AggregatedQuotesResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Quote aggregation request: {} {} from {} to {}",
        params.amount, params.token, params.from_chain, params.to_chain
    );

    // Rate limiting using Redis increment
    let client_ip = extract_client_ip(&request, &addr);
    let rate_limit_key = format!("rate_limit:quotes:{}", client_ip);

    info!(
        "Client IP detected: {} (from headers/connection)",
        client_ip
    );

    let request_count = app_state
        .cache()
        .increment(&rate_limit_key, 1)
        .await
        .unwrap_or(0);

    // Allow 100 requests per minute per IP
    if request_count > 100 {
        warn!(
            "Rate limit exceeded for IP {}: {} requests",
            client_ip, request_count
        );
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse {
                error: "Rate limit exceeded. Maximum 100 requests per minute.".to_string(),
            }),
        ));
    }

    // Set expiration for rate limit counter (only on first request)
    if request_count == 1 {
        let _ = app_state.cache().expire(&rate_limit_key, 60).await; // 1 minute
    }

    // Validate input parameters
    if params.from_chain.is_empty() {
        warn!("Missing from_chain parameter");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "from_chain parameter is required".to_string(),
            }),
        ));
    }

    if params.to_chain.is_empty() {
        warn!("Missing to_chain parameter");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "to_chain parameter is required".to_string(),
            }),
        ));
    }

    if params.token.is_empty() {
        warn!("Missing token parameter");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "token parameter is required".to_string(),
            }),
        ));
    }

    if params.amount <= 0.0 {
        warn!("Invalid amount: {}", params.amount);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "amount must be greater than 0".to_string(),
            }),
        ));
    }

    if params.from_chain.eq_ignore_ascii_case(&params.to_chain) {
        warn!("Same source and destination chain: {}", params.from_chain);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Source and destination chains must be different".to_string(),
            }),
        ));
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
        .with_timeout(std::time::Duration::from_secs(5)) // 5s timeout as per requirements
        .with_retries(1) // Single retry for fast response
        .with_hop_config(hop_config);

    // Get quotes from all bridges concurrently
    info!("Fetching quotes from all bridges with 5s timeout");
    let bridge_quotes = get_all_bridge_quotes(&request, &config).await;

    // Check if no quotes were returned
    if bridge_quotes.is_empty() {
        error!("No quotes available from any bridge");
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: "No quotes available".to_string(),
            }),
        ));
    }

    // Get bridge names for security metadata lookup
    let bridge_names: Vec<String> = bridge_quotes.iter().map(|q| q.bridge.clone()).collect();

    // Fetch security metadata for all bridges in batch
    info!(
        "Fetching security metadata for {} bridges",
        bridge_names.len()
    );
    let security_metadata = match SecurityRepository::get_batch_security_metadata(
        app_state.db(),
        &bridge_names,
    )
    .await
    {
        Ok(metadata) => metadata,
        Err(e) => {
            error!("Failed to fetch security metadata: {}", e);
            // Continue with empty metadata rather than failing the request
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
    let routes: Vec<QuoteResponse> = bridge_quotes
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
        .collect();

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

    Ok(Json(AggregatedQuotesResponse { routes }))
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
                liquidity: "1,000,000 USDC".to_string(),
                score: 0.85,
            },
            QuoteResponse {
                bridge: "Hop".to_string(),
                cost: 0.0015,
                est_time: 180,
                liquidity: "500,000 USDC".to_string(),
                score: 0.87,
            },
        ];

        let response = AggregatedQuotesResponse { routes };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("routes"));
        assert!(json.contains("Connext"));
        assert!(json.contains("Hop"));
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
}
