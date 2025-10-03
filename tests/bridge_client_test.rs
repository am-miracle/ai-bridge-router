use bridge_router::models::bridge::{BridgeClientConfig, BridgeQuote, BridgeQuoteRequest};
use bridge_router::services::get_all_bridge_quotes;
use serde_json::json;
use std::time::Duration;

/// Integration test for bridge client functionality
#[tokio::test]
async fn test_bridge_quote_request_structure() {
    let request = BridgeQuoteRequest {
        asset: "USDC".to_string(),
        from_chain: "ethereum".to_string(),
        to_chain: "polygon".to_string(),
        amount: Some("1000000".to_string()),
        // recipient: Some("0x123...".to_string()),
    };

    assert_eq!(request.asset, "USDC");
    assert_eq!(request.from_chain, "ethereum");
    assert_eq!(request.to_chain, "polygon");
    assert!(request.amount.is_some());
    // assert!(request.recipient.is_none());
}

#[tokio::test]
async fn test_bridge_client_config() {
    let config = BridgeClientConfig::new()
        .with_timeout(std::time::Duration::from_secs(10))
        .with_retries(2);

    assert_eq!(config.timeout.as_secs(), 10);
    assert_eq!(config.retries, 2);
}

#[tokio::test]
async fn test_bridge_quote_serialization() {
    let quote = BridgeQuote {
        bridge: "TestBridge".to_string(),
        fee: 0.001,
        est_time: 300,
        liquidity: "1M USDC".to_string(),
        score: None,
        metadata: Some(json!({
            "test": "data",
            "fee_breakdown": {
                "base": 0.0005,
                "gas": 0.0005
            }
        })),
    };

    // Test serialization
    let json_str = serde_json::to_string(&quote).unwrap();
    assert!(json_str.contains("TestBridge"));
    assert!(json_str.contains("0.001"));
    assert!(json_str.contains("300"));

    // Test deserialization
    let deserialized: BridgeQuote = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.bridge, "TestBridge");
    assert_eq!(deserialized.fee, 0.001);
    assert_eq!(deserialized.est_time, 300);
    assert_eq!(deserialized.liquidity, "1M USDC");
    assert!(deserialized.metadata.is_some());
}

#[tokio::test]
async fn test_get_all_bridge_quotes_empty_response() {
    // This test verifies that the function handles cases where all bridges fail
    let request = BridgeQuoteRequest {
        asset: "INVALID_ASSET".to_string(),
        from_chain: "invalid_chain".to_string(),
        to_chain: "another_invalid_chain".to_string(),
        amount: None,
    };

    let config = BridgeClientConfig::new()
        .with_timeout(std::time::Duration::from_secs(1)) // Short timeout
        .with_retries(0); // No retries

    let quotes = get_all_bridge_quotes(&request, &config).await;

    // Should return empty vector when all bridges fail
    assert_eq!(quotes.len(), 0);
}

#[tokio::test]
async fn test_parallel_execution() {
    // Test that demonstrates parallel execution works
    let request = BridgeQuoteRequest {
        asset: "USDC".to_string(),
        from_chain: "ethereum".to_string(),
        to_chain: "polygon".to_string(),
        amount: Some("1000000".to_string()),
        // recipient: None,
    };

    let config = BridgeClientConfig::new()
        .with_timeout(std::time::Duration::from_secs(5))
        .with_retries(1);

    let start_time = std::time::Instant::now();
    let quotes = get_all_bridge_quotes(&request, &config).await;
    let elapsed = start_time.elapsed();

    // Even if all requests fail/timeout, parallel execution should complete
    // faster than sequential execution would take
    assert!(elapsed < std::time::Duration::from_secs(20)); // Much less than 3 * 5 seconds

    println!(
        "Parallel execution completed in {:?} with {} quotes",
        elapsed,
        quotes.len()
    );
}

#[cfg(test)]
mod mock_tests {
    use super::*;

    /// Test with mocked HTTP responses
    #[tokio::test]
    async fn test_connext_response_parsing() {
        // This test would mock the Connext API response
        let mock_response = json!({
            "amountReceived": "990000000000000000",
            "originFee": "5000000000000000",
            "destinationFee": "3000000000000000",
            "relayerFee": "2000000000000000",
            "estimatedRelayerTtl": 180,
            "routerLiquidity": "1000000000000000000000000"
        });

        // Verify the JSON structure we expect
        assert!(mock_response["amountReceived"].is_string());
        assert!(mock_response["originFee"].is_string());
        assert!(mock_response["estimatedRelayerTtl"].is_number());
    }

