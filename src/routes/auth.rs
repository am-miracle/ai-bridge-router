use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
};
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::middleware::auth::{
    ApiKeyResponse, AuthContext, CreateApiKeyRequest, create_api_key, list_api_keys, revoke_api_key,
};

/// Create a new API key (admin only)
pub async fn create_api_key_endpoint(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKeyResponse>, (StatusCode, Json<serde_json::Value>)> {
    match create_api_key(request, &app_state).await {
        Ok(api_key) => {
            info!("Successfully created API key: {}", api_key.name);
            Ok(Json(api_key))
        }
        Err(e) => {
            error!("Failed to create API key: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to create API key",
                    "message": e.to_string()
                })),
            ))
        }
    }
}

/// List all API keys (admin only)
pub async fn list_api_keys_endpoint(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Vec<ApiKeyResponse>>, (StatusCode, Json<serde_json::Value>)> {
    match list_api_keys(&app_state).await {
        Ok(api_keys) => {
            info!("Successfully listed {} API keys", api_keys.len());
            Ok(Json(api_keys))
        }
        Err(e) => {
            error!("Failed to list API keys: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to list API keys",
                    "message": e.to_string()
                })),
            ))
        }
    }
}

/// Revoke an API key (admin only)
pub async fn revoke_api_key_endpoint(
    State(app_state): State<Arc<AppState>>,
    axum::extract::Path(api_key_id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match revoke_api_key(api_key_id, &app_state).await {
        Ok(_) => {
            info!("Successfully revoked API key: {}", api_key_id);
            Ok(Json(json!({
                "message": "API key revoked successfully",
                "api_key_id": api_key_id
            })))
        }
        Err(e) => {
            error!("Failed to revoke API key: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to revoke API key",
                    "message": e.to_string()
                })),
            ))
        }
    }
}

/// Get current API key info
pub async fn get_api_key_info(
    axum::extract::Extension(auth_context): axum::extract::Extension<AuthContext>,
) -> Json<serde_json::Value> {
    Json(json!({
        "api_key_id": auth_context.api_key_id,
        "name": auth_context.name,
        "permissions": auth_context.permissions,
        "rate_limit_per_minute": auth_context.rate_limit_per_minute,
        "rate_limit_per_hour": auth_context.rate_limit_per_hour
    }))
}

/// Health check for auth endpoints
pub async fn auth_health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "auth",
        "endpoints": [
            "/auth/api-keys",
            "/auth/api-keys/{id}",
            "/auth/me"
        ]
    }))
}

/// Create auth routes
pub fn auth_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/api-keys", post(create_api_key_endpoint))
        .route("/auth/api-keys", get(list_api_keys_endpoint))
        .route("/auth/api-keys/{id}", delete(revoke_api_key_endpoint))
        .route("/auth/me", get(get_api_key_info))
        .route("/auth/health", get(auth_health_check))
}
