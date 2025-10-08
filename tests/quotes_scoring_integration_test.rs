use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use bridge_router::{
    models::bridge::BridgeQuote,
    services::{SecurityMetadata, calculate_score},
};
use serde_json::Value;
use tower::util::ServiceExt;

/// Test that /quotes endpoint returns scores in the response
/// NOTE: This test requires database connections and is commented out for unit testing
#[tokio::test]
#[ignore] // Requires database setup
async fn test_quotes_endpoint_includes_scores() {
    // Create test app
    let app = create_test_app().await;

    // Create request with valid parameters
    let request = Request::builder()
        .method("GET")
        .uri("/quotes?from_chain=ethereum&to_chain=polygon&token=USDC&amount=100.0")
        .body(Body::empty())
        .unwrap();

    // Send request
    let response = app.oneshot(request).await.unwrap();

    // Should return 200 OK or 502 (if no bridges are available)
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::BAD_GATEWAY,
        "Expected 200 or 502, got: {}",
        response.status()
    );

    if response.status() == StatusCode::OK {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();

        // Verify response structure
        assert!(
            json.get("routes").is_some(),
            "Response should have 'routes' field"
        );

        let routes = json["routes"].as_array().unwrap();
        if !routes.is_empty() {
            // Each route should have a score field
            for route in routes {
                assert!(
                    route.get("bridge").is_some(),
                    "Route should have 'bridge' field"
                );
                assert!(
                    route.get("cost").is_some(),
                    "Route should have 'cost' field"
                );
                assert!(
                    route.get("est_time").is_some(),
                    "Route should have 'est_time' field"
                );
                assert!(
                    route.get("liquidity").is_some(),
                    "Route should have 'liquidity' field"
                );
                assert!(
                    route.get("score").is_some(),
                    "Route should have 'score' field"
                );

                let score = route["score"].as_f64().unwrap();
                assert!(
                    (0.0..=1.0).contains(&score),
                    "Score should be between 0.0 and 1.0, got: {}",
                    score
                );

                println!("Route: {}, Score: {:.3}", route["bridge"], score);
            }
        }
    }
}

/// Test score calculation with different parameters
#[tokio::test]
async fn test_score_calculation_scenarios() {
    // Test various scoring scenarios

    // Scenario 1: Perfect route (no fee, instant, audited, no exploits)
    let perfect_score = calculate_score(0.0, 0, true, false);
    assert!(
        (perfect_score - 0.94).abs() < 0.001,
        "Perfect route should score ~0.94 with audit bonus"
    );

    // Scenario 2: High fee route
    let high_fee_score = calculate_score(0.01, 60, true, false);
    let low_fee_score = calculate_score(0.001, 60, true, false);
    assert!(
        low_fee_score > high_fee_score,
        "Lower fees should score higher"
    );

    // Scenario 3: Fast vs slow routes
    let fast_score = calculate_score(0.005, 60, true, false);
    let slow_score = calculate_score(0.005, 1800, true, false);
    assert!(fast_score > slow_score, "Faster routes should score higher");

    // Scenario 4: Security impact
    let audited_score = calculate_score(0.005, 300, true, false);
    let unaudited_score = calculate_score(0.005, 300, false, false);
    assert!(
        audited_score > unaudited_score,
        "Audited bridges should score higher"
    );

    let no_exploit_score = calculate_score(0.005, 300, true, false);
    let exploit_score = calculate_score(0.005, 300, true, true);
    assert!(
        no_exploit_score > exploit_score,
        "Bridges without exploits should score higher"
    );

    println!("Perfect: {:.3}", perfect_score);
    println!(
        "High fee: {:.3}, Low fee: {:.3}",
        high_fee_score, low_fee_score
    );
    println!("Fast: {:.3}, Slow: {:.3}", fast_score, slow_score);
    println!(
        "Audited: {:.3}, Unaudited: {:.3}",
        audited_score, unaudited_score
    );
    println!(
        "No exploit: {:.3}, Exploit: {:.3}",
        no_exploit_score, exploit_score
    );
}

/// Test that quotes endpoint handles invalid parameters correctly
/// NOTE: This test requires database connections and is commented out for unit testing
#[tokio::test]
#[ignore] // Requires database setup
async fn test_quotes_endpoint_validation_with_scoring() {
    let app = create_test_app().await;

    // Test missing parameters
    let invalid_requests = vec![
        "/quotes",                                                 // missing all parameters
        "/quotes?from_chain=ethereum", // missing to_chain, token, amount
        "/quotes?from_chain=ethereum&to_chain=polygon", // missing token, amount
        "/quotes?from_chain=ethereum&to_chain=polygon&token=USDC", // missing amount
        "/quotes?from_chain=ethereum&to_chain=ethereum&token=USDC&amount=100", // same chains
        "/quotes?from_chain=ethereum&to_chain=polygon&token=USDC&amount=0", // zero amount
        "/quotes?from_chain=ethereum&to_chain=polygon&token=USDC&amount=-100", // negative amount
    ];

    for uri in invalid_requests {
        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "Invalid request {} should return 400 Bad Request",
            uri
        );
    }
}

