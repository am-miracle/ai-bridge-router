use crate::cache::CacheClient;
use crate::telemetry::record_redis_metrics;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info, warn};

/// Redis monitoring service for observability
#[allow(dead_code)]
pub struct RedisMonitor {
    cache_client: CacheClient,
    monitoring_interval: Duration,
}

impl RedisMonitor {
    /// Create a new Redis monitor
    #[allow(dead_code)]
    pub fn new(cache_client: CacheClient) -> Self {
        Self {
            cache_client,
            monitoring_interval: Duration::from_secs(30), // Monitor every 30 seconds
        }
    }

    /// Create a new Redis monitor with custom interval
    #[allow(dead_code)]
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.monitoring_interval = interval;
        self
    }

    /// Start monitoring Redis metrics
    #[allow(dead_code)]
    pub async fn start_monitoring(&self) {
        let mut interval = interval(self.monitoring_interval);
        let cache_client = self.cache_client.clone();

        info!(
            "Starting Redis monitoring with {}s interval",
            self.monitoring_interval.as_secs()
        );

        loop {
            interval.tick().await;

            if let Err(e) = self.collect_and_record_metrics(&cache_client).await {
                error!("Failed to collect Redis metrics: {}", e);
            }
        }
    }

    /// Collect Redis metrics and record them
    #[allow(dead_code)]
    async fn collect_and_record_metrics(
        &self,
        cache_client: &CacheClient,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get comprehensive Redis statistics
        let stats = cache_client.get_stats().await?;

        // Record metrics for Prometheus
        record_redis_metrics(&stats);

        // Log key metrics for observability
        info!(
            "Redis metrics: {} keys, {} bytes memory, {} clients, {}s uptime, {} hits, {} misses",
            stats.total_keys,
            stats.used_memory_bytes,
            stats.connected_clients,
            stats.uptime_seconds,
            stats.hits,
            stats.misses
        );

        // Check for potential issues
        self.check_redis_health(&stats).await;

        Ok(())
    }

    /// Check Redis health and log warnings
    #[allow(dead_code)]
    async fn check_redis_health(&self, stats: &crate::cache::CacheStats) {
        // Check memory usage (warn if > 100MB)
        if stats.used_memory_bytes > 100 * 1024 * 1024 {
            warn!(
                "Redis memory usage is high: {} bytes",
                stats.used_memory_bytes
            );
        }

        // Check hit rate (warn if < 80%)
        let total_requests = stats.hits + stats.misses;
        if total_requests > 100 {
            let hit_rate = (stats.hits as f64 / total_requests as f64) * 100.0;
            if hit_rate < 80.0 {
                warn!("Redis hit rate is low: {:.1}%", hit_rate);
            }
        }

        // Check for evicted keys (warn if > 0)
        if stats.evicted_keys > 0 {
            warn!(
                "Redis has evicted {} keys due to memory pressure",
                stats.evicted_keys
            );
        }

        // Check connection count (warn if > 50)
        if stats.connected_clients > 50 {
            warn!(
                "High number of Redis connections: {}",
                stats.connected_clients
            );
        }
    }

    /// Get current Redis monitoring information
    #[allow(dead_code)]
    pub async fn get_monitoring_info(
        &self,
    ) -> Result<crate::cache::RedisMonitoringInfo, Box<dyn std::error::Error + Send + Sync>> {
        self.cache_client
            .get_monitoring_info()
            .await
            .map_err(|e| e.into())
    }

    /// Get Redis keys matching a pattern (for debugging)
    #[allow(dead_code)]
    pub async fn get_keys(
        &self,
        pattern: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        self.cache_client
            .get_keys(pattern)
            .await
            .map_err(|e| e.into())
    }

    /// Get TTL for multiple keys
    #[allow(dead_code)]
    pub async fn get_ttls(
        &self,
        keys: &[&str],
    ) -> Result<Vec<i64>, Box<dyn std::error::Error + Send + Sync>> {
        self.cache_client
            .get_multiple_ttls(keys)
            .await
            .map_err(|e| e.into())
    }

    /// Perform a comprehensive Redis health check
    #[allow(dead_code)]
    pub async fn health_check(&self) -> RedisHealthStatus {
        match self.cache_client.health_check().await {
            true => {
                // Get additional metrics for detailed health check
                match self.cache_client.get_stats().await {
                    Ok(stats) => {
                        let mut issues = Vec::new();

                        // Check memory usage
                        if stats.used_memory_bytes > 200 * 1024 * 1024 {
                            issues.push("High memory usage".to_string());
                        }

                        // Check hit rate
                        let total_requests = stats.hits + stats.misses;
                        if total_requests > 100 {
                            let hit_rate = (stats.hits as f64 / total_requests as f64) * 100.0;
                            if hit_rate < 70.0 {
                                issues.push(format!("Low hit rate: {:.1}%", hit_rate));
                            }
                        }

                        // Check for evicted keys
                        if stats.evicted_keys > 0 {
                            issues.push(format!("{} keys evicted", stats.evicted_keys));
                        }

                        if issues.is_empty() {
                            RedisHealthStatus::Healthy
                        } else {
                            RedisHealthStatus::Degraded { issues }
                        }
                    }
                    Err(e) => {
                        error!("Failed to get Redis stats for health check: {}", e);
                        RedisHealthStatus::Unhealthy {
                            reason: format!("Stats error: {}", e),
                        }
                    }
                }
            }
            false => RedisHealthStatus::Unhealthy {
                reason: "Connection failed".to_string(),
            },
        }
    }
}

/// Redis health status
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum RedisHealthStatus {
    Healthy,
    Degraded { issues: Vec<String> },
    Unhealthy { reason: String },
}

impl RedisHealthStatus {
    #[allow(dead_code)]
    pub fn is_healthy(&self) -> bool {
        matches!(self, RedisHealthStatus::Healthy)
    }

    #[allow(dead_code)]
    pub fn status_string(&self) -> String {
        match self {
            RedisHealthStatus::Healthy => "healthy".to_string(),
            RedisHealthStatus::Degraded { issues } => format!("degraded: {}", issues.join(", ")),
            RedisHealthStatus::Unhealthy { reason } => format!("unhealthy: {}", reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_health_status() {
        let healthy = RedisHealthStatus::Healthy;
        assert!(healthy.is_healthy());
        assert_eq!(healthy.status_string(), "healthy");

        let degraded = RedisHealthStatus::Degraded {
            issues: vec!["High memory usage".to_string(), "Low hit rate".to_string()],
        };
        assert!(!degraded.is_healthy());
        assert!(degraded.status_string().contains("degraded"));

        let unhealthy = RedisHealthStatus::Unhealthy {
            reason: "Connection failed".to_string(),
        };
        assert!(!unhealthy.is_healthy());
        assert!(unhealthy.status_string().contains("unhealthy"));
    }
}
