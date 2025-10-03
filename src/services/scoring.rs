use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Security metadata for a bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetadata {
    pub bridge: String,
    pub has_audit: bool,
    pub has_exploit: bool,
    pub latest_audit_result: Option<String>,
    pub exploit_count: u32,
    pub total_loss_usd: Option<f64>,
}

/// Scoring weights for different components
#[derive(Debug, Clone)]
pub struct ScoringWeights {
    pub fee_weight: f64,
    pub time_weight: f64,
    pub security_weight: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            fee_weight: 0.4,
            time_weight: 0.4,
            security_weight: 0.2,
        }
    }
}

/// Configuration for scoring normalization
#[derive(Debug, Clone)]
pub struct ScoringConfig {
    pub max_fee_threshold: f64,  // Fees above this get score 0
    pub max_time_threshold: u64, // Times above this get score 0
    pub audit_bonus: f64,        // Bonus for having audits
    pub exploit_penalty: f64,    // Penalty for having exploits
    pub weights: ScoringWeights,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            max_fee_threshold: 0.01,  // 1% fee threshold
            max_time_threshold: 3600, // 1 hour threshold
            audit_bonus: 0.2,
            exploit_penalty: 0.5,
            weights: ScoringWeights::default(),
        }
    }
}

/// Calculate heuristic score for a bridge route
///
/// # Arguments
/// * `fee` - Fee as decimal (e.g., 0.002 for 0.2%)
/// * `est_time` - Estimated time in seconds
/// * `has_audit` - Whether the bridge has been audited
/// * `has_exploit` - Whether the bridge has had exploits
///
/// # Returns
/// Score between 0.0 and 1.0 (higher is better)
pub fn calculate_score(fee: f64, est_time: u64, has_audit: bool, has_exploit: bool) -> f64 {
    calculate_score_with_config(
        fee,
        est_time,
        has_audit,
        has_exploit,
        &ScoringConfig::default(),
    )
}

/// Calculate score with custom configuration
pub fn calculate_score_with_config(
    fee: f64,
    est_time: u64,
    has_audit: bool,
    has_exploit: bool,
    config: &ScoringConfig,
) -> f64 {
    debug!(
        "Calculating score: fee={}, time={}, audit={}, exploit={}",
        fee, est_time, has_audit, has_exploit
    );

    // Calculate fee score (lower fee = higher score)
    let fee_score = if fee <= 0.0 {
        1.0 // Free transfers get perfect score
    } else if fee >= config.max_fee_threshold {
        0.0 // Fees above threshold get zero score
    } else {
        1.0 - (fee / config.max_fee_threshold)
    };

    // Calculate time score (shorter time = higher score)
    let time_score = if est_time == 0 {
        1.0 // Instant transfers get perfect score
    } else if est_time >= config.max_time_threshold {
        0.0 // Times above threshold get zero score
    } else {
        1.0 - (est_time as f64 / config.max_time_threshold as f64)
    };

    // Calculate security score
    let mut security_score = 0.5; // Baseline security score

    if has_audit {
        security_score += config.audit_bonus;
    }

    if has_exploit {
        security_score -= config.exploit_penalty;
    }

    // Clamp security score between 0 and 1
    security_score = security_score.clamp(0.0, 1.0);

    // Calculate weighted final score
    let final_score = config.weights.fee_weight * fee_score
        + config.weights.time_weight * time_score
        + config.weights.security_weight * security_score;

    // Clamp final score between 0 and 1
    let clamped_score = final_score.clamp(0.0, 1.0);

    info!(
        "Score calculation: fee_score={:.3}, time_score={:.3}, security_score={:.3}, final={:.3}",
        fee_score, time_score, security_score, clamped_score
    );

    clamped_score
}

/// Calculate batch scores for multiple routes
#[allow(dead_code)]
pub fn calculate_batch_scores(
    routes: &[(f64, u64, bool, bool)], // (fee, time, has_audit, has_exploit)
    config: Option<&ScoringConfig>,
) -> Vec<f64> {
    let default_config = ScoringConfig::default();
    let config = config.unwrap_or(&default_config);

    routes
        .iter()
        .map(|(fee, time, audit, exploit)| {
            calculate_score_with_config(*fee, *time, *audit, *exploit, config)
        })
        .collect()
}