/// Test scoring with realistic bridge scenarios
#[tokio::test]
async fn test_realistic_bridge_scoring() {
    // Create realistic bridge scenarios based on known characteristics

    let scenarios = vec![
        // (name, fee, time, has_audit, has_exploit, expected_relative_ranking)
        ("Connext", 0.002, 120, true, false, "high"), // Low fee, fast, audited, no recent exploits
        ("Hop", 0.0015, 180, true, false, "high"),    // Very low fee, medium time, audited
        ("Axelar", 0.005, 900, true, false, "medium"), // Medium fee, slower, audited
        ("Wormhole", 0.002, 120, true, true, "lower"), // Low fee, fast, but has exploits
        ("UnknownBridge", 0.008, 600, false, false, "low"), // High fee, slow, unaudited
    ];

    let mut scores: Vec<(String, f64)> = scenarios
        .iter()
        .map(|(name, fee, time, audit, exploit, _)| {
            let score = calculate_score(*fee, *time, *audit, *exploit);
            (name.to_string(), score)
        })
        .collect();

    // Sort by score descending
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    println!("Bridge rankings by score:");
    for (i, (name, score)) in scores.iter().enumerate() {
        println!("{}. {}: {:.3}", i + 1, name, score);
    }

    // Verify that Hop and Connext (low fees, no exploits) rank highly
    let hop_score = scores.iter().find(|(name, _)| name == "Hop").unwrap().1;
    let connext_score = scores.iter().find(|(name, _)| name == "Connext").unwrap().1;
    let wormhole_score = scores
        .iter()
        .find(|(name, _)| name == "Wormhole")
        .unwrap()
        .1;
    let unknown_score = scores
        .iter()
        .find(|(name, _)| name == "UnknownBridge")
        .unwrap()
        .1;

    // Bridges without exploits should score higher than those with exploits
    assert!(
        connext_score > wormhole_score,
        "Connext (no exploits) should score higher than Wormhole (exploits)"
    );
    assert!(
        hop_score > wormhole_score,
        "Hop (no exploits) should score higher than Wormhole (exploits)"
    );

    // Audited bridges should score higher than unaudited
    assert!(
        connext_score > unknown_score,
        "Connext (audited) should score higher than UnknownBridge (unaudited)"
    );

    // All scores should be in valid range
    for (_, score) in scores {
        assert!(
            (0.0..=1.0).contains(&score),
            "Score should be between 0.0 and 1.0"
        );
    }
}

/// Test score calculation edge cases
#[tokio::test]
async fn test_score_edge_cases() {
    // Test edge cases for score calculation

    // Very high fees
    let score = calculate_score(0.05, 60, true, false);
    assert!(
        (0.0..=1.0).contains(&score),
        "High fee score should be in valid range"
    );

    // Very long times
    let score = calculate_score(0.002, 7200, true, false); // 2 hours
    assert!(
        (0.0..=1.0).contains(&score),
        "Long time score should be in valid range"
    );

    // Both audit and exploit
    let score = calculate_score(0.002, 120, true, true);
    assert!(
        (0.0..=1.0).contains(&score),
        "Audited with exploit score should be in valid range"
    );

    // Neither audit nor exploit
    let score = calculate_score(0.002, 120, false, false);
    assert!(
        (0.0..=1.0).contains(&score),
        "Baseline security score should be in valid range"
    );

    // Zero values
    let score = calculate_score(0.0, 0, false, false);
    assert!(
        (0.0..=1.0).contains(&score),
        "Zero fee/time score should be in valid range"
    );

    println!("All edge case scores are in valid range");
}

/// Helper function to create test app
async fn create_test_app() -> Router {
    // For these unit tests, we'll skip the HTTP endpoint tests that require database connections
    // and focus on the scoring logic tests instead
    // This function is not used in the current test suite
    unimplemented!("HTTP endpoint tests require database setup - use unit tests for scoring logic")
}

