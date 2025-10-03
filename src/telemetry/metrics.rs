use axum::{
    extract::Request,
    http::{Method, StatusCode},
    middleware::Next,
    response::Response,
};
use metrics::{
    Unit, counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram,
};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::time::Instant;
use tracing::info;

/// Initialize Prometheus metrics exporter
pub fn init_metrics() -> anyhow::Result<()> {
    // Build the Prometheus exporter
    let builder = PrometheusBuilder::new();

    // Install the exporter
    let _handle = builder
        .install_recorder()
        .map_err(|e| anyhow::anyhow!("Failed to install Prometheus recorder: {}", e))?;

    // Describe metrics for better documentation
    describe_metrics();

    info!("Prometheus metrics initialized");
    Ok(())
}

/// Describe all metrics for better documentation
fn describe_metrics() {
    // HTTP request metrics
    describe_counter!(
        "http_requests_total",
        Unit::Count,
        "Total number of HTTP requests"
    );

    describe_histogram!(
        "http_request_duration_seconds",
        Unit::Seconds,
        "HTTP request duration in seconds"
    );

    describe_counter!(
        "http_requests_errors_total",
        Unit::Count,
        "Total number of HTTP request errors"
    );

    // Bridge service metrics
    describe_counter!(
        "bridge_requests_total",
        Unit::Count,
        "Total number of bridge quote requests"
    );

    describe_histogram!(
        "bridge_response_time_seconds",
        Unit::Seconds,
        "Bridge service response time in seconds"
    );

    describe_counter!(
        "bridge_errors_total",
        Unit::Count,
        "Total number of bridge service errors"
    );

    // Database metrics
    describe_counter!(
        "database_queries_total",
        Unit::Count,
        "Total number of database queries"
    );

    describe_histogram!(
        "database_query_duration_seconds",
        Unit::Seconds,
        "Database query duration in seconds"
    );

    describe_gauge!(
        "database_connections_active",
        Unit::Count,
        "Number of active database connections"
    );

    // Cache metrics
    describe_counter!(
        "cache_operations_total",
        Unit::Count,
        "Total number of cache operations"
    );

    describe_counter!(
        "cache_hits_total",
        Unit::Count,
        "Total number of cache hits"
    );

    describe_counter!(
        "cache_misses_total",
        Unit::Count,
        "Total number of cache misses"
    );

    // Redis monitoring metrics
    describe_gauge!(
        "redis_memory_usage_bytes",
        Unit::Bytes,
        "Redis memory usage in bytes"
    );

    describe_gauge!(
        "redis_total_keys",
        Unit::Count,
        "Total number of keys in Redis"
    );

    describe_gauge!(
        "redis_connected_clients",
        Unit::Count,
        "Number of connected Redis clients"
    );

    describe_gauge!(
        "redis_uptime_seconds",
        Unit::Seconds,
        "Redis server uptime in seconds"
    );

    describe_counter!(
        "redis_keyspace_hits_total",
        Unit::Count,
        "Total number of Redis keyspace hits"
    );

    describe_counter!(
        "redis_keyspace_misses_total",
        Unit::Count,
        "Total number of Redis keyspace misses"
    );

    describe_counter!(
        "redis_evicted_keys_total",
        Unit::Count,
        "Total number of evicted keys"
    );

    // Security metrics
    describe_counter!(
        "security_events_total",
        Unit::Count,
        "Total number of security events"
    );

    describe_counter!(
        "security_audits_processed_total",
        Unit::Count,
        "Total number of security audits processed"
    );

    describe_counter!(
        "security_exploits_processed_total",
        Unit::Count,
        "Total number of security exploits processed"
    );
}

/// Record HTTP request metrics
#[allow(dead_code)]
pub fn record_http_request(
    method: &Method,
    path: &str,
    status: StatusCode,
    duration: std::time::Duration,
) {
    let method_str = method.as_str().to_string();
    let path_str = path.to_string();
    let status_str = status.as_str().to_string();
    let duration_secs = duration.as_secs_f64();

    // Increment request counter
    counter!(
        "http_requests_total",
        "method" => method_str.clone(),
        "path" => path_str.clone(),
        "status" => status_str.clone()
    )
    .increment(1);

    // Record request duration
    histogram!(
        "http_request_duration_seconds",
        "method" => method_str.clone(),
        "path" => path_str.clone()
    )
    .record(duration_secs);

    // Record errors
    if status.is_client_error() || status.is_server_error() {
        let error_type = if status.is_client_error() {
            "client_error"
        } else {
            "server_error"
        };
        counter!(
            "http_requests_errors_total",
            "method" => method_str,
            "path" => path_str,
            "status" => status_str,
            "error_type" => error_type
        )
        .increment(1);
    }
}

