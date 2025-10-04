use bridge_router::cache::{CacheClient, RedisConfig};
use bridge_router::utils::errors::AppResult;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct TestData {
    id: u64,
    name: String,
    active: bool,
}

/// Create a minimal Redis configuration for testing
fn create_test_redis_config() -> RedisConfig {
    RedisConfig {
        url: std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        pool_size: 5,
        connection_timeout: std::time::Duration::from_secs(5),
        command_timeout: std::time::Duration::from_secs(3),
    }
}

/// Test Redis connectivity and basic operations
#[tokio::test]
async fn test_redis_connectivity() -> AppResult<()> {
    // This test requires a running Redis instance
    // Skip if REDIS_URL is not properly configured
    match std::env::var("REDIS_URL") {
        Ok(url) if !url.is_empty() && url != "redis://localhost:6379" => {
            let config = create_test_redis_config();
            let client = CacheClient::with_config(config).await?;

            // Test health check
            assert!(client.health_check().await);

            println!("Redis connectivity test passed");
            Ok(())
        }
        _ => {
            println!("Skipping Redis test - REDIS_URL not configured for testing");
            Ok(())
        }
    }
}

/// Test Redis cache operations (set, get, delete)
#[tokio::test]
async fn test_redis_cache_operations() -> AppResult<()> {
    // Skip if Redis is not available
    let config = create_test_redis_config();
    let client = match CacheClient::with_config(config).await {
        Ok(client) => client,
        Err(_) => {
            println!("Skipping cache operations test - Redis not available");
            return Ok(());
        }
    };

    let test_key = "test:cache_operations";
    let test_data = TestData {
        id: 123,
        name: "test_item".to_string(),
        active: true,
    };

    // Clean up any existing data
    let _ = client.delete_cache(test_key).await;

    // Test SET operation
    client.set_cache(test_key, &test_data, 60).await?;

    // Test GET operation
    let retrieved: Option<TestData> = client.get_cache(test_key).await?;
    assert_eq!(retrieved, Some(test_data.clone()));

    // Test EXISTS operation
    assert!(client.exists(test_key).await?);

    // Test TTL operation
    let ttl = client.ttl(test_key).await?;
    assert!(ttl > 0 && ttl <= 60);

    // Test DELETE operation
    assert!(client.delete_cache(test_key).await?);

    // Verify deletion
    let retrieved_after_delete: Option<TestData> = client.get_cache(test_key).await?;
    assert_eq!(retrieved_after_delete, None);

    assert!(!client.exists(test_key).await?);

    println!("Redis cache operations test passed");
    Ok(())
}

/// Test Redis cache with TTL expiration
#[tokio::test]
async fn test_redis_ttl_expiration() -> AppResult<()> {
    let config = create_test_redis_config();
    let client = match CacheClient::with_config(config).await {
        Ok(client) => client,
        Err(_) => {
            println!("Skipping TTL expiration test - Redis not available");
            return Ok(());
        }
    };

    let test_key = "test:ttl_expiration";
    let test_value = "temporary_value";

    // Clean up
    let _ = client.delete_cache(test_key).await;

    // Set with very short TTL (1 second)
    client.set_cache(test_key, &test_value, 1).await?;

    // Should exist immediately
    assert!(client.exists(test_key).await?);

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Should no longer exist
    assert!(!client.exists(test_key).await?);

    println!("Redis TTL expiration test passed");
    Ok(())
}

/// Test Redis multiple operations
#[tokio::test]
async fn test_redis_multiple_operations() -> AppResult<()> {
    let config = create_test_redis_config();
    let client = match CacheClient::with_config(config).await {
        Ok(client) => client,
        Err(_) => {
            println!("Skipping multiple operations test - Redis not available");
            return Ok(());
        }
    };

    let keys = ["test:multi1", "test:multi2", "test:multi3"];
    let values = ["value1", "value2", "value3"];

    // Clean up
    for key in &keys {
        let _ = client.delete_cache(key).await;
    }

    // Set multiple values
    for (key, value) in keys.iter().zip(values.iter()) {
        client.set_cache(key, value, 300).await?;
    }

    // Get multiple values
    let retrieved: Vec<Option<String>> = client.get_multiple(&keys).await?;

    for (i, value) in retrieved.iter().enumerate() {
        assert_eq!(value, &Some(values[i].to_string()));
    }

    // Clean up
    for key in &keys {
        client.delete_cache(key).await?;
    }

    println!("Redis multiple operations test passed");
    Ok(())
}

/// Test Redis increment operation
#[tokio::test]
async fn test_redis_increment() -> AppResult<()> {
    let config = create_test_redis_config();
    let client = match CacheClient::with_config(config).await {
        Ok(client) => client,
        Err(_) => {
            println!("Skipping increment test - Redis not available");
            return Ok(());
        }
    };

    let test_key = "test:increment";

    // Clean up
    let _ = client.delete_cache(test_key).await;

    // Test increment from 0
    let result1 = client.increment(test_key, 1).await?;
    assert_eq!(result1, 1);

    // Test increment by 5
    let result2 = client.increment(test_key, 5).await?;
    assert_eq!(result2, 6);

    // Test decrement
    let result3 = client.increment(test_key, -2).await?;
    assert_eq!(result3, 4);

    // Clean up
    client.delete_cache(test_key).await?;

    println!("Redis increment test passed");
    Ok(())
}
