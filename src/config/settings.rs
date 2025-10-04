use std::vec;

use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigValidationError {
    #[error("Invalid configuration: {message}")]
    ValidationError { message: String },
    #[error("Configuration loading error: {source}")]
    ConfigError { source: ConfigError },
}

impl From<ConfigError> for ConfigValidationError {
    fn from(err: ConfigError) -> Self {
        ConfigValidationError::ConfigError { source: err }
    }
}

/// Application configuration with validation and layered loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: SettingsRedisConfig,
    pub cors: CorsConfig,
    pub bridges: BridgeConfig,
    pub cache: CacheConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub shutdown_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsRedisConfig {
    pub url: String,
    pub pool_size: u32,
    pub connection_timeout_seconds: u64,
    pub command_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub allow_credentials: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub timeout_seconds: u64,
    pub retries: u32,
    pub hop_network: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub quote_ttl_seconds: u64,
    pub stale_ttl_seconds: u64,
    pub rate_limit_per_minute: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

impl Settings {
    /// Load configuration from multiple sources with layered priority:
    /// 1. Default values (for non-sensitive config)
    /// 2. config.toml file (if exists)
    /// 3. Environment variables (both direct names and BRIDGE_ROUTER_ prefix)
    ///
    /// Sensitive values will be loaded from environment variables only
    pub fn new() -> Result<Self, ConfigValidationError> {
        let mut config_builder = Config::builder();

        // Layer 1: Set defaults for non-sensitive values
        config_builder = config_builder
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("server.shutdown_timeout_seconds", 30)?
            .set_default("database.url", "")? // Will be loaded from env
            .set_default("database.max_connections", 10)?
            .set_default("database.min_connections", 1)?
            .set_default("database.connect_timeout_seconds", 30)?
            .set_default("database.idle_timeout_seconds", 600)?
            .set_default("redis.url", "")?
            .set_default("redis.pool_size", 10)?
            .set_default("redis.connection_timeout_seconds", 5)?
            .set_default("redis.command_timeout_seconds", 3)?
            .set_default("cors.allowed_origins", vec!["http://localhost:3000"])?
            .set_default(
                "cors.allowed_methods",
                vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"],
            )?
            .set_default(
                "cors.allowed_headers",
                vec!["Content-Type", "Authorization", "Accept"],
            )?
            .set_default("cors.allow_credentials", true)?
            .set_default("bridges.timeout_seconds", 5)?
            .set_default("bridges.retries", 1)?
            .set_default("bridges.hop_network", "mainnet")?
            .set_default("cache.quote_ttl_seconds", 15)?
            .set_default("cache.stale_ttl_seconds", 300)?
            .set_default("cache.rate_limit_per_minute", 100)?
            .set_default("logging.level", "info")?
            .set_default("logging.format", "pretty")?;

        // Layer 2: Load from config file (optional)
        if std::path::Path::new("config.toml").exists() {
            config_builder = config_builder.add_source(File::with_name("config"));
        }

        // Layer 3: Load from environment variables (common direct names)
        let env_vars = [
            ("HOST", "server.host"),
            ("PORT", "server.port"),
            ("DATABASE_URL", "database.url"),
            ("DATABASE_MAX_CONNECTIONS", "database.max_connections"),
            ("DATABASE_MIN_CONNECTIONS", "database.min_connections"),
            ("REDIS_URL", "redis.url"),
            ("REDIS_POOL_SIZE", "redis.pool_size"),
            (
                "REDIS_CONNECTION_TIMEOUT",
                "redis.connection_timeout_seconds",
            ),
            ("REDIS_COMMAND_TIMEOUT", "redis.command_timeout_seconds"),
            ("ALLOWED_ORIGINS", "cors.allowed_origins"),
            ("HOP_NETWORK", "bridges.hop_network"),
            ("RUST_LOG", "logging.level"),
            ("RUST_LOG_FORMAT", "logging.format"),
        ];

        for (env_key, config_key) in env_vars {
            // Special-case ALLOWED_ORIGINS to parse as a list
            if env_key == "ALLOWED_ORIGINS" {
                if let Ok(val) = std::env::var("ALLOWED_ORIGINS") {
                    let origins: Vec<String> = val
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    config_builder = config_builder.set_override(config_key, origins)?;
                }
                continue;
            }

            if let Ok(value) = std::env::var(env_key) {
                config_builder = config_builder.set_override(config_key, value)?;
            }
        }

        // Layer 4: Load from environment variables (BRIDGE_ROUTER_ prefix)
        config_builder = config_builder.add_source(
            Environment::with_prefix("BRIDGE_ROUTER")
                .separator("_")
                .try_parsing(true),
        );

        let config = config_builder.build()?;
        let settings: Settings = config.try_deserialize()?;

        // Validate the loaded configuration (will check for required env vars)
        settings.validate()?;

        Ok(settings)
    }

    /// Validate all configuration values
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        // Server validation
        if self.server.port == 0 {
            return Err(ConfigValidationError::ValidationError {
                message: "Server port cannot be 0".to_string(),
            });
        }

        if self.server.host.is_empty() {
            return Err(ConfigValidationError::ValidationError {
                message: "Server host cannot be empty".to_string(),
            });
        }

        // Database validation
        if self.database.url.is_empty() {
            return Err(ConfigValidationError::ValidationError {
                message: "DATABASE_URL environment variable is required".to_string(),
            });
        }

        if !self.database.url.starts_with("postgres://")
            && !self.database.url.starts_with("postgresql://")
        {
            return Err(ConfigValidationError::ValidationError {
                message: "Database URL must be a valid PostgreSQL connection string".to_string(),
            });
        }

        if self.database.max_connections == 0 {
            return Err(ConfigValidationError::ValidationError {
                message: "Database max_connections must be greater than 0".to_string(),
            });
        }

        if self.database.min_connections > self.database.max_connections {
            return Err(ConfigValidationError::ValidationError {
                message: "Database min_connections cannot be greater than max_connections"
                    .to_string(),
            });
        }

        // Redis validation
        if self.redis.url.is_empty() {
            return Err(ConfigValidationError::ValidationError {
                message: "REDIS_URL environment variable is required".to_string(),
            });
        }

        if !self.redis.url.starts_with("redis://") && !self.redis.url.starts_with("rediss://") {
            return Err(ConfigValidationError::ValidationError {
                message: "Redis URL must start with redis:// or rediss://".to_string(),
            });
        }

        if self.redis.pool_size == 0 {
            return Err(ConfigValidationError::ValidationError {
                message: "Redis pool_size must be greater than 0".to_string(),
            });
        }

        // CORS validation
        if self.cors.allowed_origins.is_empty() {
            return Err(ConfigValidationError::ValidationError {
                message: "At least one CORS allowed origin must be specified".to_string(),
            });
        }

        // Bridge validation
        if self.bridges.timeout_seconds == 0 {
            return Err(ConfigValidationError::ValidationError {
                message: "Bridge timeout must be greater than 0".to_string(),
            });
        }

        if !["mainnet", "goerli", "testnet"].contains(&self.bridges.hop_network.as_str()) {
            return Err(ConfigValidationError::ValidationError {
                message: "Hop network must be 'mainnet', 'goerli', or 'testnet'".to_string(),
            });
        }

        // Cache validation
        if self.cache.quote_ttl_seconds == 0 {
            return Err(ConfigValidationError::ValidationError {
                message: "Quote TTL must be greater than 0".to_string(),
            });
        }

        if self.cache.stale_ttl_seconds <= self.cache.quote_ttl_seconds {
            return Err(ConfigValidationError::ValidationError {
                message: "Stale TTL must be greater than quote TTL".to_string(),
            });
        }

        // Logging validation
        if !["error", "warn", "info", "debug", "trace"].contains(&self.logging.level.as_str()) {
            return Err(ConfigValidationError::ValidationError {
                message: "Log level must be one of: error, warn, info, debug, trace".to_string(),
            });
        }

        if !["json", "pretty"].contains(&self.logging.format.as_str()) {
            return Err(ConfigValidationError::ValidationError {
                message: "Log format must be 'json' or 'pretty'".to_string(),
            });
        }

        Ok(())
    }

    /// Get server bind address
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new().expect("Failed to load default configuration")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_settings() -> Result<Settings, ConfigValidationError> {
        let mut config_builder = Config::builder();

        // Set test defaults
        config_builder = config_builder
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("server.shutdown_timeout_seconds", 30)?
            .set_default("database.url", "postgresql://test@localhost/test")?
            .set_default("database.max_connections", 10)?
            .set_default("database.min_connections", 1)?
            .set_default("database.connect_timeout_seconds", 30)?
            .set_default("database.idle_timeout_seconds", 600)?
            .set_default("redis.url", "redis://localhost:6379")?
            .set_default("redis.pool_size", 10)?
            .set_default("redis.connection_timeout_seconds", 5)?
            .set_default("redis.command_timeout_seconds", 3)?
            .set_default("cors.allowed_origins", vec!["http://localhost:3000"])?
            .set_default(
                "cors.allowed_methods",
                vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"],
            )?
            .set_default(
                "cors.allowed_headers",
                vec!["Content-Type", "Authorization", "Accept"],
            )?
            .set_default("cors.allow_credentials", true)?
            .set_default("bridges.timeout_seconds", 5)?
            .set_default("bridges.retries", 1)?
            .set_default("bridges.hop_network", "mainnet")?
            .set_default("cache.quote_ttl_seconds", 15)?
            .set_default("cache.stale_ttl_seconds", 300)?
            .set_default("cache.rate_limit_per_minute", 100)?
            .set_default("logging.level", "info")?
            .set_default("logging.format", "pretty")?;

        let config = config_builder.build()?;
        let settings: Settings = config.try_deserialize()?;
        settings.validate()?;
        Ok(settings)
    }

    #[test]
    fn test_settings_default_values() {
        let settings = create_test_settings().unwrap();

        // Check defaults
        assert_eq!(settings.server.host, "0.0.0.0");
        assert_eq!(settings.server.port, 8080);
        assert_eq!(settings.server.shutdown_timeout_seconds, 30);
        assert_eq!(settings.database.max_connections, 10);
        assert_eq!(settings.database.min_connections, 1);
        assert_eq!(settings.redis.pool_size, 10);
        assert_eq!(settings.bridges.hop_network, "mainnet");
        assert_eq!(settings.cache.quote_ttl_seconds, 15);
        assert_eq!(settings.logging.level, "info");
        assert_eq!(settings.logging.format, "pretty");
    }

    #[test]
    fn test_server_address() {
        let settings = create_test_settings().unwrap();
        assert_eq!(settings.server_address(), "0.0.0.0:8080");
    }

    #[test]
    fn test_settings_validation_works() {
        // Test that validation passes with valid settings
        let settings = create_test_settings();
        assert!(settings.is_ok());
    }

    #[test]
    fn test_allowed_origins_env_var_parsing() -> Result<(), Box<dyn std::error::Error>> {
        // Set environment variable
        unsafe {
            std::env::set_var(
                "ALLOWED_ORIGINS",
                "http://localhost:3000,http://0.0.0.0:3000,https://example.com",
            );
        }

        // Create config with the env var
        let mut config_builder = Config::builder();

        // Set minimal defaults for test
        config_builder = config_builder
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("server.shutdown_timeout_seconds", 30)?
            .set_default("database.url", "postgresql://test@localhost/test")?
            .set_default("database.max_connections", 10)?
            .set_default("database.min_connections", 1)?
            .set_default("database.connect_timeout_seconds", 30)?
            .set_default("database.idle_timeout_seconds", 600)?
            .set_default("redis.url", "redis://localhost:6379")?
            .set_default("redis.pool_size", 10)?
            .set_default("redis.connection_timeout_seconds", 5)?
            .set_default("redis.command_timeout_seconds", 3)?
            .set_default("cors.allowed_origins", vec!["http://localhost:3000"])?
            .set_default(
                "cors.allowed_methods",
                vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"],
            )?
            .set_default(
                "cors.allowed_headers",
                vec!["Content-Type", "Authorization", "Accept"],
            )?
            .set_default("cors.allow_credentials", true)?
            .set_default("bridges.timeout_seconds", 5)?
            .set_default("bridges.retries", 1)?
            .set_default("bridges.hop_network", "mainnet")?
            .set_default("cache.quote_ttl_seconds", 15)?
            .set_default("cache.stale_ttl_seconds", 300)?
            .set_default("cache.rate_limit_per_minute", 100)?
            .set_default("logging.level", "info")?
            .set_default("logging.format", "pretty")?;

        // Apply environment variable parsing logic
        if let Ok(val) = std::env::var("ALLOWED_ORIGINS") {
            let origins: Vec<String> = val
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            config_builder = config_builder
                .set_override("cors.allowed_origins", origins)
                .unwrap();
        }

        let config = config_builder.build().unwrap();
        let settings: Settings = config.try_deserialize().unwrap();

        // Verify the parsing worked
        assert_eq!(settings.cors.allowed_origins.len(), 3);
        assert_eq!(settings.cors.allowed_origins[0], "http://localhost:3000");
        assert_eq!(settings.cors.allowed_origins[1], "http://0.0.0.0:3000");
        assert_eq!(settings.cors.allowed_origins[2], "https://example.com");

        // Clean up
        unsafe {
            std::env::remove_var("ALLOWED_ORIGINS");
        }

        Ok(())
    }
}
