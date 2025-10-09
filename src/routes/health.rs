use axum::{Router, extract::State, response::Html, response::Json, routing::get};
use serde::Serialize;
use tracing::info;

use crate::app_state::AppState;
use crate::db::pool::check_connection;
use crate::telemetry::{get_metrics, record_redis_metrics};

#[derive(Serialize, Debug, PartialEq)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct StatusResponse {
    pub status: String,
    pub db: String,
    pub cache: String,
    pub uptime_seconds: u64,
    pub bridges_available: u8,
}

pub async fn health_check(_state: State<std::sync::Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

pub async fn status_check(
    State(app_state): State<std::sync::Arc<AppState>>,
) -> Json<StatusResponse> {
    let uptime = app_state.uptime_seconds();
    let db_status = if check_connection(app_state.db()).await {
        "connected"
    } else {
        "disconnected"
    };

    let cache_status = if app_state.cache().health_check().await {
        "connected"
    } else {
        "disconnected"
    };

    // Collect and record Redis metrics for observability
    if cache_status == "connected"
        && let Ok(stats) = app_state.cache().get_stats().await
    {
        record_redis_metrics(&stats);
        info!(
            "Redis metrics recorded: {} keys, {} bytes memory",
            stats.total_keys, stats.used_memory_bytes
        );
    }

    Json(StatusResponse {
        status: "ok".to_string(),
        db: db_status.to_string(),
        cache: cache_status.to_string(),
        uptime_seconds: uptime,
        bridges_available: 9, // Everclear, Hop, Axelar, Across, Stargate, Wormhole, LayerZero, Orbiter, cBridge, Synapse (removed: Multichain - defunct)
    })
}

/// Prometheus metrics endpoint
pub async fn metrics_endpoint() -> Html<String> {
    let metrics = get_metrics();
    Html(metrics)
}

/// Create health and status routes
pub fn health_routes() -> Router<std::sync::Arc<AppState>> {
    Router::new()
        .route("/health", get(health_check))
        .route("/status", get(status_check))
        .route("/metrics", get(metrics_endpoint))
}