/// Test that security metadata affects scoring correctly
#[tokio::test]
async fn test_security_metadata_impact_on_scoring() {
    // Test different security metadata scenarios

    let base_fee = 0.005;
    let base_time = 300;

    // Create scenarios with different security profiles
    let scenarios = vec![
        ("HighSecurity", true, false),    // Audited, no exploits
        ("MediumSecurity", true, true),   // Audited, but has exploits
        ("LowSecurity", false, false),    // No audit, no exploits
        ("VeryLowSecurity", false, true), // No audit, has exploits
    ];

    let mut scores = Vec::new();

    for (name, has_audit, has_exploit) in scenarios {
        let score = calculate_score(base_fee, base_time, has_audit, has_exploit);
        scores.push((name, score));
        println!(
            "{}: audit={}, exploit={}, score={:.3}",
            name, has_audit, has_exploit, score
        );
    }

    // Sort by score descending
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Verify expected ordering
    assert_eq!(
        scores[0].0, "HighSecurity",
        "HighSecurity should rank first"
    );
    assert_eq!(
        scores[scores.len() - 1].0,
        "VeryLowSecurity",
        "VeryLowSecurity should rank last"
    );

    // Verify security differences affect scores
    let high_sec_score = scores
        .iter()
        .find(|(name, _)| *name == "HighSecurity")
        .unwrap()
        .1;
    let low_sec_score = scores
        .iter()
        .find(|(name, _)| *name == "VeryLowSecurity")
        .unwrap()
        .1;

    assert!(
        high_sec_score > low_sec_score,
        "High security should score significantly higher than very low security"
    );

    let difference = high_sec_score - low_sec_score;
    assert!(
        difference > 0.1,
        "Security should have meaningful impact on score (difference: {:.3})",
        difference
    );
}

/// Test integration with mock security data
#[tokio::test]
async fn test_mock_security_data_integration() {
    // Create mock security metadata
    let security_metadata = vec![
        SecurityMetadata {
            bridge: "ConnextTest".to_string(),
            has_audit: true,
            has_exploit: false,
            latest_audit_result: Some("passed".to_string()),
            exploit_count: 0,
            total_loss_usd: None,
        },
        SecurityMetadata {
            bridge: "WormholeTest".to_string(),
            has_audit: true,
            has_exploit: true,
            latest_audit_result: Some("passed".to_string()),
            exploit_count: 1,
            total_loss_usd: Some(325_000_000.0),
        },
        SecurityMetadata {
            bridge: "UnauditedTest".to_string(),
            has_audit: false,
            has_exploit: false,
            latest_audit_result: None,
            exploit_count: 0,
            total_loss_usd: None,
        },
    ];

    // Create mock bridge quotes
    let bridge_quotes = vec![
        BridgeQuote {
            bridge: "ConnextTest".to_string(),
            fee: 0.002,
            est_time: 120,
            metadata: None,
        },
        BridgeQuote {
            bridge: "WormholeTest".to_string(),
            fee: 0.002,
            est_time: 120,
            metadata: None,
        },
        BridgeQuote {
            bridge: "UnauditedTest".to_string(),
            fee: 0.008,
            est_time: 600,
            metadata: None,
        },
    ];

    // Calculate scores using security metadata
    let mut scored_quotes = Vec::new();
    for quote in &bridge_quotes {
        let security = security_metadata.iter().find(|s| s.bridge == quote.bridge);
        let has_audit = security.map(|s| s.has_audit).unwrap_or(false);
        let has_exploit = security.map(|s| s.has_exploit).unwrap_or(false);

        let score = calculate_score(quote.fee, quote.est_time, has_audit, has_exploit);
        scored_quotes.push((quote.bridge.clone(), score));
    }

    // Sort by score
    scored_quotes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    println!("Scored quotes ranking:");
    for (bridge, score) in &scored_quotes {
        println!("{}: {:.3}", bridge, score);
    }

    // ConnextTest should rank highest (low fee, fast, audited, no exploits)
    assert_eq!(
        scored_quotes[0].0, "ConnextTest",
        "ConnextTest should rank first"
    );

    // UnauditedTest should rank lowest (high fee, slow, unaudited)
    assert_eq!(
        scored_quotes[2].0, "UnauditedTest",
        "UnauditedTest should rank last"
    );

    // ConnextTest should score higher than WormholeTest (both fast and low-fee, but Wormhole has exploits)
    let connext_score = scored_quotes
        .iter()
        .find(|(name, _)| name == "ConnextTest")
        .unwrap()
        .1;
    let wormhole_score = scored_quotes
        .iter()
        .find(|(name, _)| name == "WormholeTest")
        .unwrap()
        .1;
    assert!(
        connext_score > wormhole_score,
        "ConnextTest should score higher than WormholeTest due to no exploits"
    );
}
