use serde_json::Value;

use bridge_router::routes::health::{HealthResponse, StatusResponse};

/// Test the HealthResponse struct serialization
#[test]
fn test_health_response_serialization() {
    let response = HealthResponse {
        status: "ok".to_string(),
    };

    let json: Value = serde_json::to_value(&response).unwrap();

    assert_eq!(json["status"], "ok");
    assert!(json.is_object());
    assert!(json.get("status").is_some());
}

/// Test the StatusResponse struct serialization
#[test]
fn test_status_response_serialization() {
    let response = StatusResponse {
        status: "ok".to_string(),
        db: "connected".to_string(),
        cache: "connected".to_string(),
        uptime_seconds: 1234,
        bridges_available: 9,
    };

    let json: Value = serde_json::to_value(&response).unwrap();

    assert_eq!(json["status"], "ok");
    assert_eq!(json["db"], "connected");
    assert_eq!(json["cache"], "connected");
    assert_eq!(json["uptime_seconds"], 1234);
    assert_eq!(json["bridges_available"], 9);
    assert!(json.is_object());
    assert!(json.get("status").is_some());
    assert!(json.get("db").is_some());
    assert!(json.get("cache").is_some());
    assert!(json.get("uptime_seconds").is_some());
    assert!(json.get("bridges_available").is_some());
}

/// Test that the health endpoint returns the expected JSON structure
#[test]
fn test_health_response_structure() {
    let response = HealthResponse {
        status: "ok".to_string(),
    };

    let json: Value = serde_json::to_value(&response).unwrap();

    // Verify the response has the expected structure
    assert!(json.is_object());
    assert!(json.get("status").is_some());
    assert_eq!(json.get("status").unwrap().as_str().unwrap(), "ok");
}

/// Test that the status endpoint returns the expected JSON structure
#[test]
fn test_status_response_structure() {
    let response = StatusResponse {
        status: "ok".to_string(),
        db: "disconnected".to_string(),
        cache: "disconnected".to_string(),
        uptime_seconds: 0,
        bridges_available: 9,
    };

    let json: Value = serde_json::to_value(&response).unwrap();

    // Verify the response has the expected structure
    assert!(json.is_object());
    assert!(json.get("status").is_some());
    assert!(json.get("db").is_some());
    assert!(json.get("cache").is_some());
    assert!(json.get("uptime_seconds").is_some());
    assert!(json.get("bridges_available").is_some());
    assert_eq!(json.get("status").unwrap().as_str().unwrap(), "ok");
    assert_eq!(json.get("db").unwrap().as_str().unwrap(), "disconnected");
    assert_eq!(json.get("cache").unwrap().as_str().unwrap(), "disconnected");
    assert_eq!(json.get("uptime_seconds").unwrap().as_u64().unwrap(), 0);
    assert_eq!(json.get("bridges_available").unwrap().as_u64().unwrap(), 9);
}
