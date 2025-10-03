use bridge_router::db::pool::{_get_pool_stats as get_pool_stats, check_connection, init_pg_pool};
use bridge_router::utils::errors::AppResult;
use std::sync::Mutex;

// Global mutex to prevent test interference
static TEST_MUTEX: Mutex<()> = Mutex::new(());

/// Test PostgreSQL connectivity
#[tokio::test]
async fn test_postgresql_connectivity() -> AppResult<()> {
    // This test requires a running PostgreSQL instance
    // Skip if DATABASE_URL is not properly configured
    match std::env::var("DATABASE_URL") {
        Ok(url) if !url.is_empty() && url != "postgres://localhost/bridge_router" => {
            let pool = init_pg_pool().await?;

            // Test basic connectivity
            assert!(check_connection(&pool).await);

            // Test pool statistics
            let stats = get_pool_stats(&pool).await?;
            assert!(stats.total_connections > 0);

            println!("PostgreSQL connectivity test passed");
            println!("Pool stats: {:?}", stats);

            Ok(())
        }
        _ => {
            println!("Skipping PostgreSQL test - DATABASE_URL not configured for testing");
            Ok(())
        }
    }
}

/// Test database configuration validation
#[test]
fn test_database_config_validation() {
    let _guard = TEST_MUTEX.lock().unwrap();
    use bridge_router::db::pool::DatabaseConfig;

    // Test default configuration
    let default_config = DatabaseConfig::default();
    assert_eq!(default_config.max_connections, 10);
    assert_eq!(default_config.min_connections, 1);
    assert_eq!(default_config.url, "postgres://localhost/bridge_router");

    // Store original values to restore later
    let original_max = std::env::var("DATABASE_MAX_CONNECTIONS").ok();
    let original_min = std::env::var("DATABASE_MIN_CONNECTIONS").ok();

    // Test configuration loading from environment
    unsafe {
        std::env::set_var("DATABASE_MAX_CONNECTIONS", "20");
        std::env::set_var("DATABASE_MIN_CONNECTIONS", "2");
    }

    let config = DatabaseConfig::from_env().unwrap();
    assert_eq!(config.max_connections, 20);
    assert_eq!(config.min_connections, 2);

    // Restore original values or remove if they didn't exist
    match original_max {
        Some(val) => unsafe { std::env::set_var("DATABASE_MAX_CONNECTIONS", val) },
        None => unsafe { std::env::remove_var("DATABASE_MAX_CONNECTIONS") },
    }
    match original_min {
        Some(val) => unsafe { std::env::set_var("DATABASE_MIN_CONNECTIONS", val) },
        None => unsafe { std::env::remove_var("DATABASE_MIN_CONNECTIONS") },
    }
}

/// Test invalid database configuration
#[test]
fn test_invalid_database_config() {
    let _guard = TEST_MUTEX.lock().unwrap();
    use bridge_router::db::pool::DatabaseConfig;

    // Ensure clean environment first
    unsafe {
        std::env::remove_var("DATABASE_MAX_CONNECTIONS");
        std::env::remove_var("DATABASE_MIN_CONNECTIONS");
    }

    // Test invalid max connections
    unsafe {
        std::env::set_var("DATABASE_MAX_CONNECTIONS", "invalid");
    }
    let result = DatabaseConfig::from_env();
    assert!(result.is_err());

    // Test invalid min connections
    unsafe {
        std::env::remove_var("DATABASE_MAX_CONNECTIONS");
        std::env::set_var("DATABASE_MIN_CONNECTIONS", "invalid");
    }
    let result = DatabaseConfig::from_env();
    assert!(result.is_err());

    // Clean up
    unsafe {
        std::env::remove_var("DATABASE_MIN_CONNECTIONS");
    }
}
