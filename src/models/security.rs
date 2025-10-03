use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Audit report for a bridge protocol
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AuditReport {
    pub id: i32,
    pub bridge: String,
    pub audit_firm: String,
    pub audit_date: NaiveDate,
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::NaiveDateTime>,
}

/// Historical exploit/incident data for a bridge
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ExploitHistory {
    pub id: i32,
    pub bridge: String,
    pub incident_date: NaiveDate,
    pub loss_amount: Option<rust_decimal::Decimal>,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::NaiveDateTime>,
}

/// Response structure for security endpoints
#[derive(Debug, Serialize)]
pub struct SecurityDataResponse {
    pub audits: Vec<AuditReport>,
    pub exploits: Vec<ExploitHistory>,
}

/// Response for audit reports only
#[derive(Debug, Serialize)]
pub struct AuditReportsResponse {
    pub audits: Vec<AuditReport>,
}

/// Response for exploit history only
#[derive(Debug, Serialize)]
pub struct ExploitHistoryResponse {
    pub exploits: Vec<ExploitHistory>,
}

impl AuditReport {
    #[allow(dead_code)]
    /// Check if the audit result indicates a passing grade
    pub fn is_passed(&self) -> bool {
        self.result.to_lowercase().contains("passed")
    }

    /// Check if the audit found issues
    #[allow(dead_code)]
    pub fn has_issues(&self) -> bool {
        self.result.to_lowercase().contains("issues")
            || self.result.to_lowercase().contains("found")
            || self.result.to_lowercase().contains("failed")
    }
}

impl ExploitHistory {
    /// Get the loss amount as f64 for easier handling
    #[allow(dead_code)]
    pub fn loss_amount_f64(&self) -> Option<f64> {
        self.loss_amount
            .map(|amount| amount.try_into().unwrap_or(0.0))
    }

    /// Check if this was a major exploit (>$10M loss)
    #[allow(dead_code)]
    pub fn is_major_exploit(&self) -> bool {
        self.loss_amount_f64()
            .map(|amount| amount >= 10_000_000.0)
            .unwrap_or(false)
    }

    /// Get a formatted loss amount string
    #[allow(dead_code)]
    pub fn formatted_loss(&self) -> String {
        match self.loss_amount_f64() {
            Some(amount) => {
                if amount >= 1_000_000_000.0 {
                    format!("${:.1}B", amount / 1_000_000_000.0)
                } else if amount >= 1_000_000.0 {
                    format!("${:.1}M", amount / 1_000_000.0)
                } else if amount >= 1_000.0 {
                    format!("${:.1}K", amount / 1_000.0)
                } else {
                    format!("${:.2}", amount)
                }
            }
            None => "Unknown".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_audit_report_is_passed() {
        let passed_audit = AuditReport {
            id: 1,
            bridge: "Test".to_string(),
            audit_firm: "TestFirm".to_string(),
            audit_date: NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            result: "passed".to_string(),
            created_at: None,
        };

        let failed_audit = AuditReport {
            id: 2,
            bridge: "Test".to_string(),
            audit_firm: "TestFirm".to_string(),
            audit_date: NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            result: "issues found".to_string(),
            created_at: None,
        };

        assert!(passed_audit.is_passed());
        assert!(!failed_audit.is_passed());
        assert!(!passed_audit.has_issues());
        assert!(failed_audit.has_issues());
    }

    #[test]
    fn test_exploit_history_major_exploit() {
        let major_exploit = ExploitHistory {
            id: 1,
            bridge: "Test".to_string(),
            incident_date: NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
            loss_amount: Some(Decimal::new(50_000_000, 0)), // $50M
            description: "Major exploit".to_string(),
            created_at: None,
        };

        let minor_exploit = ExploitHistory {
            id: 2,
            bridge: "Test".to_string(),
            incident_date: NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
            loss_amount: Some(Decimal::new(5_000_000, 0)), // $5M
            description: "Minor exploit".to_string(),
            created_at: None,
        };

        assert!(major_exploit.is_major_exploit());
        assert!(!minor_exploit.is_major_exploit());
    }

    #[test]
    fn test_formatted_loss() {
        let exploit_billion = ExploitHistory {
            id: 1,
            bridge: "Test".to_string(),
            incident_date: NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
            loss_amount: Some(Decimal::new(1_500_000_000, 0)), // $1.5B
            description: "Huge exploit".to_string(),
            created_at: None,
        };

        let exploit_million = ExploitHistory {
            id: 2,
            bridge: "Test".to_string(),
            incident_date: NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
            loss_amount: Some(Decimal::new(50_000_000, 0)), // $50M
            description: "Large exploit".to_string(),
            created_at: None,
        };

        assert_eq!(exploit_billion.formatted_loss(), "$1.5B");
        assert_eq!(exploit_million.formatted_loss(), "$50.0M");
    }
}
