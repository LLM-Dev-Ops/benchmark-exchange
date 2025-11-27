//! Tests for scoring and aggregation logic
//!
//! Tests score normalization, aggregation methods, and confidence intervals.

use llm_benchmark_domain::submission::{ConfidenceInterval, MetricScore, StatisticalSignificance};
use llm_benchmark_testing::fixtures::*;
use std::collections::HashMap;

#[test]
fn test_aggregate_score_calculation() {
    // Arrange
    let results = create_test_submission_results();

    // Assert - Aggregate score should be reasonable
    assert!(results.aggregate_score >= 0.0);
    assert!(results.aggregate_score <= 1.0);
}

#[test]
fn test_metric_scores_present() {
    // Arrange
    let results = create_test_submission_results();

    // Assert - Should have metric scores
    assert!(!results.metric_scores.is_empty());
    assert!(results.metric_scores.contains_key("accuracy"));
}

#[test]
fn test_metric_score_with_unit() {
    // Arrange
    let mut metric_scores = HashMap::new();
    metric_scores.insert(
        "latency".to_string(),
        MetricScore {
            value: 150.0,
            unit: Some("ms".to_string()),
            raw_values: None,
            std_dev: None,
        },
    );

    // Assert
    let latency = metric_scores.get("latency").unwrap();
    assert_eq!(latency.value, 150.0);
    assert_eq!(latency.unit, Some("ms".to_string()));
}

#[test]
fn test_metric_score_with_raw_values() {
    // Arrange
    let raw_values = vec![0.91, 0.92, 0.93, 0.94];
    let mean = raw_values.iter().sum::<f64>() / raw_values.len() as f64;

    let metric = MetricScore {
        value: mean,
        unit: None,
        raw_values: Some(raw_values.clone()),
        std_dev: Some(0.012),
    };

    // Assert
    assert!(metric.raw_values.is_some());
    assert_eq!(metric.raw_values.as_ref().unwrap().len(), 4);
    assert!(metric.std_dev.is_some());
}

#[test]
fn test_confidence_interval_bounds() {
    // Arrange
    let ci = ConfidenceInterval {
        lower: 0.90,
        upper: 0.94,
        confidence_level: 0.95,
    };

    // Assert
    assert!(ci.lower < ci.upper);
    assert!(ci.confidence_level > 0.0 && ci.confidence_level < 1.0);
    assert_eq!(ci.confidence_level, 0.95);
}

#[test]
fn test_statistical_significance_p_value() {
    // Arrange
    let stats = StatisticalSignificance {
        p_value: 0.01,
        effect_size: 0.5,
        sample_size: 100,
        test_used: "t-test".to_string(),
    };

    // Assert - p-value should be between 0 and 1
    assert!(stats.p_value >= 0.0 && stats.p_value <= 1.0);

    // Significant result (p < 0.05)
    assert!(stats.p_value < 0.05);
}

#[test]
fn test_score_normalization() {
    // Test various score normalization scenarios

    // Scenario 1: Scores already normalized (0-1)
    let normalized_score = 0.85;
    assert!(normalized_score >= 0.0 && normalized_score <= 1.0);

    // Scenario 2: Percentage to decimal
    let percentage = 85.0;
    let decimal = percentage / 100.0;
    assert_eq!(decimal, 0.85);

    // Scenario 3: Inverse normalization (lower is better, e.g., latency)
    let max_latency = 1000.0;
    let actual_latency = 150.0;
    let normalized = 1.0 - (actual_latency / max_latency);
    assert!(normalized >= 0.0 && normalized <= 1.0);
}

#[test]
fn test_weighted_average_aggregation() {
    // Arrange - Different metrics with different weights
    let scores = vec![
        ("accuracy", 0.92, 0.5),   // 50% weight
        ("speed", 0.85, 0.3),      // 30% weight
        ("reliability", 0.88, 0.2), // 20% weight
    ];

    // Calculate weighted average
    let weighted_sum: f64 = scores.iter()
        .map(|(_, score, weight)| score * weight)
        .sum();

    let total_weight: f64 = scores.iter()
        .map(|(_, _, weight)| weight)
        .sum();

    let weighted_avg = weighted_sum / total_weight;

    // Assert
    assert!((total_weight - 1.0).abs() < f64::EPSILON); // Weights sum to 1
    assert!(weighted_avg >= 0.0 && weighted_avg <= 1.0);
    assert!((weighted_avg - 0.897).abs() < 0.01); // Expected: 0.92*0.5 + 0.85*0.3 + 0.88*0.2
}

