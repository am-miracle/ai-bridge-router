use axum::{
    Router,
    extract::{Query, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Json, Response},
    routing::get,
};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::time::{Duration, timeout};
use tracing::{error, info, warn};

use crate::app_state::AppState;
use crate::db::SecurityRepository;
use crate::models::bridge::{BridgeClientConfig, BridgeQuote, BridgeQuoteRequest};
use crate::models::quote::{
    AggregatedQuotesResponse, BridgeQuoteError, CostBreakdown, CostDetails, ErrorResponse,
    OutputDetails, QuoteParams, QuoteResponse, RequestMetadata, ResponseMetadata, SecurityDetails,
    TimingDetails, categorize_security, categorize_timing, format_timing,
};
use crate::services::{SecurityMetadata, calculate_score, get_all_bridge_quotes};
use crate::utils::errors::AppResult;

/// Cache TTL for fresh quotes in seconds (15 seconds)
const QUOTE_CACHE_TTL_SECONDS: u64 = 15;

/// Maximum age for stale cache in seconds (5 minutes)
const MAX_STALE_CACHE_AGE_SECONDS: u64 = 300;

/// Rate limit: maximum requests per minute per IP
const RATE_LIMIT_PER_MINUTE: i64 = 100;

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

/// Extract client IP from request headers and connection info
fn extract_client_ip(request: &Request, connect_info: Option<&SocketAddr>) -> String {
    // Try X-Forwarded-For header (proxy/load balancer)
    if let Some(forwarded_for) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded_for.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                let ip = first_ip.trim();
                if !ip.is_empty() && ip != "unknown" {
                    return ip.to_string();
                }
            }
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(real_ip_str) = real_ip.to_str() {
            if !real_ip_str.is_empty() && real_ip_str != "unknown" {
                return real_ip_str.to_string();
            }
        }
    }

    // Try CF-Connecting-IP (Cloudflare)
    if let Some(cf_ip) = request.headers().get("cf-connecting-ip") {
        if let Ok(cf_ip_str) = cf_ip.to_str() {
            if !cf_ip_str.is_empty() && cf_ip_str != "unknown" {
                return cf_ip_str.to_string();
            }
        }
    }

    // Fallback to connection info or localhost
    match connect_info {
        Some(addr) => addr.ip().to_string(),
        None => "127.0.0.1".to_string(),
    }
}

/// Convert amount float to smallest unit string based on token
fn amount_to_smallest_unit(amount: f64, token: &str) -> String {
    match token.to_uppercase().as_str() {
        "USDC" | "USDT" => {
            let smallest_unit = (amount * 1_000_000.0) as u64;
            smallest_unit.to_string()
        }
        "ETH" | "WETH" | "DAI" => {
            let smallest_unit = (amount * 1_000_000_000_000_000_000.0) as u64;
            smallest_unit.to_string()
        }
        _ => {
            let smallest_unit = (amount * 1_000_000_000_000_000_000.0) as u64;
            smallest_unit.to_string()
        }
    }
}

/// Calculate security score (0.0 to 1.0)
fn calculate_security_score(has_audit: bool, has_exploit: bool) -> f64 {
    let mut score: f64 = 0.5; // Base score

    if has_audit {
        score += 0.3;
    }

    if has_exploit {
        score -= 0.4;
    }

    score.max(0.0).min(1.0)
}

