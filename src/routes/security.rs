use axum::{
    Router,
    extract::{Query, State},
    response::{Json, Result as AxumResult},
    routing::get,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{error, info};

use crate::app_state::AppState;
use crate::db::SecurityRepository;
use crate::models::security::{AuditReportsResponse, ExploitHistoryResponse, SecurityDataResponse};
use crate::utils::errors::AppError;

/// Query parameters for filtering security data
#[derive(Debug, Deserialize)]
pub struct SecurityQueryParams {
    /// Filter by bridge name
    pub bridge: Option<String>,
    /// Minimum loss amount for exploits (in USD)
    pub _min_loss: Option<f64>,
    /// Include only major exploits (>= $10M)
    pub _major_only: Option<bool>,
}

/// Query parameters for audit reports
#[derive(Debug, Deserialize)]
pub struct AuditQueryParams {
    /// Filter by bridge name
    pub bridge: Option<String>,
}

/// Query parameters for exploit history
#[derive(Debug, Deserialize)]
pub struct ExploitQueryParams {
    /// Filter by bridge name
    pub bridge: Option<String>,
    /// Minimum loss amount (in USD)
    pub min_loss: Option<f64>,
    /// Include only major exploits (>= $10M)
    pub major_only: Option<bool>,
}

/// GET /security/audits - Get audit reports
///
/// Returns all audit reports or filtered by bridge name
///
/// Query parameters:
/// - bridge: Filter by specific bridge name
///
/// Example: GET /security/audits?bridge=Connext
pub async fn get_audit_reports(
    Query(params): Query<AuditQueryParams>,
    State(app_state): State<Arc<AppState>>,
) -> AxumResult<Json<AuditReportsResponse>> {
    let audits = match params.bridge {
        Some(bridge_name) => {
            info!("Fetching audit reports for bridge: {}", bridge_name);
            SecurityRepository::get_audit_reports_by_bridge(app_state.db(), &bridge_name)
                .await
                .map_err(|e| {
                    error!(
                        "Failed to fetch audit reports for bridge {}: {}",
                        bridge_name, e
                    );
                    e
                })?
        }
        None => {
            info!("Fetching all audit reports");
            SecurityRepository::get_audit_reports_cached(app_state.db(), app_state.cache())
                .await
                .map_err(|e| {
                    error!("Failed to fetch audit reports: {}", e);
                    e
                })?
        }
    };

    info!("Successfully retrieved {} audit reports", audits.len());
    Ok(Json(AuditReportsResponse { audits }))
}

/// GET /security/exploits - Get exploit history
///
/// Returns all exploit history or filtered by parameters
///
/// Query parameters:
/// - bridge: Filter by specific bridge name
/// - min_loss: Minimum loss amount in USD
/// - major_only: If true, only return exploits >= $10M
///
/// Example: GET /security/exploits?major_only=true
pub async fn get_exploit_history(
    Query(params): Query<ExploitQueryParams>,
    State(app_state): State<Arc<AppState>>,
) -> AxumResult<Json<ExploitHistoryResponse>> {
    let exploits = if params.major_only.unwrap_or(false) {
        info!("Fetching major exploits only (>= $10M)");
        let min_amount = rust_decimal::Decimal::new(10_000_000, 0);
        SecurityRepository::get_major_exploits(app_state.db(), min_amount)
            .await
            .map_err(|e| {
                error!("Failed to fetch major exploits: {}", e);
                e
            })?
    } else if let Some(min_loss) = params.min_loss {
        info!("Fetching exploits with minimum loss: ${}", min_loss);
        let min_amount = rust_decimal::Decimal::try_from(min_loss).map_err(|e| {
            error!("Invalid min_loss parameter: {}", e);
            AppError::Validation(format!("Invalid min_loss parameter: {}", e))
        })?;
        SecurityRepository::get_major_exploits(app_state.db(), min_amount)
            .await
            .map_err(|e| {
                error!("Failed to fetch exploits with min_loss {}: {}", min_loss, e);
                e
            })?
    } else if let Some(bridge_name) = params.bridge {
        info!("Fetching exploit history for bridge: {}", bridge_name);
        SecurityRepository::get_exploit_history_by_bridge(app_state.db(), &bridge_name)
            .await
            .map_err(|e| {
                error!(
                    "Failed to fetch exploit history for bridge {}: {}",
                    bridge_name, e
                );
                e
            })?
    } else {
        info!("Fetching all exploit history");
        SecurityRepository::get_exploit_history(app_state.db())
            .await
            .map_err(|e| {
                error!("Failed to fetch exploit history: {}", e);
                e
            })?
    };

    info!(
        "Successfully retrieved {} exploit history records",
        exploits.len()
    );
    Ok(Json(ExploitHistoryResponse { exploits }))
}

/// GET /security - Get combined security data
///
/// Returns both audit reports and exploit history
///
/// Query parameters:
/// - bridge: Filter by specific bridge name
///
/// Example: GET /security?bridge=Connext
pub async fn get_security_data(
    Query(params): Query<SecurityQueryParams>,
    State(app_state): State<Arc<AppState>>,
) -> AxumResult<Json<SecurityDataResponse>> {
    let (audits, exploits) = if let Some(bridge_name) = params.bridge {
        info!("Fetching security data for bridge: {}", bridge_name);
        let audits_fut =
            SecurityRepository::get_audit_reports_by_bridge(app_state.db(), &bridge_name);
        let exploits_fut =
            SecurityRepository::get_exploit_history_by_bridge(app_state.db(), &bridge_name);

        let (audits_result, exploits_result) = tokio::join!(audits_fut, exploits_fut);

        let audits = audits_result.map_err(|e| {
            error!(
                "Failed to fetch audit reports for bridge {}: {}",
                bridge_name, e
            );
            e
        })?;

        let exploits = exploits_result.map_err(|e| {
            error!(
                "Failed to fetch exploit history for bridge {}: {}",
                bridge_name, e
            );
            e
        })?;

        (audits, exploits)
    } else {
        info!("Fetching all security data");
        let audits_fut = SecurityRepository::get_audit_reports(app_state.db());
        let exploits_fut = SecurityRepository::get_exploit_history(app_state.db());

        let (audits_result, exploits_result) = tokio::join!(audits_fut, exploits_fut);

        let audits = audits_result.map_err(|e| {
            error!("Failed to fetch audit reports: {}", e);
            e
        })?;

        let exploits = exploits_result.map_err(|e| {
            error!("Failed to fetch exploit history: {}", e);
            e
        })?;

        (audits, exploits)
    };

    info!(
        "Successfully retrieved {} audit reports and {} exploit history records",
        audits.len(),
        exploits.len()
    );

    Ok(Json(SecurityDataResponse { audits, exploits }))
}

/// GET /security/health - Health check for security endpoints
pub async fn security_health_check() -> AxumResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "status": "ok",
        "service": "security",
        "endpoints": [
            "/security/audits",
            "/security/exploits",
            "/security"
        ]
    })))
}

