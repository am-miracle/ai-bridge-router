use bridge_router::cache::{CacheClient, RedisConfig};
use bridge_router::utils::errors::AppResult;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

// Global mutex to prevent test interference
static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct TestData {
    id: u64,
    name: String,
    active: bool,
}

/// Test Redis utility functions that were recently implemented
#[tokio::test]
async fn test_redis_utility_functions() -> AppResult<()> {
    // Skip if Redis is not available
    let client = match CacheClient::new().await {
        Ok(client) => client,
        Err(_) => {
            println!("Skipping Redis utility functions test - Redis not available");
            return Ok(());
        }
    };

    let test_key = "test:utility_functions";
    let test_data = TestData {
        id: 456,
        name: "utility_test".to_string(),
        active: true,
    };

    // Clean up any existing data
    let _ = client.delete_cache(test_key).await;

    // Test 1: SET with TTL
    client.set_cache(test_key, &test_data, 60).await?;
    println!("✓ SET operation with TTL");

    // Test 2: GET operation
    let retrieved: Option<TestData> = client.get_cache(test_key).await?;
    assert_eq!(retrieved, Some(test_data.clone()));
    println!("✓ GET operation");

    // Test 3: EXISTS operation
    assert!(client.exists(test_key).await?);
    println!("✓ EXISTS operation");

    // Test 4: TTL operation
    let ttl = client.ttl(test_key).await?;
    assert!(ttl > 0 && ttl <= 60);
    println!("✓ TTL operation: {} seconds remaining", ttl);

    // Test 5: EXPIRE operation (extend TTL)
    assert!(client.expire(test_key, 120).await?);
    let new_ttl = client.ttl(test_key).await?;
    assert!(new_ttl > 60 && new_ttl <= 120);
    println!("✓ EXPIRE operation: extended to {} seconds", new_ttl);

    // Test 6: INCREMENT operation
    let counter_key = "test:counter";
    let _ = client.delete_cache(counter_key).await;

    let count1 = client.increment(counter_key, 1).await?;
    assert_eq!(count1, 1);

    let count2 = client.increment(counter_key, 5).await?;
    assert_eq!(count2, 6);
    println!("✓ INCREMENT operation: {} -> {}", count1, count2);

    // Test 7: GET_MULTIPLE operation
    let keys = vec![test_key, counter_key, "nonexistent_key"];
    let values = client.get_multiple::<serde_json::Value>(&keys).await?;
    assert_eq!(values.len(), 3);
    assert!(values[0].is_some()); // test_key exists
    assert!(values[1].is_some()); // counter_key exists
    assert!(values[2].is_none()); // nonexistent_key doesn't exist
    println!("✓ GET_MULTIPLE operation: {} keys processed", keys.len());

    // Test 8: DELETE operation
    assert!(client.delete_cache(test_key).await?);
    assert!(!client.exists(test_key).await?);
    println!("✓ DELETE operation");

    // Test 9: GET_STATS operation
    let stats = client.get_stats().await?;
    assert!(stats.config.pool_size > 0);
    println!(
        "✓ GET_STATS operation: {} bytes used, pool size {}",
        stats.used_memory_bytes, stats.config.pool_size
    );

    // Test 10: HEALTH_CHECK operation
    assert!(client.health_check().await);
    println!("✓ HEALTH_CHECK operation");

    // Clean up
    let _ = client.delete_cache(counter_key).await;

    println!("All Redis utility functions test passed! 🎉");
    Ok(())
}

/// Test Redis configuration validation
#[test]
fn test_redis_config_validation() {
    let _guard = TEST_MUTEX.lock().unwrap();
    // Test default configuration
    let default_config = RedisConfig::default();
    assert_eq!(default_config.pool_size, 10);
    assert_eq!(default_config.connection_timeout.as_secs(), 5);
    assert_eq!(default_config.command_timeout.as_secs(), 3);
    assert_eq!(default_config.url, "redis://localhost:6379");

    // Clean environment first
    unsafe {
        std::env::remove_var("REDIS_POOL_SIZE");
        std::env::remove_var("REDIS_CONNECTION_TIMEOUT");
        std::env::remove_var("REDIS_COMMAND_TIMEOUT");
    }

    // Test configuration loading from environment
    unsafe {
        std::env::set_var("REDIS_POOL_SIZE", "20");
        std::env::set_var("REDIS_CONNECTION_TIMEOUT", "10");
        std::env::set_var("REDIS_COMMAND_TIMEOUT", "5");
    }

    let config = RedisConfig::from_env().unwrap();
    assert_eq!(config.pool_size, 20);
    assert_eq!(config.connection_timeout.as_secs(), 10);
    assert_eq!(config.command_timeout.as_secs(), 5);

    // Clean up
    unsafe {
        std::env::remove_var("REDIS_POOL_SIZE");
        std::env::remove_var("REDIS_CONNECTION_TIMEOUT");
        std::env::remove_var("REDIS_COMMAND_TIMEOUT");
    }
}

/// Test invalid Redis configuration
#[test]
fn test_invalid_redis_config() {
    // Ensure clean environment first
    unsafe {
        std::env::remove_var("REDIS_POOL_SIZE");
        std::env::remove_var("REDIS_CONNECTION_TIMEOUT");
        std::env::remove_var("REDIS_COMMAND_TIMEOUT");
    }

    // Test invalid pool size
    unsafe {
        std::env::set_var("REDIS_POOL_SIZE", "invalid");
    }
    let result = RedisConfig::from_env();
    assert!(result.is_err());

    // Test invalid connection timeout
    unsafe {
        std::env::remove_var("REDIS_POOL_SIZE");
        std::env::set_var("REDIS_CONNECTION_TIMEOUT", "invalid");
    }
    let result = RedisConfig::from_env();
    assert!(result.is_err());

    // Test invalid command timeout
    unsafe {
        std::env::remove_var("REDIS_CONNECTION_TIMEOUT");
        std::env::set_var("REDIS_COMMAND_TIMEOUT", "invalid");
    }
    let result = RedisConfig::from_env();
    assert!(result.is_err());

    // Clean up
    unsafe {
        std::env::remove_var("REDIS_COMMAND_TIMEOUT");
    }
}