/// Convert BridgeQuote to detailed QuoteResponse
fn convert_to_quote_response(
    quote: &BridgeQuote,
    params: &QuoteParams,
    security_metadata: Option<&SecurityMetadata>,
) -> QuoteResponse {
    let has_audit = security_metadata.map(|s| s.has_audit).unwrap_or(false);
    let has_exploit = security_metadata.map(|s| s.has_exploit).unwrap_or(false);

    // Calculate overall score (combines cost, time, security)
    let overall_score = calculate_score(quote.fee, quote.est_time, has_audit, has_exploit);

    // Calculate security score
    let security_score = calculate_security_score(has_audit, has_exploit);

    // Calculate output amounts
    let expected_output = params.amount - quote.fee;
    let minimum_output = expected_output * (1.0 - params.slippage / 100.0);

    // For MVP, we don't have real-time gas prices yet
    // So we'll just show the bridge fee
    let cost = CostDetails {
        total_fee: quote.fee,
        total_fee_usd: quote.fee, // TODO: Add token price conversion
        breakdown: CostBreakdown {
            bridge_fee: quote.fee,
            gas_estimate_usd: 0.0, // TODO: Add gas price fetching
        },
    };

    let output = OutputDetails {
        expected: expected_output,
        minimum: minimum_output,
        input: params.amount,
    };

    let timing = TimingDetails {
        seconds: quote.est_time,
        display: format_timing(quote.est_time),
        category: categorize_timing(quote.est_time),
    };

    let security = SecurityDetails {
        score: security_score,
        level: categorize_security(security_score),
        has_audit,
        has_exploit,
    };

    // Determine availability and warnings
    let mut warnings = Vec::new();
    let available = true; // All quotes that come back are available

    // Add warning if security score is low
    if security_score < 0.4 {
        warnings.push("low_security".to_string());
    }

    // Add warning if timing is slow
    if quote.est_time > 600 {
        warnings.push("slow_route".to_string());
    }

    let status = "operational".to_string();

    QuoteResponse {
        bridge: quote.bridge.clone(),
        score: overall_score,
        cost,
        output,
        timing,
        security,
        available,
        status,
        warnings,
    }
}

