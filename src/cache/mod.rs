use crate::config::Settings;
use crate::utils::errors::{AppError, AppResult};
use deadpool_redis::redis::cmd;
use deadpool_redis::{Config, Pool, PoolError, Runtime};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, info, warn};

/// Redis configuration
#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
    pub connection_timeout: Duration,
    pub command_timeout: Duration,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "rediss://localhost:6379".to_string(),
            pool_size: 10,
            connection_timeout: Duration::from_secs(5),
            command_timeout: Duration::from_secs(3),
        }
    }
}

/// Redis cache client with connection pooling
#[derive(Clone)]
pub struct CacheClient {
    pool: Pool,
    config: RedisConfig,
}

impl CacheClient {
    /// Create a new cache client with Settings configuration
    pub async fn with_settings(settings: &Settings) -> AppResult<Self> {
        let redis_config = RedisConfig {
            url: settings.redis.url.clone(),
            pool_size: settings.redis.pool_size,
            connection_timeout: Duration::from_secs(settings.redis.connection_timeout_seconds),
            command_timeout: Duration::from_secs(settings.redis.command_timeout_seconds),
        };
        Self::with_config(redis_config).await
    }

    /// Create a new cache client with custom configuration
    pub async fn with_config(config: RedisConfig) -> AppResult<Self> {
        // Initialize rustls crypto provider for TLS support
        if config.url.starts_with("rediss://") {
            rustls::crypto::ring::default_provider()
                .install_default()
                .map_err(|e| {
                    AppError::config(format!("Failed to install rustls crypto provider: {:?}", e))
                })?;
        }

        // Create deadpool configuration
        let pool_config = Config::from_url(&config.url);

        // Create the connection pool
        let pool = pool_config
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| AppError::config(format!("Failed to create Redis pool: {}", e)))?;

        // Test the connection with timeout
        let mut test_conn = timeout(config.connection_timeout, pool.get())
            .await
            .map_err(|_| AppError::Timeout("Redis connection timeout".to_string()))?
            .map_err(|e| match e {
                PoolError::Backend(redis_err) => AppError::Redis(redis_err),
                PoolError::Timeout(_) => AppError::Timeout("Redis pool timeout".to_string()),
                PoolError::Closed => AppError::ServiceUnavailable("Redis pool closed".to_string()),
                PoolError::NoRuntimeSpecified => {
                    AppError::config("No runtime specified for Redis pool")
                }
                PoolError::PostCreateHook(_) => {
                    AppError::config("Redis pool post-create hook failed")
                }
            })?;

        // Test ping with command timeout
        let ping_result = timeout(
            config.command_timeout,
            cmd("PING").query_async::<String>(&mut test_conn),
        )
        .await
        .map_err(|_| AppError::Timeout("Redis ping timeout".to_string()))?
        .map_err(AppError::Redis)?;

        info!(
            "Redis connection pool established successfully (ping: {})",
            ping_result
        );

