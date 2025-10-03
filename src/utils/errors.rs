use axum::{
    Json,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use std::time::Instant;
use thiserror::Error;
use tracing::{error, warn};
use uuid::Uuid;

/// Application error types with proper error handling
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] deadpool_redis::redis::RedisError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("Bridge service error: {0}")]
    #[allow(dead_code)]
    BridgeService(String),

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    #[allow(dead_code)]
    Authorization(String),

    #[error("Timeout error: {0}")]
    Timeout(String),
}

/// Standardized error response format
#[derive(Serialize, Debug)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub code: u16,
    pub request_id: Option<String>,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl AppError {
    /// Create a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a validation error
    #[allow(dead_code)]
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create a not found error
    #[allow(dead_code)]
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    /// Create an internal error
    #[allow(dead_code)]
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(anyhow::anyhow!(msg.into()))
    }

    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Authentication(_) => StatusCode::UNAUTHORIZED,
            AppError::Authorization(_) => StatusCode::FORBIDDEN,
            AppError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            AppError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::Timeout(_) => StatusCode::REQUEST_TIMEOUT,
            AppError::Config(_)
            | AppError::Internal(_)
            | AppError::Database(_)
            | AppError::Redis(_)
            | AppError::Serialization(_)
            | AppError::HttpClient(_)
            | AppError::BridgeService(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get the error type for logging
    pub fn error_type(&self) -> &'static str {
        match self {
            AppError::Database(_) => "database_error",
            AppError::Redis(_) => "redis_error",
            AppError::Config(_) => "config_error",
            AppError::Validation(_) => "validation_error",
            AppError::NotFound(_) => "not_found",
            AppError::Internal(_) => "internal_error",
            AppError::Serialization(_) => "serialization_error",
            AppError::HttpClient(_) => "http_client_error",
            AppError::BridgeService(_) => "bridge_service_error",
            AppError::RateLimited => "rate_limited",
            AppError::ServiceUnavailable(_) => "service_unavailable",
            AppError::Authentication(_) => "authentication_error",
            AppError::Authorization(_) => "authorization_error",
            AppError::Timeout(_) => "timeout_error",
        }
    }

    /// Check if this error should be logged as an error (vs warning)
    pub fn should_log_as_error(&self) -> bool {
        matches!(
            self,
            AppError::Database(_)
                | AppError::Redis(_)
                | AppError::Config(_)
                | AppError::Internal(_)
                | AppError::Serialization(_)
                | AppError::HttpClient(_)
                | AppError::BridgeService(_)
                | AppError::ServiceUnavailable(_)
        )
    }

    /// Get a safe error message for clients (no sensitive details)
    pub fn safe_message(&self) -> String {
        match self {
            AppError::Database(_) => "Database operation failed".to_string(),
            AppError::Redis(_) => "Cache operation failed".to_string(),
            AppError::Config(_) => "Configuration error".to_string(),
            AppError::Validation(msg) => msg.clone(),
            AppError::NotFound(msg) => msg.clone(),
            AppError::Internal(_) => "Internal server error".to_string(),
            AppError::Serialization(_) => "Data serialization failed".to_string(),
            AppError::HttpClient(_) => "External service error".to_string(),
            AppError::BridgeService(msg) => format!("Bridge service error: {}", msg),
            AppError::RateLimited => "Rate limit exceeded".to_string(),
            AppError::ServiceUnavailable(msg) => format!("Service unavailable: {}", msg),
            AppError::Authentication(msg) => msg.clone(),
            AppError::Authorization(msg) => msg.clone(),
            AppError::Timeout(msg) => format!("Request timeout: {}", msg),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let request_id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339();

        // Log the full error details
        if self.should_log_as_error() {
            error!(
                request_id = %request_id,
                error_type = %self.error_type(),
                error = %self,
                status = %status.as_u16(),
                "Application error occurred"
            );
        } else {
            warn!(
                request_id = %request_id,
                error_type = %self.error_type(),
                error = %self,
                status = %status.as_u16(),
                "Application warning occurred"
            );
        }

        let error_response = ErrorResponse {
            error: self.error_type().to_string(),
            message: self.safe_message(),
            code: status.as_u16(),
            request_id: Some(request_id),
            timestamp,
            details: None,
        };

        (status, Json(error_response)).into_response()
    }
}

/// Request context for error handling
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RequestContext {
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub start_time: Instant,
}

impl RequestContext {
    #[allow(dead_code)]
    pub fn new(method: String, path: String) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            method,
            path,
            start_time: Instant::now(),
        }
    }

    #[allow(dead_code)]
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}

/// Global error handler middleware
#[allow(dead_code)]
pub async fn error_handler_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let ctx = RequestContext::new(method.to_string(), path);

    // Add request context to headers for downstream handlers
    let mut request = request;
    request
        .headers_mut()
        .insert("x-request-id", ctx.request_id.parse().unwrap());

    let response = next.run(request).await;
    let status = response.status();
    let latency_ms = ctx.elapsed_ms();

    // Log request completion
    if status.is_success() {
        tracing::info!(
            request_id = %ctx.request_id,
            method = %ctx.method,
            path = %ctx.path,
            status = %status.as_u16(),
            latency_ms = latency_ms,
            "Request completed successfully"
        );
    } else if status.is_client_error() {
        tracing::warn!(
            request_id = %ctx.request_id,
            method = %ctx.method,
            path = %ctx.path,
            status = %status.as_u16(),
            latency_ms = latency_ms,
            "Request completed with client error"
        );
    } else if status.is_server_error() {
        tracing::error!(
            request_id = %ctx.request_id,
            method = %ctx.method,
            path = %ctx.path,
            status = %status.as_u16(),
            latency_ms = latency_ms,
            "Request completed with server error"
        );
    }

    response
}

/// Convert any error to AppError
#[allow(dead_code)]
pub trait IntoAppError<T> {
    fn into_app_error(self) -> Result<T, AppError>;
}

impl<T, E> IntoAppError<T> for Result<T, E>
where
    E: Into<AppError>,
{
    fn into_app_error(self) -> Result<T, AppError> {
        self.map_err(Into::into)
    }
}

/// Result type alias for application errors
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            AppError::validation("test").status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::not_found("test").status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AppError::config("test").status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_error_types() {
        assert_eq!(
            AppError::validation("test").error_type(),
            "validation_error"
        );
        assert_eq!(AppError::not_found("test").error_type(), "not_found");
        assert_eq!(AppError::config("test").error_type(), "config_error");
    }

    #[test]
    fn test_safe_messages() {
        let validation_error = AppError::validation("Invalid input");
        assert_eq!(validation_error.safe_message(), "Invalid input");

        let internal_error = AppError::internal("Sensitive details");
        assert_eq!(internal_error.safe_message(), "Internal server error");
    }

    #[test]
    fn test_error_logging_levels() {
        assert!(AppError::Database(sqlx::Error::PoolClosed).should_log_as_error());
        assert!(!AppError::validation("test").should_log_as_error());
    }

    #[test]
    fn test_request_context() {
        let ctx = RequestContext::new("GET".to_string(), "/test".to_string());
        assert_eq!(ctx.method, "GET");
        assert_eq!(ctx.path, "/test");
        assert!(!ctx.request_id.is_empty());
    }

    // Note: Middleware testing requires a full Axum application setup
    // For now, we test the core error handling functionality
}