/// Create and configure the security routes
pub fn security_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/security", get(get_security_data))
        .route("/security/audits", get(get_audit_reports))
        .route("/security/exploits", get(get_exploit_history))
        .route("/security/health", get(security_health_check))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::security::*;

    #[test]
    fn test_security_query_params_deserialization() {
        // Test that our query parameter structs deserialize correctly
        // This would typically be tested as part of integration tests

        // let query_str = "bridge=Connext&min_loss=1000000&major_only=true";
        // In a real test, you would parse this with axum's Query extractor

        // For now, just test the struct definitions are valid
        let params = SecurityQueryParams {
            bridge: Some("Connext".to_string()),
            _min_loss: Some(1000000.0),
            _major_only: Some(true),
        };

        assert_eq!(params.bridge, Some("Connext".to_string()));
        assert_eq!(params._min_loss, Some(1000000.0));
        assert_eq!(params._major_only, Some(true));
    }

    #[test]
    fn test_response_serialization() {
        use chrono::NaiveDate;
        use rust_decimal::Decimal;

        let audit = AuditReport {
            id: 1,
            bridge: "TestBridge".to_string(),
            audit_firm: "TestFirm".to_string(),
            audit_date: NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            result: "passed".to_string(),
            created_at: None,
        };

        let exploit = ExploitHistory {
            id: 1,
            bridge: "TestBridge".to_string(),
            incident_date: NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
            loss_amount: Some(Decimal::new(1000000, 0)),
            description: "Test exploit".to_string(),
            created_at: None,
        };

        let response = SecurityDataResponse {
            audits: vec![audit],
            exploits: vec![exploit],
        };

        // Test that the response can be serialized to JSON
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("TestBridge"));
        assert!(json.contains("passed"));
        assert!(json.contains("Test exploit"));
    }
}