/// Record bridge service metrics
#[allow(dead_code)]
pub fn record_bridge_request(bridge: &str, success: bool, duration: std::time::Duration) {
    let bridge_str = bridge.to_string();
    let duration_secs = duration.as_secs_f64();
    let status = if success { "success" } else { "error" };

    counter!(
        "bridge_requests_total",
        "bridge" => bridge_str.clone(),
        "status" => status
    )
    .increment(1);

    histogram!(
        "bridge_response_time_seconds",
        "bridge" => bridge_str.clone()
    )
    .record(duration_secs);

    if !success {
        counter!(
            "bridge_errors_total",
            "bridge" => bridge_str
        )
        .increment(1);
    }
}

/// Record database metrics
#[allow(dead_code)]
pub fn record_database_query(operation: &str, success: bool, duration: std::time::Duration) {
    let operation_str = operation.to_string();
    let duration_secs = duration.as_secs_f64();
    let status = if success { "success" } else { "error" };

    counter!(
        "database_queries_total",
        "operation" => operation_str.clone(),
        "status" => status
    )
    .increment(1);

    histogram!(
        "database_query_duration_seconds",
        "operation" => operation_str
    )
    .record(duration_secs);
}

/// Update database connection gauge
#[allow(dead_code)]
pub fn update_database_connections(active: u32, idle: u32) {
    gauge!("database_connections_active").set(active as f64);
    gauge!("database_connections_idle").set(idle as f64);
}

/// Record cache metrics
#[allow(dead_code)]
pub fn record_cache_operation(operation: &str, hit: bool) {
    let operation_str = operation.to_string();

    counter!(
        "cache_operations_total",
        "operation" => operation_str.clone()
    )
    .increment(1);

    if hit {
        counter!(
            "cache_hits_total",
            "operation" => operation_str
        )
        .increment(1);
    } else {
        counter!(
            "cache_misses_total",
            "operation" => operation_str
        )
        .increment(1);
    }
}

/// Record security metrics
#[allow(dead_code)]
pub fn record_security_event(event_type: &str, severity: &str) {
    let event_type_str = event_type.to_string();
    let severity_str = severity.to_string();

    counter!(
        "security_events_total",
        "event_type" => event_type_str,
        "severity" => severity_str
    )
    .increment(1);
}

/// Record security audit processing
#[allow(dead_code)]
pub fn record_security_audit_processed(bridge: &str) {
    let bridge_str = bridge.to_string();

    counter!(
        "security_audits_processed_total",
        "bridge" => bridge_str
    )
    .increment(1);
}

/// Record security exploit processing
#[allow(dead_code)]
pub fn record_security_exploit_processed(bridge: &str) {
    let bridge_str = bridge.to_string();

    counter!(
        "security_exploits_processed_total",
        "bridge" => bridge_str
    )
    .increment(1);
}

/// Record Redis monitoring metrics
pub fn record_redis_metrics(stats: &crate::cache::CacheStats) {
    gauge!("redis_memory_usage_bytes").set(stats.used_memory_bytes as f64);
    gauge!("redis_total_keys").set(stats.total_keys as f64);
    gauge!("redis_connected_clients").set(stats.connected_clients as f64);
    gauge!("redis_uptime_seconds").set(stats.uptime_seconds as f64);
    counter!("redis_keyspace_hits_total").increment(stats.hits);
    counter!("redis_keyspace_misses_total").increment(stats.misses);
    counter!("redis_evicted_keys_total").increment(stats.evicted_keys);
}

/// Record Redis monitoring information
#[allow(dead_code)]
pub fn record_redis_monitoring_info(info: &crate::cache::RedisMonitoringInfo) {
    gauge!("redis_memory_usage_bytes").set(info.memory_usage_bytes as f64);
    gauge!("redis_total_keys").set(info.db_size as f64);
    gauge!("redis_connected_clients").set(info.pool_size as f64);
}

/// Axum middleware for HTTP request metrics
#[allow(dead_code)]
pub async fn metrics_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // Execute the request
    let response = next.run(request).await;
    let status = response.status();
    let duration = start.elapsed();

    // Record metrics
    record_http_request(&method, &path, status, duration);

    response
}

/// Get metrics in Prometheus format
pub fn get_metrics() -> String {
    // For now, return a simple metrics format
    // In a real implementation, you would use the Prometheus exporter
    "# HELP http_requests_total Total number of HTTP requests\n\
     # TYPE http_requests_total counter\n\
     http_requests_total{method=\"GET\",path=\"/health\",status=\"200\"} 0\n\
     # HELP bridge_requests_total Total number of bridge quote requests\n\
     # TYPE bridge_requests_total counter\n\
     bridge_requests_total{bridge=\"Connext\",status=\"success\"} 0\n"
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_initialization() {
        // This test would require a test environment setup
        // For now, we'll test the metric recording functions
        let duration = std::time::Duration::from_millis(100);

        // Test that these don't panic
        record_http_request(&Method::GET, "/health", StatusCode::OK, duration);
        record_bridge_request("Connext", true, duration);
        record_database_query("SELECT", true, duration);
        record_cache_operation("GET", true);
        record_security_event("audit", "info");
    }

    #[test]
    fn test_metrics_descriptions() {
        // Test that describe_metrics doesn't panic
        describe_metrics();
    }

    // Note: Middleware testing requires a full Axum application setup
    // For now, we test the core metrics functionality
}