/// Get scoring statistics for analysis
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ScoringStats {
    pub total_routes: usize,
    pub avg_score: f64,
    pub min_score: f64,
    pub max_score: f64,
    pub audited_routes: usize,
    pub exploited_routes: usize,
}

impl ScoringStats {
    #[allow(dead_code)]
    pub fn from_scores(scores: &[f64], security_metadata: &[SecurityMetadata]) -> Self {
        let total_routes = scores.len();
        let avg_score = if total_routes > 0 {
            scores.iter().sum::<f64>() / total_routes as f64
        } else {
            0.0
        };

        let min_score = scores.iter().copied().fold(f64::INFINITY, f64::min);
        let max_score = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        let audited_routes = security_metadata.iter().filter(|m| m.has_audit).count();
        let exploited_routes = security_metadata.iter().filter(|m| m.has_exploit).count();

        Self {
            total_routes,
            avg_score,
            min_score: if min_score == f64::INFINITY {
                0.0
            } else {
                min_score
            },
            max_score: if max_score == f64::NEG_INFINITY {
                0.0
            } else {
                max_score
            },
            audited_routes,
            exploited_routes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_score_basic() {
        // Test perfect route: no fee, instant, audited, no exploits
        let score = calculate_score(0.0, 0, true, false);
        assert!(
            (score - 1.0).abs() < 0.1,
            "Perfect route should score ~1.0, got {}",
            score
        );

        // Test worst route: high fee, slow, no audit, has exploits
        let score = calculate_score(0.01, 3600, false, true);
        assert!(
            score < 0.1,
            "Worst route should score very low, got {}",
            score
        );
    }

    #[test]
    fn test_calculate_score_fee_component() {
        let config = ScoringConfig::default();

        // Test fee scoring in isolation (no time/security factors)
        let score1 = calculate_score_with_config(0.0, 0, false, false, &config);
        let score2 = calculate_score_with_config(0.005, 0, false, false, &config);
        let score3 = calculate_score_with_config(0.01, 0, false, false, &config);

        // Lower fees should have higher scores
        assert!(score1 > score2);
        assert!(score2 > score3);
    }

    #[test]
    fn test_calculate_score_time_component() {
        let config = ScoringConfig::default();

        // Test time scoring in isolation
        let score1 = calculate_score_with_config(0.0, 0, false, false, &config);
        let score2 = calculate_score_with_config(0.0, 1800, false, false, &config);
        let score3 = calculate_score_with_config(0.0, 3600, false, false, &config);

        // Shorter times should have higher scores
        assert!(score1 > score2);
        assert!(score2 > score3);
    }

    #[test]
    fn test_calculate_score_security_component() {
        let config = ScoringConfig::default();

        // Test security scoring
        let no_audit_no_exploit = calculate_score_with_config(0.0, 0, false, false, &config);
        let audit_no_exploit = calculate_score_with_config(0.0, 0, true, false, &config);
        let no_audit_exploit = calculate_score_with_config(0.0, 0, false, true, &config);
        let audit_exploit = calculate_score_with_config(0.0, 0, true, true, &config);

        // Audited should be better than not audited
        assert!(audit_no_exploit > no_audit_no_exploit);

        // No exploit should be better than exploit
        assert!(no_audit_no_exploit > no_audit_exploit);

        // Audit with exploit should be better than no audit with exploit
        assert!(audit_exploit > no_audit_exploit);
    }

    #[test]
    fn test_calculate_score_realistic_scenarios() {
        // Connext: low fee, fast, audited, no recent exploits
        let connext_score = calculate_score(0.002, 120, true, false);

        // Hop: very low fee, medium time, audited, no recent exploits
        let hop_score = calculate_score(0.0015, 180, true, false);

        // Axelar: medium fee, slow, audited, no recent exploits
        let axelar_score = calculate_score(0.005, 900, true, false);

        // Wormhole: low fee, fast, audited, has exploits
        let wormhole_score = calculate_score(0.002, 120, true, true);

        // All scores should be between 0 and 1
        assert!((0.0..=1.0).contains(&connext_score));
        assert!((0.0..=1.0).contains(&hop_score));
        assert!((0.0..=1.0).contains(&axelar_score));
        assert!((0.0..=1.0).contains(&wormhole_score));

        // Hop should score highest (lowest fee)
        assert!(hop_score >= connext_score);

        // Routes without exploits should outscore those with exploits
        assert!(connext_score > wormhole_score);

        println!("Connext: {:.3}", connext_score);
        println!("Hop: {:.3}", hop_score);
        println!("Axelar: {:.3}", axelar_score);
        println!("Wormhole: {:.3}", wormhole_score);
    }

    #[test]
    fn test_batch_scoring() {
        let routes = vec![
            (0.001, 60, true, false),  // Excellent route
            (0.005, 300, true, false), // Good route
            (0.01, 600, false, true),  // Poor route
        ];

        let scores = calculate_batch_scores(&routes, None);

        assert_eq!(scores.len(), 3);
        assert!(scores[0] > scores[1]); // First should be best
        assert!(scores[1] > scores[2]); // Second should be better than third
    }

    #[test]
    fn test_scoring_config_customization() {
        let mut config = ScoringConfig::default();
        config.weights.fee_weight = 0.8; // Prioritize fees heavily
        config.weights.time_weight = 0.1;
        config.weights.security_weight = 0.1;

        let high_fee_fast = calculate_score_with_config(0.008, 60, true, false, &config);
        let low_fee_slow = calculate_score_with_config(0.001, 600, true, false, &config);

        // With heavy fee weighting, low fee should win despite being slower
        assert!(low_fee_slow > high_fee_fast);
    }

    #[test]
    fn test_edge_cases() {
        // Test edge cases
        let score = calculate_score(0.0, 0, true, false);
        assert!((score - 0.94).abs() < 0.001); // Expected score with audit bonus
        assert!(calculate_score(-0.001, 0, true, false) >= 0.0); // Negative fee
        assert!(calculate_score(f64::INFINITY, 0, true, false) >= 0.0); // Infinite fee
        assert!(calculate_score(0.0, u64::MAX, true, false) >= 0.0); // Max time
    }

    #[test]
    fn test_scoring_stats() {
        let scores = vec![0.8, 0.6, 0.9, 0.3, 0.7];
        let metadata = vec![
            SecurityMetadata {
                bridge: "Bridge1".to_string(),
                has_audit: true,
                has_exploit: false,
                latest_audit_result: Some("passed".to_string()),
                exploit_count: 0,
                total_loss_usd: None,
            },
            SecurityMetadata {
                bridge: "Bridge2".to_string(),
                has_audit: false,
                has_exploit: true,
                latest_audit_result: None,
                exploit_count: 1,
                total_loss_usd: Some(1000000.0),
            },
            SecurityMetadata {
                bridge: "Bridge3".to_string(),
                has_audit: true,
                has_exploit: false,
                latest_audit_result: Some("passed".to_string()),
                exploit_count: 0,
                total_loss_usd: None,
            },
            SecurityMetadata {
                bridge: "Bridge4".to_string(),
                has_audit: false,
                has_exploit: false,
                latest_audit_result: None,
                exploit_count: 0,
                total_loss_usd: None,
            },
            SecurityMetadata {
                bridge: "Bridge5".to_string(),
                has_audit: true,
                has_exploit: false,
                latest_audit_result: Some("passed".to_string()),
                exploit_count: 0,
                total_loss_usd: None,
            },
        ];

        let stats = ScoringStats::from_scores(&scores, &metadata);

        assert_eq!(stats.total_routes, 5);
        assert!((stats.avg_score - 0.66).abs() < 0.001);
        assert_eq!(stats.min_score, 0.3);
        assert_eq!(stats.max_score, 0.9);
        assert_eq!(stats.audited_routes, 3);
        assert_eq!(stats.exploited_routes, 1);
    }
}
