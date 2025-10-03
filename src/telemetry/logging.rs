use std::time::Instant;
use tracing::{Level, info};
use tracing_subscriber::{
    EnvFilter, Layer, Registry,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};
// OpenTelemetry integration can be added later if needed

/// Initialize structured logging with JSON output format
pub fn init_logging() -> anyhow::Result<()> {
    // Set up environment filter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("bridge_router=info,tower_http=debug"));

    // Set up JSON formatter for production
    let json_layer = fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE);

    // Set up pretty formatter for development
    let pretty_layer = fmt::layer()
        .pretty()
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE);

    // Choose formatter based on environment
    let is_production = std::env::var("RUST_LOG_FORMAT")
        .unwrap_or_else(|_| "pretty".to_string())
        .to_lowercase()
        == "json";

    let fmt_layer = if is_production {
        json_layer.boxed()
    } else {
        pretty_layer.boxed()
    };

    // Initialize the subscriber
    Registry::default().with(env_filter).with(fmt_layer).init();

    info!(
        "Structured logging initialized with {} format",
        if is_production { "JSON" } else { "pretty" }
    );

    Ok(())
}

/// Request context for structured logging
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub user_agent: Option<String>,
    pub client_ip: Option<String>,
    pub start_time: Instant,
}

impl RequestContext {
    #[allow(dead_code)]
    pub fn new(method: String, path: String) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            method,
            path,
            user_agent: None,
            client_ip: None,
            start_time: Instant::now(),
        }
    }

    #[allow(dead_code)]
    pub fn with_user_agent(mut self, user_agent: Option<String>) -> Self {
        self.user_agent = user_agent;
        self
    }

    #[allow(dead_code)]
    pub fn with_client_ip(mut self, client_ip: Option<String>) -> Self {
        self.client_ip = client_ip;
        self
    }

    #[allow(dead_code)]
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}

/// Log a request start
#[allow(dead_code)]
pub fn log_request_start(ctx: &RequestContext) {
    info!(
        request_id = %ctx.request_id,
        method = %ctx.method,
        path = %ctx.path,
        user_agent = ?ctx.user_agent,
        client_ip = ?ctx.client_ip,
        "Request started"
    );
}

/// Log a request completion
#[allow(dead_code)]
pub fn log_request_complete(ctx: &RequestContext, status: u16, error: Option<&str>) {
    let latency_ms = ctx.elapsed_ms();

    let level = match status {
        200..=299 => Level::INFO,
        300..=399 => Level::INFO,
        400..=499 => Level::WARN,
        500..=599 => Level::ERROR,
        _ => Level::WARN,
    };

    match level {
        Level::INFO => {
            info!(
                request_id = %ctx.request_id,
                method = %ctx.method,
                path = %ctx.path,
                status = status,
                latency_ms = latency_ms,
                "Request completed"
            );
        }
        Level::WARN => {
            tracing::warn!(
                request_id = %ctx.request_id,
                method = %ctx.method,
                path = %ctx.path,
                status = status,
                latency_ms = latency_ms,
                error = ?error,
                "Request completed with warning"
            );
        }
        Level::ERROR => {
            tracing::error!(
                request_id = %ctx.request_id,
                method = %ctx.method,
                path = %ctx.path,
                status = status,
                latency_ms = latency_ms,
                error = ?error,
                "Request completed with error"
            );
        }
        _ => {}
    }
}

/// Log application startup
#[allow(dead_code)]
pub fn log_startup(host: &str, port: u16) {
    info!(
        host = %host,
        port = port,
        version = env!("CARGO_PKG_VERSION"),
        "Bridge Router server starting"
    );
}

/// Log application shutdown
#[allow(dead_code)]
pub fn log_shutdown() {
    info!("Bridge Router server shutting down");
}

/// Log database connection status
#[allow(dead_code)]
pub fn log_database_status(connected: bool) {
    if connected {
        info!("Database connection established");
    } else {
        tracing::error!("Database connection failed");
    }
}

/// Log Redis connection status
#[allow(dead_code)]
pub fn log_redis_status(connected: bool) {
    if connected {
        info!("Redis connection established");
    } else {
        tracing::error!("Redis connection failed");
    }
}

/// Log bridge service status
#[allow(dead_code)]
pub fn log_bridge_service_status(bridge: &str, available: bool, latency_ms: Option<u64>) {
    if available {
        info!(
            bridge = %bridge,
            latency_ms = ?latency_ms,
            "Bridge service available"
        );
    } else {
        tracing::warn!(
            bridge = %bridge,
            "Bridge service unavailable"
        );
    }
}

/// Log security events
#[allow(dead_code)]
pub fn log_security_event(event: &str, details: &str, severity: SecuritySeverity) {
    match severity {
        SecuritySeverity::Info => {
            info!(
                event = %event,
                details = %details,
                "Security event"
            );
        }
        SecuritySeverity::Warning => {
            tracing::warn!(
                event = %event,
                details = %details,
                "Security warning"
            );
        }
        SecuritySeverity::Error => {
            tracing::error!(
                event = %event,
                details = %details,
                "Security error"
            );
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum SecuritySeverity {
    Info,
    Warning,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_context_creation() {
        let ctx = RequestContext::new("GET".to_string(), "/health".to_string());
        assert_eq!(ctx.method, "GET");
        assert_eq!(ctx.path, "/health");
        assert!(!ctx.request_id.is_empty());
    }

    #[test]
    fn test_request_context_with_metadata() {
        let ctx = RequestContext::new("POST".to_string(), "/quotes".to_string())
            .with_user_agent(Some("test-agent".to_string()))
            .with_client_ip(Some("127.0.0.1".to_string()));

        assert_eq!(ctx.user_agent, Some("test-agent".to_string()));
        assert_eq!(ctx.client_ip, Some("127.0.0.1".to_string()));
    }

    #[test]
    fn test_elapsed_time() {
        let ctx = RequestContext::new("GET".to_string(), "/test".to_string());
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(ctx.elapsed_ms() >= 10);
    }
}
