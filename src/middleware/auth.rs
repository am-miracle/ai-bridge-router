use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::utils::errors::{AppError, AppResult};

/// API Key information
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub key_hash: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub rate_limit_per_minute: i32,
    pub rate_limit_per_hour: i32,
    pub is_active: bool,
    pub created_at: chrono::NaiveDateTime,
    pub last_used_at: Option<chrono::NaiveDateTime>,
    pub expires_at: Option<chrono::NaiveDateTime>,
}

/// API Key creation request
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub rate_limit_per_minute: Option<i32>,
    pub rate_limit_per_hour: Option<i32>,
    pub expires_at: Option<chrono::NaiveDateTime>,
}

/// API Key response (without sensitive data)
#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub rate_limit_per_minute: i32,
    pub rate_limit_per_hour: i32,
    pub is_active: bool,
    pub created_at: chrono::NaiveDateTime,
    pub last_used_at: Option<chrono::NaiveDateTime>,
    pub expires_at: Option<chrono::NaiveDateTime>,
    pub key: String, // Only returned on creation
}

/// Authentication context
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub api_key_id: Uuid,
    pub name: String,
    pub permissions: Vec<String>,
    pub rate_limit_per_minute: i32,
    pub rate_limit_per_hour: i32,
}

/// Extract API key from request headers
pub fn extract_api_key(headers: &HeaderMap) -> AppResult<String> {
    // Try X-API-Key header first
    if let Some(api_key) = headers.get("X-API-Key") {
        return Ok(api_key
            .to_str()
            .map_err(|_| AppError::Authentication("Invalid API key format".to_string()))?
            .to_string());
    }

    // Try Authorization header with Bearer token
    if let Some(auth_header) = headers.get("Authorization") {
        let auth_str = auth_header
            .to_str()
            .map_err(|_| AppError::Authentication("Invalid authorization header".to_string()))?;

        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            return Ok(token.to_string());
        }
    }

    Err(AppError::Authentication("Missing API key".to_string()))
}

/// Hash API key for storage
pub fn hash_api_key(key: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Validate API key against database
pub async fn validate_api_key(api_key: &str, app_state: &Arc<AppState>) -> AppResult<AuthContext> {
    let key_hash = hash_api_key(api_key);

    // Query database for API key
    let api_key_record = sqlx::query_as::<_, ApiKey>(
        r#"
        SELECT
            id,
            key_hash,
            name,
            description,
            permissions,
            rate_limit_per_minute,
            rate_limit_per_hour,
            is_active,
            created_at,
            last_used_at,
            expires_at
        FROM api_keys
        WHERE key_hash = $1 AND is_active = true
        "#,
    )
    .bind(&key_hash)
    .fetch_optional(app_state.db())
    .await?
    .ok_or_else(|| AppError::Authentication("Invalid API key".to_string()))?;

    // Check if key is expired
    if let Some(expires_at) = api_key_record.expires_at
        && chrono::Utc::now().naive_utc() > expires_at
    {
        return Err(AppError::Authentication("API key expired".to_string()));
    }

    // Update last used timestamp
    sqlx::query!(
        "UPDATE api_keys SET last_used_at = NOW() WHERE id = $1",
        api_key_record.id
    )
    .execute(app_state.db())
    .await?;

    info!("API key validated for: {}", api_key_record.name);

    Ok(AuthContext {
        api_key_id: api_key_record.id,
        name: api_key_record.name,
        permissions: api_key_record.permissions,
        rate_limit_per_minute: api_key_record.rate_limit_per_minute,
        rate_limit_per_hour: api_key_record.rate_limit_per_hour,
    })
}

/// Authentication middleware for security endpoints
pub async fn security_auth_middleware(
    State(app_state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    // Extract API key
    let api_key = match extract_api_key(&headers) {
        Ok(key) => key,
        Err(e) => {
            error!("Authentication failed: {}", e);
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Authentication required",
                    "message": e.to_string(),
                    "code": 401
                })),
            )
                .into_response();
        }
    };

    // Validate API key
    let auth_context = match validate_api_key(&api_key, &app_state).await {
        Ok(context) => context,
        Err(e) => {
            error!("API key validation failed: {}", e);
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Authentication failed",
                    "message": e.to_string(),
                    "code": 401
                })),
            )
                .into_response();
        }
    };

    // Check if key has required permissions
    if !auth_context
        .permissions
        .contains(&"security:read".to_string())
    {
        warn!(
            "API key {} attempted to access security endpoint without permission",
            auth_context.name
        );
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Insufficient permissions",
                "message": "API key does not have security:read permission",
                "code": 403
            })),
        )
            .into_response();
    }

    // Check rate limits
    if let Err(e) = check_rate_limits(&auth_context, &app_state).await {
        warn!("Rate limit exceeded for API key: {}", auth_context.name);
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({
                "error": "Rate limit exceeded",
                "message": e.to_string(),
                "code": 429
            })),
        )
            .into_response();
    }

    // Add auth context to request extensions
    let mut request = request;
    request.extensions_mut().insert(auth_context);

    next.run(request).await
}

