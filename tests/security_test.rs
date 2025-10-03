use bridge_router::{
    db::{SecurityRepository, pool::init_pg_pool},
    models::security::*,
};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Integration tests for security repository functions
///
/// Note: These tests require a PostgreSQL database to be available
/// and will create/modify test data. They should be run in isolation.
#[cfg(test)]
mod security_repository_tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Ignored by default since it requires a real database
    async fn test_audit_reports_repository() {
        // Initialize database connection
        let pool = init_pg_pool()
            .await
            .expect("Failed to create database connection pool");

        // Test getting all audit reports
        let all_audits = SecurityRepository::get_audit_reports(&pool)
            .await
            .expect("Failed to fetch audit reports");

        assert!(
            !all_audits.is_empty(),
            "Should have audit reports from seed data"
        );

        // Test getting audit reports for a specific bridge
        let connext_audits = SecurityRepository::get_audit_reports_by_bridge(&pool, "Connext")
            .await
            .expect("Failed to fetch Connext audit reports");

        assert!(
            !connext_audits.is_empty(),
            "Should have Connext audit reports from seed data"
        );

        // Verify all returned audits are for Connext
        for audit in &connext_audits {
            assert_eq!(audit.bridge, "Connext");
        }

        // Test getting latest audit for a bridge
        let latest_connext = SecurityRepository::_get_latest_audit_by_bridge(&pool, "Connext")
            .await
            .expect("Failed to fetch latest Connext audit");

        assert!(
            latest_connext.is_some(),
            "Should have at least one Connext audit"
        );

        if let Some(audit) = latest_connext {
            assert_eq!(audit.bridge, "Connext");
        }

        // Test getting audit for non-existent bridge
        let nonexistent_audits =
            SecurityRepository::get_audit_reports_by_bridge(&pool, "NonExistentBridge")
                .await
                .expect("Failed to fetch audit reports for non-existent bridge");

        assert!(
            nonexistent_audits.is_empty(),
            "Should have no audits for non-existent bridge"
        );
    }

    #[tokio::test]
    #[ignore] // Ignored by default since it requires a real database
    async fn test_exploit_history_repository() {
        // Initialize database connection
        let pool = init_pg_pool()
            .await
            .expect("Failed to create database connection pool");

        // Test getting all exploit history
        let all_exploits = SecurityRepository::get_exploit_history(&pool)
            .await
            .expect("Failed to fetch exploit history");

        assert!(
            !all_exploits.is_empty(),
            "Should have exploit history from seed data"
        );

        // Test getting exploit history for a specific bridge
        let wormhole_exploits =
            SecurityRepository::get_exploit_history_by_bridge(&pool, "Wormhole")
                .await
                .expect("Failed to fetch Wormhole exploit history");

        assert!(
            !wormhole_exploits.is_empty(),
            "Should have Wormhole exploits from seed data"
        );

        // Verify all returned exploits are for Wormhole
        for exploit in &wormhole_exploits {
            assert_eq!(exploit.bridge, "Wormhole");
        }

        // Test getting major exploits (>= $10M)
        let min_amount = Decimal::new(10_000_000, 0);
        let major_exploits = SecurityRepository::get_major_exploits(&pool, min_amount)
            .await
            .expect("Failed to fetch major exploits");

        assert!(
            !major_exploits.is_empty(),
            "Should have major exploits from seed data"
        );

        // Verify all returned exploits meet the minimum threshold
        for exploit in &major_exploits {
            if let Some(loss) = exploit.loss_amount {
                assert!(
                    loss >= min_amount,
                    "Loss amount should be >= {}",
                    min_amount
                );
            }
        }

        // Test getting exploit counts by bridge
        let exploit_counts = SecurityRepository::_get_exploit_count_by_bridge(&pool)
            .await
            .expect("Failed to fetch exploit counts");

        assert!(!exploit_counts.is_empty(), "Should have exploit counts");

        // Verify the structure
        for (bridge, count) in &exploit_counts {
            assert!(!bridge.is_empty(), "Bridge name should not be empty");
            assert!(*count > 0, "Count should be positive");
        }

        // Test getting total loss by bridge
        let total_losses = SecurityRepository::_get_total_loss_by_bridge(&pool)
            .await
            .expect("Failed to fetch total losses");

        assert!(!total_losses.is_empty(), "Should have total loss data");

        // Verify the structure
        for (bridge, total_loss) in &total_losses {
            assert!(!bridge.is_empty(), "Bridge name should not be empty");
            if let Some(loss) = total_loss {
                assert!(*loss > Decimal::ZERO, "Total loss should be positive");
            }
        }
    }
}

/// Unit tests for security models
#[cfg(test)]
mod security_model_tests {
    use super::*;