        Ok(Self { pool, config })
    }

    /// Get a Redis connection from the pool with timeout
    async fn get_connection(&self) -> AppResult<deadpool_redis::Connection> {
        timeout(self.config.connection_timeout, self.pool.get())
            .await
            .map_err(|_| AppError::Timeout("Redis connection timeout".to_string()))?
            .map_err(|e| match e {
                PoolError::Backend(redis_err) => AppError::Redis(redis_err),
                PoolError::Timeout(_) => AppError::Timeout("Redis pool timeout".to_string()),
                PoolError::Closed => AppError::ServiceUnavailable("Redis pool closed".to_string()),
                PoolError::NoRuntimeSpecified => {
                    AppError::config("No runtime specified for Redis pool")
                }
                PoolError::PostCreateHook(_) => {
                    AppError::config("Redis pool post-create hook failed")
                }
            })
    }

    /// Set a value in the cache with TTL
    pub async fn set_cache<T>(&self, key: &str, value: &T, ttl_seconds: u64) -> AppResult<()>
    where
        T: Serialize,
    {
        let serialized = serde_json::to_string(value)?;
        let mut conn = self.get_connection().await?;

        // Apply command timeout to Redis operations
        let set_result = if ttl_seconds > 0 {
            timeout(
                self.config.command_timeout,
                cmd("SETEX")
                    .arg(&[key, &ttl_seconds.to_string(), &serialized])
                    .query_async::<()>(&mut conn),
            )
            .await
        } else {
            timeout(
                self.config.command_timeout,
                cmd("SET")
                    .arg(&[key, &serialized])
                    .query_async::<()>(&mut conn),
            )
            .await
        };

        set_result
            .map_err(|_| AppError::Timeout("Redis SET command timeout".to_string()))?
            .map_err(AppError::Redis)?;

        tracing::debug!("Cache SET: {} (TTL: {}s)", key, ttl_seconds);
        Ok(())
    }

    /// Get a value from the cache
    pub async fn get_cache<T>(&self, key: &str) -> AppResult<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut conn = self.get_connection().await?;

        // Apply command timeout to Redis GET operation
        let get_result = timeout(
            self.config.command_timeout,
            cmd("GET")
                .arg(&[key])
                .query_async::<Option<String>>(&mut conn),
        )
        .await;

        match get_result {
            Ok(Ok(Some(value))) => {
                tracing::debug!("Cache HIT: {}", key);
                let deserialized = serde_json::from_str(&value)?;
                Ok(Some(deserialized))
            }
            Ok(Ok(None)) => {
                tracing::debug!("Cache MISS: {}", key);
                Ok(None)
            }
            Ok(Err(e)) => {
                error!("Cache GET error for key '{}': {}", key, e);
                Err(AppError::Redis(e))
            }
            Err(_) => {
                error!("Cache GET timeout for key '{}'", key);
                Err(AppError::Timeout("Redis GET command timeout".to_string()))
            }
        }
    }

    #[allow(dead_code)]
    /// Delete a value from the cache
    pub async fn delete_cache(&self, key: &str) -> AppResult<bool> {
        let mut conn = self.get_connection().await?;

        let delete_result = timeout(
            self.config.command_timeout,
            cmd("DEL").arg(&[key]).query_async::<u64>(&mut conn),
        )
        .await;

        let deleted = delete_result
            .map_err(|_| AppError::Timeout("Redis DELETE command timeout".to_string()))?
            .map_err(AppError::Redis)?;

        let was_deleted = deleted > 0;
        tracing::debug!("Cache DELETE: {} (existed: {})", key, was_deleted);
        Ok(was_deleted)
    }

    #[allow(dead_code)]
    /// Check if a key exists in the cache
    pub async fn exists(&self, key: &str) -> AppResult<bool> {
        let mut conn = self.get_connection().await?;

        let exists_result = timeout(
            self.config.command_timeout,
            cmd("EXISTS").arg(&[key]).query_async::<u64>(&mut conn),
        )
        .await;

        let exists = exists_result
            .map_err(|_| AppError::Timeout("Redis EXISTS command timeout".to_string()))?
            .map_err(AppError::Redis)?;

        Ok(exists > 0)
    }

    #[allow(dead_code)]
    /// Set TTL for an existing key
    pub async fn expire(&self, key: &str, ttl_seconds: u64) -> AppResult<bool> {
        let mut conn = self.get_connection().await?;

        let expire_result = timeout(
            self.config.command_timeout,
            cmd("EXPIRE")
                .arg(&[key, &ttl_seconds.to_string()])
                .query_async::<u64>(&mut conn),
        )
        .await;

        let result = expire_result
            .map_err(|_| AppError::Timeout("Redis EXPIRE command timeout".to_string()))?
            .map_err(AppError::Redis)?;

        Ok(result > 0)
    }

    #[allow(dead_code)]
    /// Get TTL for a key
    pub async fn ttl(&self, key: &str) -> AppResult<i64> {
        let mut conn = self.get_connection().await?;

        let ttl_result = timeout(
            self.config.command_timeout,
            cmd("TTL").arg(&[key]).query_async::<i64>(&mut conn),
        )
        .await;

        let ttl = ttl_result
            .map_err(|_| AppError::Timeout("Redis TTL command timeout".to_string()))?
            .map_err(AppError::Redis)?;

        Ok(ttl)
    }

    #[allow(dead_code)]
    /// Increment a numeric value in the cache
    pub async fn increment(&self, key: &str, delta: i64) -> AppResult<i64> {
        let mut conn = self.get_connection().await?;

        let incr_result = timeout(
            self.config.command_timeout,
            cmd("INCRBY")
                .arg(&[key, &delta.to_string()])
                .query_async::<i64>(&mut conn),
        )
        .await;

        let result = incr_result
            .map_err(|_| AppError::Timeout("Redis INCR command timeout".to_string()))?
            .map_err(AppError::Redis)?;

        Ok(result)
    }

    #[allow(dead_code)]
    /// Get multiple values at once
    pub async fn get_multiple<T>(&self, keys: &[&str]) -> AppResult<Vec<Option<T>>>
    where
        T: for<'de> Deserialize<'de>,
    {
        if keys.is_empty() {
            return Ok(vec![]);
        }

        let mut conn = self.get_connection().await?;

        let get_result = timeout(
            self.config.command_timeout,
            cmd("MGET")
                .arg(keys)
                .query_async::<Vec<Option<String>>>(&mut conn),
        )
        .await;

        let values = get_result
            .map_err(|_| AppError::Timeout("Redis MGET command timeout".to_string()))?
            .map_err(AppError::Redis)?;

        let mut results = Vec::with_capacity(values.len());
        for (i, value) in values.into_iter().enumerate() {
            match value {
                Some(v) => match serde_json::from_str(&v) {
                    Ok(deserialized) => {
                        tracing::debug!("Cache HIT: {}", keys[i]);
                        results.push(Some(deserialized));
                    }
                    Err(e) => {
                        warn!(
                            "Failed to deserialize cached value for key '{}': {}",
                            keys[i], e
                        );
                        results.push(None);
                    }
                },
                None => {
                    tracing::debug!("Cache MISS: {}", keys[i]);
                    results.push(None);
                }
            }
        }

        Ok(results)
    }

    #[allow(dead_code)]
    /// Clear all cache entries (use with caution)
    pub async fn clear_all(&self) -> AppResult<()> {
        let mut conn = self.get_connection().await?;

        let flush_result = timeout(
            self.config.command_timeout,
            cmd("FLUSHDB").query_async::<()>(&mut conn),
        )
        .await;

        flush_result
            .map_err(|_| AppError::Timeout("Redis FLUSHDB command timeout".to_string()))?
            .map_err(AppError::Redis)?;

        warn!("All cache entries cleared");
        Ok(())
    }

    /// Get comprehensive cache statistics using Redis INFO command
    pub async fn get_stats(&self) -> AppResult<CacheStats> {
        let mut conn = self.get_connection().await?;

        let info_result = timeout(
            self.config.command_timeout,
            cmd("INFO").arg("all").query_async::<String>(&mut conn),
        )
        .await;

        let info = info_result
            .map_err(|_| AppError::Timeout("Redis INFO command timeout".to_string()))?
            .map_err(AppError::Redis)?;

        let mut stats = CacheStats {
            used_memory_bytes: 0,
            total_keys: 0,
            connected_clients: 0,
            uptime_seconds: 0,
            hits: 0,
            misses: 0,
            evicted_keys: 0,
        };

        // Parse Redis INFO output
        for line in info.lines() {
            if line.starts_with("used_memory:") {
                if let Some(value) = line.split(':').nth(1) {
                    stats.used_memory_bytes = value.parse().unwrap_or(0);
                }
            } else if line.starts_with("db0:keys=") {
                if let Some(value) = line.split('=').nth(1) {
                    stats.total_keys = value.parse().unwrap_or(0);
                }
            } else if line.starts_with("connected_clients:") {
                if let Some(value) = line.split(':').nth(1) {
                    stats.connected_clients = value.parse().unwrap_or(0);
                }
            } else if line.starts_with("uptime_in_seconds:") {
                if let Some(value) = line.split(':').nth(1) {
                    stats.uptime_seconds = value.parse().unwrap_or(0);
                }
            } else if line.starts_with("keyspace_hits:") {
                if let Some(value) = line.split(':').nth(1) {
                    stats.hits = value.parse().unwrap_or(0);
                }
            } else if line.starts_with("keyspace_misses:") {
                if let Some(value) = line.split(':').nth(1) {
                    stats.misses = value.parse().unwrap_or(0);
                }
            } else if line.starts_with("evicted_keys:")
                && let Some(value) = line.split(':').nth(1)
            {
                stats.evicted_keys = value.parse().unwrap_or(0);
            }
        }

        Ok(stats)
    }

    #[allow(dead_code)]
    /// Get Redis monitoring information (keys, memory usage, etc.)
    pub async fn get_monitoring_info(&self) -> AppResult<RedisMonitoringInfo> {
        let mut conn = self.get_connection().await?;

        // Get database size
        let db_size = timeout(
            self.config.command_timeout,
            cmd("DBSIZE").query_async::<u64>(&mut conn),
        )
        .await
        .map_err(|_| AppError::Timeout("Redis DBSIZE command timeout".to_string()))?
        .map_err(AppError::Redis)?;

        // Get memory usage
        let memory_usage = timeout(
            self.config.command_timeout,
            cmd("MEMORY")
                .arg("USAGE")
                .arg("")
                .query_async::<i64>(&mut conn),
        )
        .await
        .map_err(|_| AppError::Timeout("Redis MEMORY USAGE command timeout".to_string()))?
        .map_err(AppError::Redis)?;

        // Get slow log (last 10 entries) - simplified to just get count
        let slow_log_count = timeout(
            self.config.command_timeout,
            cmd("SLOWLOG").arg("LEN").query_async::<u64>(&mut conn),
        )
        .await
        .map_err(|_| AppError::Timeout("Redis SLOWLOG command timeout".to_string()))?
        .map_err(AppError::Redis)?;

        Ok(RedisMonitoringInfo {
            db_size,
            memory_usage_bytes: memory_usage.max(0) as u64,
            slow_log_entries: slow_log_count as usize,
            pool_size: self.config.pool_size,
            connection_timeout: self.config.connection_timeout,
            command_timeout: self.config.command_timeout,
        })
    }

    #[allow(dead_code)]
    /// Get keys matching a pattern (for debugging/monitoring)
    pub async fn get_keys(&self, pattern: &str) -> AppResult<Vec<String>> {
        let mut conn = self.get_connection().await?;

        let keys = timeout(
            self.config.command_timeout,
            cmd("KEYS")
                .arg(pattern)
                .query_async::<Vec<String>>(&mut conn),
        )
        .await
        .map_err(|_| AppError::Timeout("Redis KEYS command timeout".to_string()))?
        .map_err(AppError::Redis)?;

        Ok(keys)
    }

    /// Get TTL for multiple keys at once
    pub async fn get_multiple_ttls(&self, keys: &[&str]) -> AppResult<Vec<i64>> {
        if keys.is_empty() {
            return Ok(vec![]);
        }

        let mut conn = self.get_connection().await?;
        let mut ttls = Vec::with_capacity(keys.len());

        for key in keys {
            let ttl = timeout(
                self.config.command_timeout,
                cmd("TTL").arg(key).query_async::<i64>(&mut conn),
            )
            .await
            .map_err(|_| AppError::Timeout("Redis TTL command timeout".to_string()))?
            .map_err(AppError::Redis)?;

            ttls.push(ttl);
        }

        Ok(ttls)
    }

    /// Health check for Redis connection
    pub async fn health_check(&self) -> bool {
        match self.get_connection().await {
            Ok(mut conn) => {
                let ping_result = timeout(
                    self.config.command_timeout,
                    cmd("PING").query_async::<String>(&mut conn),
                )
                .await;

                match ping_result {
                    Ok(Ok(_)) => true,
                    Ok(Err(e)) => {
                        error!("Redis health check failed: {}", e);
                        false
                    }
                    Err(_) => {
                        error!("Redis health check timeout");
                        false
                    }
                }
            }
            Err(e) => {
                error!("Redis connection failed: {}", e);
                false
            }
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub used_memory_bytes: u64,
    pub total_keys: u64,
    pub connected_clients: u64,
    pub uptime_seconds: u64,
    pub hits: u64,
    pub misses: u64,
    pub evicted_keys: u64,
}

/// Redis monitoring information
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RedisMonitoringInfo {
    pub db_size: u64,
    pub memory_usage_bytes: u64,
    pub slow_log_entries: usize,
    pub pool_size: u32,
    pub connection_timeout: Duration,
    pub command_timeout: Duration,
}