/// Admin authentication middleware for API key management
pub async fn admin_auth_middleware(
    State(app_state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    // Extract API key
    let api_key = match extract_api_key(&headers) {
        Ok(key) => key,
        Err(e) => {
            error!("Admin authentication failed: {}", e);
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Authentication required",
                    "message": e.to_string(),
                    "code": 401
                })),
            )
                .into_response();
        }
    };

    // Validate API key
    let auth_context = match validate_api_key(&api_key, &app_state).await {
        Ok(context) => context,
        Err(e) => {
            error!("Admin API key validation failed: {}", e);
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Authentication failed",
                    "message": e.to_string(),
                    "code": 401
                })),
            )
                .into_response();
        }
    };

    // Check if key has admin permissions
    if !auth_context
        .permissions
        .contains(&"admin:manage".to_string())
    {
        warn!(
            "API key {} attempted admin access without permission",
            auth_context.name
        );
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Insufficient permissions",
                "message": "API key does not have admin:manage permission",
                "code": 403
            })),
        )
            .into_response();
    }

    // Add auth context to request extensions
    let mut request = request;
    request.extensions_mut().insert(auth_context);

    next.run(request).await
}

/// Check rate limits for API key
async fn check_rate_limits(auth_context: &AuthContext, app_state: &Arc<AppState>) -> AppResult<()> {
    let minute_key = format!("rate_limit:api_key:minute:{}", auth_context.api_key_id);
    let hour_key = format!("rate_limit:api_key:hour:{}", auth_context.api_key_id);

    let minute_count = app_state.cache().increment(&minute_key, 1).await?;
    let hour_count = app_state.cache().increment(&hour_key, 1).await?;

    // Set expiration on first request
    if minute_count == 1 {
        app_state.cache().expire(&minute_key, 60).await?; // 1 minute
    }
    if hour_count == 1 {
        app_state.cache().expire(&hour_key, 3600).await?; // 1 hour
    }

    if minute_count > auth_context.rate_limit_per_minute as i64 {
        return Err(AppError::RateLimited);
    }
    if hour_count > auth_context.rate_limit_per_hour as i64 {
        return Err(AppError::RateLimited);
    }

    Ok(())
}

/// Create a new API key
pub async fn create_api_key(
    request: CreateApiKeyRequest,
    app_state: &Arc<AppState>,
) -> AppResult<ApiKeyResponse> {
    let api_key_id = Uuid::new_v4();
    let raw_key = format!("br_{}", Uuid::new_v4().to_string().replace('-', ""));
    let key_hash = hash_api_key(&raw_key);

    let permissions = serde_json::to_value(&request.permissions)?;

    sqlx::query!(
        r#"
        INSERT INTO api_keys (
            id, key_hash, name, description, permissions,
            rate_limit_per_minute, rate_limit_per_hour, is_active,
            created_at, expires_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), $9)
        "#,
        api_key_id,
        key_hash,
        request.name,
        request.description,
        permissions,
        request.rate_limit_per_minute.unwrap_or(100),
        request.rate_limit_per_hour.unwrap_or(1000),
        true,
        request.expires_at
    )
    .execute(app_state.db())
    .await?;

    info!("Created new API key: {}", request.name);

    Ok(ApiKeyResponse {
        id: api_key_id,
        name: request.name,
        description: request.description,
        permissions: request.permissions,
        rate_limit_per_minute: request.rate_limit_per_minute.unwrap_or(100),
        rate_limit_per_hour: request.rate_limit_per_hour.unwrap_or(1000),
        is_active: true,
        created_at: chrono::Utc::now().naive_utc(),
        last_used_at: None,
        expires_at: request.expires_at,
        key: raw_key,
    })
}

/// List all API keys (admin only)
pub async fn list_api_keys(app_state: &Arc<AppState>) -> AppResult<Vec<ApiKeyResponse>> {
    let api_keys = sqlx::query_as::<_, ApiKey>(
        r#"
        SELECT
            id,
            key_hash,
            name,
            description,
            permissions,
            rate_limit_per_minute,
            rate_limit_per_hour,
            is_active,
            created_at,
            last_used_at,
            expires_at
        FROM api_keys
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(app_state.db())
    .await?;

    let responses: Vec<ApiKeyResponse> = api_keys
        .into_iter()
        .map(|key| ApiKeyResponse {
            id: key.id,
            name: key.name,
            description: key.description,
            permissions: key.permissions,
            rate_limit_per_minute: key.rate_limit_per_minute,
            rate_limit_per_hour: key.rate_limit_per_hour,
            is_active: key.is_active,
            created_at: key.created_at,
            last_used_at: key.last_used_at,
            expires_at: key.expires_at,
            key: "***hidden***".to_string(), // Never return the actual key
        })
        .collect();

    Ok(responses)
}

/// Revoke an API key
pub async fn revoke_api_key(api_key_id: Uuid, app_state: &Arc<AppState>) -> AppResult<()> {
    sqlx::query!(
        "UPDATE api_keys SET is_active = false WHERE id = $1",
        api_key_id
    )
    .execute(app_state.db())
    .await?;

    info!("Revoked API key: {}", api_key_id);
    Ok(())
}