    #[test]
    fn test_audit_report_methods() {
        let passed_audit = AuditReport {
            id: 1,
            bridge: "TestBridge".to_string(),
            audit_firm: "TestFirm".to_string(),
            audit_date: NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            result: "passed".to_string(),
            created_at: None,
        };

        let failed_audit = AuditReport {
            id: 2,
            bridge: "TestBridge".to_string(),
            audit_firm: "TestFirm".to_string(),
            audit_date: NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            result: "issues found".to_string(),
            created_at: None,
        };

        let unclear_audit = AuditReport {
            id: 3,
            bridge: "TestBridge".to_string(),
            audit_firm: "TestFirm".to_string(),
            audit_date: NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            result: "under review".to_string(),
            created_at: None,
        };

        // Test is_passed method
        assert!(passed_audit.is_passed());
        assert!(!failed_audit.is_passed());
        assert!(!unclear_audit.is_passed());

        // Test has_issues method
        assert!(!passed_audit.has_issues());
        assert!(failed_audit.has_issues());
        assert!(!unclear_audit.has_issues());
    }

    #[test]
    fn test_exploit_history_methods() {
        let major_exploit = ExploitHistory {
            id: 1,
            bridge: "TestBridge".to_string(),
            incident_date: NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
            loss_amount: Some(Decimal::new(50_000_000, 0)), // $50M
            description: "Major exploit".to_string(),
            created_at: None,
        };

        let minor_exploit = ExploitHistory {
            id: 2,
            bridge: "TestBridge".to_string(),
            incident_date: NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
            loss_amount: Some(Decimal::new(5_000_000, 0)), // $5M
            description: "Minor exploit".to_string(),
            created_at: None,
        };

        let unknown_loss_exploit = ExploitHistory {
            id: 3,
            bridge: "TestBridge".to_string(),
            incident_date: NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
            loss_amount: None,
            description: "Unknown loss exploit".to_string(),
            created_at: None,
        };

        // Test loss_amount_f64 method
        assert_eq!(major_exploit.loss_amount_f64(), Some(50_000_000.0));
        assert_eq!(minor_exploit.loss_amount_f64(), Some(5_000_000.0));
        assert_eq!(unknown_loss_exploit.loss_amount_f64(), None);

        // Test is_major_exploit method
        assert!(major_exploit.is_major_exploit());
        assert!(!minor_exploit.is_major_exploit());
        assert!(!unknown_loss_exploit.is_major_exploit());

        // Test formatted_loss method
        assert_eq!(major_exploit.formatted_loss(), "$50.0M");
        assert_eq!(minor_exploit.formatted_loss(), "$5.0M");
        assert_eq!(unknown_loss_exploit.formatted_loss(), "Unknown");
    }

    #[test]
    fn test_formatted_loss_scales() {
        let billion_exploit = ExploitHistory {
            id: 1,
            bridge: "TestBridge".to_string(),
            incident_date: NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
            loss_amount: Some(Decimal::new(1_500_000_000, 0)), // $1.5B
            description: "Billion dollar exploit".to_string(),
            created_at: None,
        };

        let thousand_exploit = ExploitHistory {
            id: 2,
            bridge: "TestBridge".to_string(),
            incident_date: NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
            loss_amount: Some(Decimal::new(50_000, 0)), // $50K
            description: "Thousand dollar exploit".to_string(),
            created_at: None,
        };

        let small_exploit = ExploitHistory {
            id: 3,
            bridge: "TestBridge".to_string(),
            incident_date: NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
            loss_amount: Some(Decimal::new(500, 0)), // $500
            description: "Small exploit".to_string(),
            created_at: None,
        };

        assert_eq!(billion_exploit.formatted_loss(), "$1.5B");
        assert_eq!(thousand_exploit.formatted_loss(), "$50.0K");
        assert_eq!(small_exploit.formatted_loss(), "$500.00");
    }

    #[test]
    fn test_response_serialization() {
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
            loss_amount: Some(Decimal::new(1_000_000, 0)),
            description: "Test exploit".to_string(),
            created_at: None,
        };

        // Test individual response types
        let audit_response = AuditReportsResponse {
            audits: vec![audit.clone()],
        };
        let audit_json = serde_json::to_string(&audit_response).unwrap();
        assert!(audit_json.contains("TestBridge"));
        assert!(audit_json.contains("TestFirm"));

        let exploit_response = ExploitHistoryResponse {
            exploits: vec![exploit.clone()],
        };
        let exploit_json = serde_json::to_string(&exploit_response).unwrap();
        assert!(exploit_json.contains("TestBridge"));
        assert!(exploit_json.contains("Test exploit"));

        // Test combined response
        let combined_response = SecurityDataResponse {
            audits: vec![audit],
            exploits: vec![exploit],
        };
        let combined_json = serde_json::to_string(&combined_response).unwrap();
        assert!(combined_json.contains("audits"));
        assert!(combined_json.contains("exploits"));
        assert!(combined_json.contains("TestBridge"));
    }

    #[test]
    fn test_decimal_conversions() {
        // Test various decimal operations
        let amount = Decimal::from_str("325000000.50").unwrap();
        let as_f64: f64 = amount.try_into().unwrap();
        assert_eq!(as_f64, 325000000.5);

        let zero = Decimal::ZERO;
        assert_eq!(zero.to_string(), "0");

        let negative = Decimal::new(-1000000, 0);
        assert!(negative < Decimal::ZERO);

        let large = Decimal::new(999_999_999_999_999_999, 0);
        assert!(large > Decimal::new(1_000_000_000, 0));
    }
}