#[test]
fn test_harmonic_mean_aggregation() {
    // Harmonic mean is useful for rates and ratios
    let values = vec![0.9, 0.8, 0.85];
    let n = values.len() as f64;
    let reciprocal_sum: f64 = values.iter()
        .map(|v| 1.0 / v)
        .sum();

    let harmonic_mean = n / reciprocal_sum;

    // Assert
    assert!(harmonic_mean >= 0.0 && harmonic_mean <= 1.0);
    assert!(harmonic_mean < values.iter().sum::<f64>() / n); // HM <= AM
}

#[test]
fn test_test_case_pass_rate() {
    // Arrange
    let results = create_test_submission_results();

    // Calculate pass rate
    let total = results.test_case_results.len();
    let passed = results.test_case_results.iter()
        .filter(|r| r.passed)
        .count();
    let pass_rate = passed as f64 / total as f64;

    // Assert
    assert!(pass_rate >= 0.0 && pass_rate <= 1.0);
    assert!(total > 0);
}

#[test]
fn test_score_variance_calculation() {
    // Arrange
    let scores = vec![0.90, 0.92, 0.91, 0.93, 0.89];
    let mean = scores.iter().sum::<f64>() / scores.len() as f64;

    // Calculate variance
    let variance: f64 = scores.iter()
        .map(|s| (s - mean).powi(2))
        .sum::<f64>() / scores.len() as f64;

    let std_dev = variance.sqrt();

    // Assert
    assert!(variance >= 0.0);
    assert!(std_dev >= 0.0);
    assert!(std_dev < 1.0); // For scores in 0-1 range
}

#[test]
fn test_confidence_interval_calculation() {
    // Simplified CI calculation for demonstration
    let mean: f64 = 0.92;
    let std_dev: f64 = 0.02;
    let n: f64 = 100.0;
    let z_score: f64 = 1.96; // 95% confidence

    let margin_of_error = z_score * (std_dev / n.sqrt());
    let lower = mean - margin_of_error;
    let upper = mean + margin_of_error;

    let ci = ConfidenceInterval {
        lower,
        upper,
        confidence_level: 0.95,
    };

    // Assert
    assert!(ci.lower < mean);
    assert!(ci.upper > mean);
    assert!(ci.lower < ci.upper);
}

#[test]
fn test_outlier_detection() {
    // Using IQR method for outlier detection
    let mut values = vec![0.90, 0.91, 0.92, 0.91, 0.93, 0.89, 0.50]; // 0.50 is outlier
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let q1_idx = values.len() / 4;
    let q3_idx = 3 * values.len() / 4;
    let q1 = values[q1_idx];
    let q3 = values[q3_idx];
    let iqr = q3 - q1;

    let lower_bound = q1 - 1.5 * iqr;
    let upper_bound = q3 + 1.5 * iqr;

    let outliers: Vec<f64> = values.iter()
        .filter(|&&v| v < lower_bound || v > upper_bound)
        .copied()
        .collect();

    // Assert - 0.50 should be detected as outlier
    assert!(!outliers.is_empty());
    assert!(outliers.contains(&0.50));
}

#[test]
fn test_percentile_calculation() {
    // Arrange
    let mut scores = vec![0.85, 0.90, 0.92, 0.88, 0.95, 0.87, 0.93];
    scores.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Calculate 90th percentile
    let percentile = 0.90;
    let index = (percentile * (scores.len() - 1) as f64) as usize;
    let p90 = scores[index];

    // Assert
    assert!(p90 > scores[0]); // Should be higher than minimum
    assert!(p90 >= scores[index]);
}