/// Process quotes with security metadata
async fn process_quotes_with_security(
    bridge_quotes: &[BridgeQuote],
    params: &QuoteParams,
    app_state: &Arc<AppState>,
) -> Vec<QuoteResponse> {
    // Get bridge names for security metadata lookup
    let bridge_names: Vec<String> = bridge_quotes.iter().map(|q| q.bridge.clone()).collect();

    // Fetch security metadata for all bridges in batch
    let security_metadata = match timeout(
        Duration::from_secs(10),
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

    // Create lookup map
    let security_map: HashMap<String, &SecurityMetadata> = security_metadata
        .iter()
        .map(|m| (m.bridge.clone(), m))
        .collect();

    // Convert quotes to response format
    bridge_quotes
        .iter()
        .map(|quote| {
            let security = security_map.get(&quote.bridge).copied();
            convert_to_quote_response(quote, params, security)
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
        "Quote request: {} {} from {} to {}",
        params.amount, params.token, params.from_chain, params.to_chain
    );

    // Rate limiting
    let connect_info = request.extensions().get::<SocketAddr>();
    let client_ip = extract_client_ip(&request, connect_info);
    let rate_limit_key = format!("rate_limit:quotes:{}", client_ip);

    let request_count = match app_state.cache().increment(&rate_limit_key, 1).await {
        Ok(count) => count,
        Err(e) => {
            error!("Redis increment failed: {}", e);
            0
        }
    };

    if request_count > RATE_LIMIT_PER_MINUTE {
        warn!("Rate limit exceeded for IP {}", client_ip);
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse {
                error: "Rate limit exceeded. Maximum 100 requests per minute.".to_string(),
            }),
        )
            .into_response();
    }

    // Set expiration on first request
    if request_count == 1 {
        let _ = app_state.cache().expire(&rate_limit_key, 60).await;
    }

    // Validate parameters
    if params.from_chain.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "from_chain parameter is required".to_string(),
            }),
        )
            .into_response();
    }

    if params.to_chain.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "to_chain parameter is required".to_string(),
            }),
        )
            .into_response();
    }

    if params.token.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "token parameter is required".to_string(),
            }),
        )
            .into_response();
    }

    if params.amount <= 0.0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "amount must be greater than 0".to_string(),
            }),
        )
            .into_response();
    }

    if params.from_chain.eq_ignore_ascii_case(&params.to_chain) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Source and destination chains must be different".to_string(),
            }),
        )
            .into_response();
    }

    // Check cache
    let cache_key = generate_cache_key(&params);
    match app_state
        .cache()
        .get_cache::<AggregatedQuotesResponse>(&cache_key)
        .await
    {
        Ok(Some(cached_response)) => {
            info!("Cache HIT for key: {}", cache_key);
            let mut headers = HeaderMap::new();
            headers.insert(header::CACHE_CONTROL, "public, max-age=15".parse().unwrap());
            headers.insert("X-Cache", "HIT".parse().unwrap());
            return (StatusCode::OK, headers, Json(cached_response)).into_response();
        }
        Ok(None) => {
            info!("Cache MISS for key: {}", cache_key);
        }
        Err(e) => {
            error!("Cache get failed: {}", e);
        }
    }

    // Convert amount to smallest unit
    let amount_smallest_unit = amount_to_smallest_unit(params.amount, &params.token);

    // Create bridge quote request
    let request = BridgeQuoteRequest {
        asset: params.token.clone(),
        from_chain: params.from_chain.clone(),
        to_chain: params.to_chain.clone(),
        amount: Some(amount_smallest_unit),
        slippage: params.slippage,
    };

    // Configure bridge client
    let config = BridgeClientConfig::new()
        .with_cache(app_state.cache().clone())
        .with_timeout(Duration::from_secs(10))
        .with_retries(0);

    // Fetch quotes from all bridges
    info!("Fetching quotes from all bridges");
    let bridge_results = get_all_bridge_quotes(&request, &config).await;

    // Separate successful quotes and errors
    let mut quotes = Vec::new();
    let mut errors = Vec::new();

    for result in &bridge_results {
        if let Some(quote) = &result.quote {
            quotes.push(quote.clone());
        }
        if let Some(err) = &result.error {
            errors.push(BridgeQuoteError {
                bridge: result.bridge.clone(),
                error: err.clone(),
            });
        }
    }

    // If no quotes, try stale cache
    if quotes.is_empty() {
        let stale_cache_key = format!("{}_stale", cache_key);
        match app_state
            .cache()
            .get_cache::<AggregatedQuotesResponse>(&stale_cache_key)
            .await
        {
            Ok(Some(stale_response)) => {
                warn!("Returning stale cache for key: {}", cache_key);
                let mut headers = HeaderMap::new();
                headers.insert(
                    header::CACHE_CONTROL,
                    "public, max-age=0, must-revalidate".parse().unwrap(),
                );
                headers.insert("X-Cache", "STALE".parse().unwrap());
                return (StatusCode::OK, headers, Json(stale_response)).into_response();
            }
            _ => {}
        }

        // No cache available
        return (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: "No quotes available and no cached data found".to_string(),
            }),
        )
            .into_response();
    }

    // Process quotes with security metadata
    let routes = process_quotes_with_security(&quotes, &params, &app_state).await;

    info!("Returning {} routes", routes.len());

    // Create response
    let response = AggregatedQuotesResponse {
        routes: routes.clone(),
        metadata: ResponseMetadata {
            total_routes: routes.len(),
            available_routes: routes.iter().filter(|r| r.available).count(),
            request: RequestMetadata {
                from: params.from_chain.clone(),
                to: params.to_chain.clone(),
                token: params.token.clone(),
                amount: params.amount,
            },
        },
        errors: if routes.is_empty() {
            errors
        } else {
            Vec::new()
        },
    };

    // Cache the response (fresh)
    let _ = app_state
        .cache()
        .set_cache(&cache_key, &response, QUOTE_CACHE_TTL_SECONDS)
        .await;

    // Cache for stale fallback
    let stale_cache_key = format!("{}_stale", cache_key);
    let _ = app_state
        .cache()
        .set_cache(&stale_cache_key, &response, MAX_STALE_CACHE_AGE_SECONDS)
        .await;

    // Return response
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

/// Health check for bridge services
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_to_smallest_unit() {
        assert_eq!(amount_to_smallest_unit(1.5, "USDC"), "1500000");
        assert_eq!(amount_to_smallest_unit(0.000001, "USDC"), "1");
        assert_eq!(amount_to_smallest_unit(1.0, "ETH"), "1000000000000000000");
    }

    #[test]
    fn test_generate_cache_key() {
        let params = QuoteParams {
            from_chain: "Ethereum".to_string(),
            to_chain: "Polygon".to_string(),
            token: "usdc".to_string(),
            amount: 100.5,
            slippage: 0.5,
        };

        let key = generate_cache_key(&params);
        assert_eq!(key, "quotes:ethereum:polygon:USDC:100.5");
    }

    #[test]
    fn test_calculate_security_score() {
        assert!((calculate_security_score(true, false) - 0.8).abs() < 0.001);
        assert!((calculate_security_score(false, false) - 0.5).abs() < 0.001);
        assert!((calculate_security_score(true, true) - 0.4).abs() < 0.001);
        assert!((calculate_security_score(false, true) - 0.1).abs() < 0.001);
    }
}
