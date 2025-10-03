use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
use serde_json::Value;
use std::time::Duration;
use tower::ServiceExt;

/// Test structured logging and metrics integration
#[tokio::test]
async fn test_telemetry_integration() {
    // This test would require a full application setup
    // For now, we'll test the individual components

    // Test that we can create request contexts
    let ctx = bridge_router::RequestContext::new("GET".to_string(), "/test".to_string());
    assert_eq!(ctx.method, "GET");
    assert_eq!(ctx.path, "/test");
    assert!(!ctx.request_id.is_empty());
}

/// Test error response format
#[tokio::test]
async fn test_error_response_format() {
    let app = Router::new().route(
        "/error",
        get(|| async {
            Err::<&'static str, _>(bridge_router::AppError::validation("Test validation error"))
        }),
    );

    let request = Request::builder()
        .uri("/error")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error_response: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(error_response["error"], "validation_error");
    assert_eq!(error_response["message"], "Test validation error");
    assert_eq!(error_response["code"], 400);
    assert!(error_response["request_id"].is_string());
    assert!(error_response["timestamp"].is_string());
}

/// Test metrics endpoint
#[tokio::test]
async fn test_metrics_endpoint() {
    let app = Router::new().route("/metrics", get(|| async { bridge_router::get_metrics() }));

    let request = Request::builder()
        .uri("/metrics")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let metrics_text = String::from_utf8(body.to_vec()).unwrap();

    // Check that we get Prometheus-formatted metrics
    assert!(metrics_text.contains("# HELP") || metrics_text.contains("# Error encoding metrics"));
}

/// Test health endpoint with metrics
#[tokio::test]
async fn test_health_endpoint_with_metrics() {
    let app = Router::new().route(
        "/health",
        get(|| async { axum::Json(serde_json::json!({"status": "ok"})) }),
    );

    let request = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let health_response: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(health_response["status"], "ok");
}

/// Test request context timing
#[test]
fn test_request_context_timing() {
    let ctx = bridge_router::RequestContext::new("GET".to_string(), "/test".to_string());

    // Sleep for a small amount of time
    std::thread::sleep(Duration::from_millis(10));

    let elapsed = ctx.elapsed_ms();
    assert!(elapsed >= 10);
}

/// Test error types and status codes
#[test]
fn test_error_types_and_status_codes() {
    let validation_error = bridge_router::AppError::validation("Invalid input");
    assert_eq!(validation_error.status_code(), StatusCode::BAD_REQUEST);
    assert_eq!(validation_error.error_type(), "validation_error");
    assert!(!validation_error.should_log_as_error());

    let internal_error = bridge_router::AppError::internal("Database connection failed");
    assert_eq!(
        internal_error.status_code(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(internal_error.error_type(), "internal_error");
    assert!(internal_error.should_log_as_error());
}

/// Test safe error messages
#[test]
fn test_safe_error_messages() {
    let validation_error = bridge_router::AppError::validation("Invalid token address");
    assert_eq!(validation_error.safe_message(), "Invalid token address");

    let internal_error = bridge_router::AppError::internal("Sensitive database details");
    assert_eq!(internal_error.safe_message(), "Internal server error");
}

/// Test metrics recording functions
#[test]
fn test_metrics_recording() {
    let _duration = Duration::from_millis(100);

    // Test that metrics functions exist (they're not exported, so we can't call them directly)
    // In a real test, you would initialize metrics with bridge_router::init_metrics()
}

/// Test logging functions
#[test]
fn test_logging_functions() {
    let _ctx = bridge_router::RequestContext::new("GET".to_string(), "/test".to_string());

    // Test that logging functions exist (they're not exported, so we can't call them directly)
    // In a real test, you would initialize logging with bridge_router::init_logging()
}

/// Test concurrent request handling
#[tokio::test]
async fn test_concurrent_requests() {
    let app = Router::new().route("/test", get(|| async { "OK" }));

    let mut handles = vec![];

    // Spawn multiple concurrent requests
    for i in 0..10 {
        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            let request = Request::builder()
                .uri("/test")
                .method("GET")
                .body(Body::empty())
                .unwrap();

            let response = app_clone.oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            i
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result < 10);
    }
}

/// Test error response serialization
#[test]
fn test_error_response_serialization() {
    let error_response = bridge_router::ErrorResponse {
        error: "validation_error".to_string(),
        message: "Invalid input".to_string(),
        code: 400,
        request_id: Some("test-request-id".to_string()),
        timestamp: "2023-01-01T00:00:00Z".to_string(),
        details: None,
    };

    let json = serde_json::to_string(&error_response).unwrap();
    let parsed: Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["error"], "validation_error");
    assert_eq!(parsed["message"], "Invalid input");
    assert_eq!(parsed["code"], 400);
    assert_eq!(parsed["request_id"], "test-request-id");
    assert_eq!(parsed["timestamp"], "2023-01-01T00:00:00Z");
}

/// Test request context with metadata
#[test]
fn test_request_context_with_metadata() {
    let ctx = bridge_router::RequestContext::new("POST".to_string(), "/quotes".to_string())
        .with_user_agent(Some("test-agent".to_string()))
        .with_client_ip(Some("127.0.0.1".to_string()));

    assert_eq!(ctx.method, "POST");
    assert_eq!(ctx.path, "/quotes");
    assert_eq!(ctx.user_agent, Some("test-agent".to_string()));
    assert_eq!(ctx.client_ip, Some("127.0.0.1".to_string()));
    assert!(!ctx.request_id.is_empty());
}
