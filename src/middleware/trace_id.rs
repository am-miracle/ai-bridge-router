use axum::{
    extract::Request,
    http::{HeaderValue, header::HeaderName},
    middleware::Next,
    response::Response,
};

use tracing::{Instrument, info_span};
use uuid::Uuid;

/// Extension key for storing trace ID in request extensions
#[derive(Debug, Clone)]
pub struct TraceId(#[allow(dead_code)] pub String);

/// Middleware to generate and inject trace IDs into requests
pub async fn trace_id_middleware(mut request: Request, next: Next) -> Response {
    // Check if trace ID is already provided in request headers
    let trace_id = request
        .headers()
        .get("X-Trace-ID")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            // Generate new trace ID if not provided
            Uuid::new_v4().to_string()
        });

    // Store trace ID in request extensions for use in handlers
    request.extensions_mut().insert(TraceId(trace_id.clone()));

    // Create a span with the trace ID
    let span = info_span!(
        "request",
        trace_id = %trace_id,
        method = %request.method(),
        uri = %request.uri(),
    );

    // Enter the span for the duration of the request
    async move {
        // Process the request
        let mut response = next.run(request).await;

        // Add trace ID to response headers
        if let Ok(header_value) = HeaderValue::from_str(&trace_id) {
            response
                .headers_mut()
                .insert(HeaderName::from_static("x-trace-id"), header_value);
        }

        response
    }
    .instrument(span)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Method, StatusCode},
        response::Json,
        routing::get,
    };
    use serde_json::json;
    use tower::ServiceExt;

    async fn test_handler(request: Request) -> Json<serde_json::Value> {
        let trace_id = request
            .extensions()
            .get::<TraceId>()
            .map(|t| t.0.clone())
            .unwrap_or_else(|| "none".to_string());

        // Test that we can log with trace ID context
        tracing::info!(trace_id = %trace_id, "Test handler called");

        Json(json!({
            "message": "Hello, World!",
            "trace_id": trace_id
        }))
    }

    #[tokio::test]
    async fn test_trace_id_middleware_generates_id() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(axum::middleware::from_fn(trace_id_middleware));

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Check that response has trace ID header
        assert!(response.headers().contains_key("x-trace-id"));

        let trace_id = response.headers().get("x-trace-id").unwrap();
        let trace_id_str = trace_id.to_str().unwrap();

        // Verify it's a valid UUID format
        assert!(Uuid::parse_str(trace_id_str).is_ok());
    }

    #[tokio::test]
    async fn test_trace_id_middleware_preserves_existing_id() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(axum::middleware::from_fn(trace_id_middleware));

        let existing_trace_id = "test-trace-id-123";

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("X-Trace-ID", existing_trace_id)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Check that response preserves the existing trace ID
        let response_trace_id = response
            .headers()
            .get("x-trace-id")
            .unwrap()
            .to_str()
            .unwrap();

        assert_eq!(response_trace_id, existing_trace_id);
    }

    #[tokio::test]
    async fn test_trace_id_extension() {
        let mut request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        // Manually insert trace ID
        let trace_id = "manual-trace-id";
        request
            .extensions_mut()
            .insert(TraceId(trace_id.to_string()));

        // Test manual extraction
        let extracted_trace_id = request
            .extensions()
            .get::<TraceId>()
            .map(|t| t.0.clone())
            .unwrap();
        assert_eq!(extracted_trace_id, trace_id);
    }

    #[test]
    fn test_trace_id_struct() {
        let trace_id = TraceId("test-id".to_string());
        assert_eq!(trace_id.0, "test-id");

        // Test Clone
        let cloned = trace_id.clone();
        assert_eq!(cloned.0, "test-id");
    }

    #[tokio::test]
    async fn test_middleware_with_invalid_header() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(axum::middleware::from_fn(trace_id_middleware));

        // Test with invalid UTF-8 in header (should generate new ID)
        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("X-Trace-ID", &[0xFF, 0xFE][..]) // Invalid UTF-8
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should generate new UUID since the header was invalid
        assert!(response.headers().contains_key("x-trace-id"));
        let trace_id = response
            .headers()
            .get("x-trace-id")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(Uuid::parse_str(trace_id).is_ok());
    }

    #[tokio::test]
    async fn test_span_creation() {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        // Initialize tracing for this test
        let _guard = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_test_writer())
            .set_default();

        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(axum::middleware::from_fn(trace_id_middleware));

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // The test mainly verifies that tracing spans are created without panicking
        // Actual span verification would require more complex tracing infrastructure
    }
}
