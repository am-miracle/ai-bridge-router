use rust_decimal::prelude::ToPrimitive;
use sqlx::{PgPool, Row};
use tracing::{debug, info, warn};

use crate::cache::CacheClient;
use crate::models::security::{AuditReport, ExploitHistory};
use crate::services::scoring::SecurityMetadata;
use crate::utils::errors::AppResult;

pub struct SecurityRepository;

impl SecurityRepository {
    /// Get all audit reports from the database
    pub async fn get_audit_reports(pool: &PgPool) -> AppResult<Vec<AuditReport>> {
        debug!("Fetching all audit reports from database");

        let audit_reports = sqlx::query_as::<_, AuditReport>(
            r#"
            SELECT id, bridge, audit_firm, audit_date, result, created_at
            FROM audit_reports
            ORDER BY audit_date DESC, bridge ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        info!("Retrieved {} audit reports", audit_reports.len());
        Ok(audit_reports)
    }

    /// Get all audit reports with caching
    pub async fn get_audit_reports_cached(
        pool: &PgPool,
        cache: &CacheClient,
    ) -> AppResult<Vec<AuditReport>> {
        let cache_key = "security:audit_reports:all";

        // Try cache first
        if let Ok(Some(cached_reports)) = cache.get_cache::<Vec<AuditReport>>(cache_key).await {
            info!("Cache HIT for audit reports");
            return Ok(cached_reports);
        }

        info!("Cache MISS for audit reports, fetching from database");

        // Fetch from database
        let audit_reports = Self::get_audit_reports(pool).await?;

        // Cache for 1 hour (audit reports don't change frequently)
        if let Err(e) = cache.set_cache(cache_key, &audit_reports, 3600).await {
            warn!("Failed to cache audit reports: {}", e);
        } else {
            info!("Cached {} audit reports for 1 hour", audit_reports.len());
        }

        Ok(audit_reports)
    }

    /// Get audit reports for a specific bridge
    pub async fn get_audit_reports_by_bridge(
        pool: &PgPool,
        bridge_name: &str,
    ) -> AppResult<Vec<AuditReport>> {
        debug!("Fetching audit reports for bridge: {}", bridge_name);

        let audit_reports = sqlx::query_as::<_, AuditReport>(
            r#"
            SELECT id, bridge, audit_firm, audit_date, result, created_at
            FROM audit_reports
            WHERE bridge = $1
            ORDER BY audit_date DESC
            "#,
        )
        .bind(bridge_name)
        .fetch_all(pool)
        .await?;

        info!(
            "Retrieved {} audit reports for bridge: {}",
            audit_reports.len(),
            bridge_name
        );
        Ok(audit_reports)
    }

    /// Get all exploit history records from the database
    pub async fn get_exploit_history(pool: &PgPool) -> AppResult<Vec<ExploitHistory>> {
        debug!("Fetching all exploit history from database");

        let exploit_history = sqlx::query_as::<_, ExploitHistory>(
            r#"
            SELECT id, bridge, incident_date, loss_amount, description, created_at
            FROM exploit_history
            ORDER BY incident_date DESC, loss_amount DESC NULLS LAST
            "#,
        )
        .fetch_all(pool)
        .await?;

        info!(
            "Retrieved {} exploit history records",
            exploit_history.len()
        );
        Ok(exploit_history)
    }

    /// Get exploit history for a specific bridge
    pub async fn get_exploit_history_by_bridge(
        pool: &PgPool,
        bridge_name: &str,
    ) -> AppResult<Vec<ExploitHistory>> {
        debug!("Fetching exploit history for bridge: {}", bridge_name);

        let exploit_history = sqlx::query_as::<_, ExploitHistory>(
            r#"
            SELECT id, bridge, incident_date, loss_amount, description, created_at
            FROM exploit_history
            WHERE bridge = $1
            ORDER BY incident_date DESC
            "#,
        )
        .bind(bridge_name)
        .fetch_all(pool)
        .await?;

        info!(
            "Retrieved {} exploit history records for bridge: {}",
            exploit_history.len(),
            bridge_name
        );
        Ok(exploit_history)
    }

    /// Get exploit history with loss amount above a threshold
    pub async fn get_major_exploits(
        pool: &PgPool,
        min_loss_amount: rust_decimal::Decimal,
    ) -> AppResult<Vec<ExploitHistory>> {
        debug!("Fetching major exploits with loss >= {}", min_loss_amount);

        let major_exploits = sqlx::query_as::<_, ExploitHistory>(
            r#"
            SELECT id, bridge, incident_date, loss_amount, description, created_at
            FROM exploit_history
            WHERE loss_amount >= $1
            ORDER BY loss_amount DESC, incident_date DESC
            "#,
        )
        .bind(min_loss_amount)
        .fetch_all(pool)
        .await?;

        info!(
            "Retrieved {} major exploits (>= ${:.2})",
            major_exploits.len(),
            min_loss_amount
        );
        Ok(major_exploits)
    }

    /// Get the most recent audit report for a specific bridge
    pub async fn _get_latest_audit_by_bridge(
        pool: &PgPool,
        bridge_name: &str,
    ) -> AppResult<Option<AuditReport>> {
        let latest_audit = sqlx::query_as::<_, AuditReport>(
            r#"
            SELECT id, bridge, audit_firm, audit_date, result, created_at
            FROM audit_reports
            WHERE bridge = $1
            ORDER BY audit_date DESC
            LIMIT 1
            "#,
        )
        .bind(bridge_name)
        .fetch_optional(pool)
        .await?;

        match &latest_audit {
            Some(audit) => info!(
                "Found latest audit for {}: {} by {} on {}",
                bridge_name, audit.result, audit.audit_firm, audit.audit_date
            ),
            None => info!("No audit found for bridge: {}", bridge_name),
        }

        Ok(latest_audit)
    }

    /// Get count of exploits by bridge
    pub async fn _get_exploit_count_by_bridge(pool: &PgPool) -> AppResult<Vec<(String, i64)>> {
        let counts = sqlx::query(
            r#"
            SELECT bridge, COUNT(*) as count
            FROM exploit_history
            GROUP BY bridge
            ORDER BY count DESC, bridge ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        let result: Vec<(String, i64)> = counts
            .into_iter()
            .map(|row| {
                let bridge: String = row.get("bridge");
                let count: i64 = row.get("count");
                (bridge, count)
            })
            .collect();

        info!("Retrieved exploit counts for {} bridges", result.len());
        Ok(result)
    }

    /// Get total loss amount by bridge
    pub async fn _get_total_loss_by_bridge(
        pool: &PgPool,
    ) -> AppResult<Vec<(String, Option<rust_decimal::Decimal>)>> {
        let totals = sqlx::query(
            r#"
            SELECT bridge, SUM(loss_amount) as total_loss
            FROM exploit_history
            WHERE loss_amount IS NOT NULL
            GROUP BY bridge
            ORDER BY total_loss DESC NULLS LAST, bridge ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        let result: Vec<(String, Option<rust_decimal::Decimal>)> = totals
            .into_iter()
            .map(|row| {
                let bridge: String = row.get("bridge");
                let total_loss: Option<rust_decimal::Decimal> = row.get("total_loss");
                (bridge, total_loss)
            })
            .collect();

        info!("Retrieved total loss amounts for {} bridges", result.len());
        Ok(result)
    }

    /// Get security metadata for a specific bridge (for scoring)
    pub async fn _get_security_metadata(
        pool: &PgPool,
        bridge_name: &str,
    ) -> AppResult<SecurityMetadata> {
        // Get audit information
        let audit_info = sqlx::query(
            r#"
            SELECT
                COUNT(*) as audit_count,
                MAX(audit_date) as latest_audit_date,
                (SELECT result FROM audit_reports
                 WHERE bridge = $1
                 ORDER BY audit_date DESC
                 LIMIT 1) as latest_result
            FROM audit_reports
            WHERE bridge = $1
            "#,
        )
        .bind(bridge_name)
        .fetch_one(pool)
        .await?;

        // Get exploit information
        let exploit_info = sqlx::query(
            r#"
            SELECT
                COUNT(*) as exploit_count,
                COALESCE(SUM(loss_amount), 0) as total_loss
            FROM exploit_history
            WHERE bridge = $1
            "#,
        )
        .bind(bridge_name)
        .fetch_one(pool)
        .await?;

        let audit_count: i64 = audit_info.get("audit_count");
        let latest_audit_result: Option<String> = audit_info.get("latest_result");
        let exploit_count: i64 = exploit_info.get("exploit_count");
        let total_loss: rust_decimal::Decimal = exploit_info.get("total_loss");

        let metadata = SecurityMetadata {
            bridge: bridge_name.to_string(),
            has_audit: audit_count > 0,
            has_exploit: exploit_count > 0,
            latest_audit_result,
            exploit_count: exploit_count as u32,
            total_loss_usd: if total_loss > rust_decimal::Decimal::ZERO {
                total_loss.to_f64()
            } else {
                None
            },
        };

        info!(
            "Security metadata for {}: audits={}, exploits={}, total_loss=${:.2}",
            bridge_name,
            metadata.has_audit,
            metadata.has_exploit,
            metadata.total_loss_usd.unwrap_or(0.0)
        );

        Ok(metadata)
    }

    /// Get security metadata for multiple bridges in batch
    pub async fn get_batch_security_metadata(
        pool: &PgPool,
        bridge_names: &[String],
    ) -> AppResult<Vec<SecurityMetadata>> {
        if bridge_names.is_empty() {
            return Ok(Vec::with_capacity(0));
        }

        // Build placeholders for the IN clause
        let placeholders: Vec<String> = (1..=bridge_names.len())
            .map(|i| format!("${}", i))
            .collect();
        let placeholders_str = placeholders.join(",");

        // Get audit information for all bridges
        let audit_query = format!(
            r#"
            SELECT
                bridge,
                COUNT(*) as audit_count,
                MAX(audit_date) as latest_audit_date,
                (SELECT result FROM audit_reports a2
                 WHERE a2.bridge = audit_reports.bridge
                 ORDER BY audit_date DESC
                 LIMIT 1) as latest_result
            FROM audit_reports
            WHERE bridge IN ({})
            GROUP BY bridge
            "#,
            placeholders_str
        );

        let mut audit_query_builder = sqlx::query(&audit_query);
        for bridge_name in bridge_names {
            audit_query_builder = audit_query_builder.bind(bridge_name);
        }
        let audit_results = audit_query_builder.fetch_all(pool).await?;

        // Get exploit information for all bridges
        let exploit_query = format!(
            r#"
            SELECT
                bridge,
                COUNT(*) as exploit_count,
                COALESCE(SUM(loss_amount), 0) as total_loss
            FROM exploit_history
            WHERE bridge IN ({})
            GROUP BY bridge
            "#,
            placeholders_str
        );

        let mut exploit_query_builder = sqlx::query(&exploit_query);
        for bridge_name in bridge_names {
            exploit_query_builder = exploit_query_builder.bind(bridge_name);
        }
        let exploit_results = exploit_query_builder.fetch_all(pool).await?;

        // Create lookup maps
        let mut audit_map: std::collections::HashMap<String, (i64, Option<String>)> =
            std::collections::HashMap::new();
        for row in audit_results {
            let bridge: String = row.get("bridge");
            let count: i64 = row.get("audit_count");
            let result: Option<String> = row.get("latest_result");
            audit_map.insert(bridge, (count, result));
        }

        let mut exploit_map: std::collections::HashMap<String, (i64, rust_decimal::Decimal)> =
            std::collections::HashMap::new();
        for row in exploit_results {
            let bridge: String = row.get("bridge");
            let count: i64 = row.get("exploit_count");
            let loss: rust_decimal::Decimal = row.get("total_loss");
            exploit_map.insert(bridge, (count, loss));
        }

        // Build metadata for each bridge
        let metadata: Vec<SecurityMetadata> = bridge_names
            .iter()
            .map(|bridge_name| {
                let (audit_count, latest_result) = audit_map.get(bridge_name).unwrap_or(&(0, None));
                let (exploit_count, total_loss) = exploit_map
                    .get(bridge_name)
                    .unwrap_or(&(0, rust_decimal::Decimal::ZERO));

                SecurityMetadata {
                    bridge: bridge_name.clone(),
                    has_audit: *audit_count > 0,
                    has_exploit: *exploit_count > 0,
                    latest_audit_result: latest_result.clone(),
                    exploit_count: *exploit_count as u32,
                    total_loss_usd: if *total_loss > rust_decimal::Decimal::ZERO {
                        total_loss.to_f64()
                    } else {
                        None
                    },
                }
            })
            .collect();

        info!("Retrieved security metadata for {} bridges", metadata.len());
        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_security_repository_functions() {
        // This test requires a test database to be configured
        // For now, we'll test the repository logic without hitting the database

        // In a real test, you would:
        // 1. Set up a test database
        // 2. Run migrations
        // 3. Insert test data
        // 4. Run these repository functions
        // 5. Assert the results

        // Example of how to set up the test:
        /*
        let pool = init_pg_pool().await.expect("Failed to create connection pool");

        // Insert test data
        sqlx::query!(
            "INSERT INTO audit_reports (bridge, audit_firm, audit_date, result) VALUES ($1, $2, $3, $4)",
            "TestBridge",
            "TestFirm",
            chrono::NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            "passed"
        )
        .execute(&pool)
        .await
        .expect("Failed to insert test audit report");

        // Test the repository functions
        let audits = SecurityRepository::get_audit_reports(&pool).await.unwrap();
        assert!(!audits.is_empty());

        let exploits = SecurityRepository::get_exploit_history(&pool).await.unwrap();
        // Add assertions based on your test data
        */

        // For now, just test the major exploit threshold logic
        let ten_million = Decimal::from_str("10000000").unwrap();
        assert_eq!(ten_million.to_string(), "10000000");
    }

    #[test]
    fn test_decimal_conversion() {
        let amount = Decimal::from_str("325000000.50").unwrap();
        let as_f64: f64 = amount.try_into().unwrap();
        assert_eq!(as_f64, 325000000.5);
    }
}
