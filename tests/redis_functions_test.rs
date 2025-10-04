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

/// Test Redis utility functions that were recently implemented
#[tokio::test]
async fn test_redis_utility_functions() -> AppResult<()> {
    // Skip if Redis is not available
    let config = create_test_redis_config();
    let client = match CacheClient::with_config(config).await {
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
    println!(
        "✓ GET_STATS operation: {} bytes used",
        stats.used_memory_bytes
    );

    // Test 10: HEALTH_CHECK operation
    assert!(client.health_check().await);
    println!("✓ HEALTH_CHECK operation");

    // Clean up
    let _ = client.delete_cache(counter_key).await;

    println!("All Redis utility functions test passed! 🎉");
    Ok(())
}