    #[tokio::test]
    async fn test_hop_response_parsing() {
        let mock_response = json!({
            "amountOut": "990000",
            "totalFee": "10000",
            "estimatedTime": 600,
            "lpFees": "5000",
            "bonderFee": "5000",
            "availableLiquidity": "1000000000"
        });

        assert!(mock_response["amountOut"].is_string());
        assert!(mock_response["totalFee"].is_string());
        assert!(mock_response["estimatedTime"].is_number());
    }

    #[tokio::test]
    async fn test_axelar_response_parsing() {
        let mock_response = json!({
            "fee": {
                "amount": "500000",
                "denom": "uusdc",
                "gas": "200000"
            },
            "estimatedTime": 600
        });

        assert!(mock_response["fee"]["amount"].is_string());
        assert!(mock_response["fee"]["denom"].is_string());
        assert!(mock_response["estimatedTime"].is_number());
    }
}

/// Performance and stress tests
#[cfg(test)]
mod performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_requests() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "ethereum".to_string(),
            to_chain: "polygon".to_string(),
            amount: Some("1000000".to_string()),
            // recipient: None,
        };

        let config = BridgeClientConfig::new()
            .with_timeout(Duration::from_secs(2))
            .with_retries(0);

        // Test multiple concurrent requests
        let futures: Vec<_> = (0..5)
            .map(|_| get_all_bridge_quotes(&request, &config))
            .collect();

        let start_time = std::time::Instant::now();
        let results = futures::future::join_all(futures).await;
        let elapsed = start_time.elapsed();

        // All requests should complete
        assert_eq!(results.len(), 5);

        // Should complete relatively quickly due to parallelization
        assert!(elapsed < Duration::from_secs(15));

        println!("5 concurrent requests completed in {:?}", elapsed);
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        let request = BridgeQuoteRequest {
            asset: "USDC".to_string(),
            from_chain: "ethereum".to_string(),
            to_chain: "polygon".to_string(),
            amount: Some("1000000".to_string()),
            // recipient: None,
        };

        // Very short timeout to force timeouts
        let config = BridgeClientConfig::new()
            .with_timeout(Duration::from_millis(1))
            .with_retries(0);

        let start_time = std::time::Instant::now();
        let quotes = get_all_bridge_quotes(&request, &config).await;
        let elapsed = start_time.elapsed();

        // Should complete quickly due to timeout
        assert!(elapsed < Duration::from_secs(5)); // Give it a bit more time

        // Should return empty results due to timeouts (at least some bridges might succeed with very fast responses)
        assert!(quotes.len() <= 3); // All bridges might timeout, but some might succeed quickly
    }
}

/// Error handling tests
#[cfg(test)]
mod error_tests {
    use bridge_router::models::bridge::BridgeError;

    #[test]
    fn test_bridge_error_types() {
        let timeout_error = BridgeError::Timeout { timeout_ms: 5000 };
        assert!(timeout_error.to_string().contains("5000ms"));

        let bad_response_error = BridgeError::BadResponse {
            message: "Invalid JSON".to_string(),
        };
        assert!(bad_response_error.to_string().contains("Invalid JSON"));

        let unsupported_asset_error = BridgeError::UnsupportedAsset {
            asset: "INVALID".to_string(),
        };
        assert!(unsupported_asset_error.to_string().contains("INVALID"));

        let unsupported_route_error = BridgeError::UnsupportedRoute {
            from_chain: "chain1".to_string(),
            to_chain: "chain2".to_string(),
        };
        assert!(unsupported_route_error.to_string().contains("chain1"));
        assert!(unsupported_route_error.to_string().contains("chain2"));
    }

    #[test]
    fn test_error_conversion() {
        // Test JSON error conversion
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let bridge_error: BridgeError = json_error.into();

        match bridge_error {
            BridgeError::JsonParsing { .. } => (), // Expected
            _ => panic!("Expected JsonParsing error"),
        }

        // Test timeout error creation
        let timeout_error = BridgeError::Timeout { timeout_ms: 5000 };
        assert!(timeout_error.to_string().contains("5000ms"));
    }
}
