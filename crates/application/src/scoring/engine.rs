//! Scoring Engine - Core scoring and aggregation logic
//!
//! The scoring engine coordinates evaluation of submissions, aggregates scores
//! across test cases, and computes confidence intervals and statistical metrics.

use crate::scoring::evaluators::{
    ContainsEvaluator, EvaluationResult, Evaluator, EvaluatorConfig,
    ExactMatchEvaluator, FuzzyMatchEvaluator, JsonSchemaEvaluator, NumericToleranceEvaluator,
    RegexMatchEvaluator,
};
use crate::ApplicationError;
use llm_benchmark_domain::evaluation::{AggregationMethod, EvaluationCriteria, ScoreNormalization};
use llm_benchmark_domain::submission::{
    ConfidenceInterval, MetricScore, StatisticalSignificance, SubmissionResults, TestCaseResult,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

/// Scoring engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringEngineConfig {
    /// Default confidence level for statistical calculations
    pub default_confidence_level: f64,
    /// Minimum number of test cases required for statistical analysis
    pub min_test_cases_for_stats: usize,
    /// Maximum number of concurrent evaluations
    pub max_concurrent_evaluations: usize,
    /// Enable detailed scoring breakdown
    pub detailed_breakdown: bool,
    /// Z-score threshold for outlier detection
    pub outlier_z_threshold: f64,
}

impl Default for ScoringEngineConfig {
    fn default() -> Self {
        Self {
            default_confidence_level: 0.95,
            min_test_cases_for_stats: 30,
            max_concurrent_evaluations: 100,
            detailed_breakdown: true,
            outlier_z_threshold: 3.0,
        }
    }
}

/// Test case input for scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseInput {
    /// Unique identifier for this test case
    pub id: String,
    /// The expected output/answer
    pub expected: String,
    /// The actual output from the model
    pub actual: String,
    /// Additional context for evaluation
    pub context: HashMap<String, serde_json::Value>,
    /// Latency of the model response in milliseconds
    pub latency_ms: Option<u64>,
    /// Number of tokens generated
    pub tokens_generated: Option<u32>,
    /// Weight for this test case (default 1.0)
    pub weight: f64,
}

impl Default for TestCaseInput {
    fn default() -> Self {
        Self {
            id: String::new(),
            expected: String::new(),
            actual: String::new(),
            context: HashMap::new(),
            latency_ms: None,
            tokens_generated: None,
            weight: 1.0,
        }
    }
}

/// Scoring request containing all test cases to evaluate
#[derive(Debug, Clone)]
pub struct ScoringRequest {
    /// Test cases to evaluate
    pub test_cases: Vec<TestCaseInput>,
    /// Evaluation criteria from the benchmark definition
    pub criteria: EvaluationCriteria,
    /// Optional metadata about the submission
    pub metadata: HashMap<String, serde_json::Value>,
}

/// The main scoring engine
pub struct ScoringEngine {
    config: ScoringEngineConfig,
    evaluators: HashMap<String, Arc<dyn Evaluator>>,
}

impl ScoringEngine {
    /// Create a new scoring engine with default evaluators
    pub fn new(config: ScoringEngineConfig) -> Self {
        let mut engine = Self {
            config,
            evaluators: HashMap::new(),
        };
        engine.register_default_evaluators();
        engine
    }

    /// Register default evaluators
    fn register_default_evaluators(&mut self) {
        // Exact match evaluator
        self.register_evaluator("exact_match", Arc::new(ExactMatchEvaluator));

        // Fuzzy match evaluator
        self.register_evaluator("fuzzy_match", Arc::new(FuzzyMatchEvaluator));

        // Numeric tolerance evaluator
        self.register_evaluator("numeric_tolerance", Arc::new(NumericToleranceEvaluator));

        // Contains evaluator
        self.register_evaluator("contains", Arc::new(ContainsEvaluator));

        // Regex evaluator
        self.register_evaluator("regex", Arc::new(RegexMatchEvaluator));

        // JSON schema evaluator
        self.register_evaluator("json_schema", Arc::new(JsonSchemaEvaluator));
    }

    /// Register a custom evaluator
    pub fn register_evaluator(&mut self, name: &str, evaluator: Arc<dyn Evaluator>) {
        self.evaluators.insert(name.to_string(), evaluator);
    }

    /// Get an evaluator by name
    pub fn get_evaluator(&self, name: &str) -> Option<Arc<dyn Evaluator>> {
        self.evaluators.get(name).cloned()
    }

    /// Score a submission request
    #[instrument(skip(self, request), fields(test_cases = request.test_cases.len()))]
    pub async fn score(&self, request: &ScoringRequest) -> Result<SubmissionResults, ApplicationError> {
        info!(
            "Starting scoring for {} test cases",
            request.test_cases.len()
        );

        // Validate minimum test cases
        if request.test_cases.len() < request.criteria.minimum_test_cases {
            return Err(ApplicationError::ValidationFailed(format!(
                "Insufficient test cases: got {}, required {}",
                request.test_cases.len(),
                request.criteria.minimum_test_cases
            )));
        }

        // Determine evaluator from metric type
        let evaluator_name = self.metric_type_to_evaluator(&request.criteria.primary_metric.metric_type);
        let evaluator = self
            .get_evaluator(&evaluator_name)
            .ok_or_else(|| ApplicationError::Internal(format!("Evaluator not found: {}", evaluator_name)))?;

        // Evaluate all test cases
        let mut test_case_results = Vec::with_capacity(request.test_cases.len());
        let mut scores: Vec<f64> = Vec::with_capacity(request.test_cases.len());
        let mut weights: Vec<f64> = Vec::with_capacity(request.test_cases.len());

        let eval_config = EvaluatorConfig::default();

        for test_case in &request.test_cases {
            let eval_result = evaluator
                .evaluate(&test_case.actual, Some(&test_case.expected), &eval_config)
                .await;

            let (passed, score, error) = if eval_result.error.is_some() {
                warn!(test_case_id = %test_case.id, error = ?eval_result.error, "Evaluation error");
                (
                    false,
                    0.0,
                    Some(llm_benchmark_domain::submission::TestCaseError {
                        error_type: llm_benchmark_domain::submission::TestCaseErrorType::EvaluationError,
                        message: eval_result.error.unwrap_or_default(),
                    }),
                )
            } else {
                (eval_result.passed, eval_result.score, None)
            };

            test_case_results.push(TestCaseResult {
                test_case_id: test_case.id.clone(),
                passed,
                score,
                latency_ms: test_case.latency_ms,
                tokens_generated: test_case.tokens_generated,
                error,
            });

            scores.push(score);
            weights.push(test_case.weight);
        }

        // Calculate aggregate score
        let aggregate_score = self.aggregate_scores(&scores, &weights, &request.criteria.aggregation_method)?;

        // Normalize score if configured
        let aggregate_score = self.normalize_score(aggregate_score, &request.criteria.score_normalization);

        // Calculate metric scores
        let mut metric_scores = HashMap::new();

        // Primary metric
        let primary_raw: Vec<f64> = scores.iter().copied().collect();
        let primary_std_dev = self.calculate_std_dev(&primary_raw);
        metric_scores.insert(
            request.criteria.primary_metric.name.clone(),
            MetricScore {
                value: aggregate_score,
                unit: request.criteria.primary_metric.unit.clone(),
                raw_values: Some(primary_raw),
                std_dev: Some(primary_std_dev),
            },
        );

        // Secondary metrics (if any)
        for secondary in &request.criteria.secondary_metrics {
            // For now, secondary metrics use the same scores
            // In production, each metric would have its own evaluation
            metric_scores.insert(
                secondary.name.clone(),
                MetricScore {
                    value: aggregate_score,
                    unit: secondary.unit.clone(),
                    raw_values: None,
                    std_dev: None,
                },
            );
        }

        // Calculate confidence interval if enough samples
        let confidence_interval = if scores.len() >= self.config.min_test_cases_for_stats {
            Some(self.calculate_confidence_interval(
                &scores,
                request.criteria.confidence_level,
            ))
        } else {
            None
        };

        // Calculate statistical significance
        let statistical_significance = if scores.len() >= self.config.min_test_cases_for_stats {
            Some(self.calculate_statistical_significance(&scores))
        } else {
            None
        };

        debug!(
            aggregate_score = %aggregate_score,
            passed_count = test_case_results.iter().filter(|r| r.passed).count(),
            "Scoring complete"
        );

        Ok(SubmissionResults {
            aggregate_score,
            metric_scores,
            test_case_results,
            confidence_interval,
            statistical_significance,
        })
    }

    /// Map metric type to evaluator name
    fn metric_type_to_evaluator(&self, metric_type: &llm_benchmark_domain::evaluation::MetricType) -> String {
        use llm_benchmark_domain::evaluation::MetricType;
        match metric_type {
            MetricType::ExactMatch => "exact_match".to_string(),
            MetricType::Accuracy => "exact_match".to_string(),
            MetricType::F1Score => "fuzzy_match".to_string(),
            MetricType::Bleu => "fuzzy_match".to_string(),
            MetricType::Rouge => "fuzzy_match".to_string(),
            MetricType::Perplexity => "numeric_tolerance".to_string(),
            MetricType::Latency => "numeric_tolerance".to_string(),
            MetricType::Throughput => "numeric_tolerance".to_string(),
            MetricType::CostPerToken => "numeric_tolerance".to_string(),
            MetricType::Custom { .. } => "exact_match".to_string(),
        }
    }

    /// Aggregate scores using the specified method
    fn aggregate_scores(
        &self,
        scores: &[f64],
        weights: &[f64],
        method: &AggregationMethod,
    ) -> Result<f64, ApplicationError> {
        if scores.is_empty() {
            return Err(ApplicationError::ValidationFailed(
                "No scores to aggregate".to_string(),
            ));
        }

        match method {
            AggregationMethod::Mean => Ok(self.mean(scores)),
            AggregationMethod::WeightedMean { weights: method_weights } => {
                // Use method weights if provided, otherwise use test case weights
                let w: Vec<f64> = if method_weights.is_empty() {
                    weights.to_vec()
                } else {
                    scores
                        .iter()
                        .enumerate()
                        .map(|(i, _)| method_weights.get(&i.to_string()).copied().unwrap_or(1.0))
                        .collect()
                };
                Ok(self.weighted_mean(scores, &w))
            }
            AggregationMethod::Median => Ok(self.median(scores)),
            AggregationMethod::GeometricMean => Ok(self.geometric_mean(scores)),
            AggregationMethod::HarmonicMean => Ok(self.harmonic_mean(scores)),
            AggregationMethod::Min => Ok(scores.iter().cloned().fold(f64::INFINITY, f64::min)),
            AggregationMethod::Max => Ok(scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max)),
            AggregationMethod::Percentile { percentile } => Ok(self.percentile(scores, *percentile)),
            AggregationMethod::Custom { formula } => {
                // For custom formulas, fall back to mean
                warn!(formula = %formula, "Custom aggregation not implemented, using mean");
                Ok(self.mean(scores))
            }
        }
    }

    /// Calculate arithmetic mean
    fn mean(&self, scores: &[f64]) -> f64 {
        if scores.is_empty() {
            return 0.0;
        }
        scores.iter().sum::<f64>() / scores.len() as f64
    }

    /// Calculate weighted mean
    fn weighted_mean(&self, scores: &[f64], weights: &[f64]) -> f64 {
        if scores.is_empty() || weights.is_empty() {
            return 0.0;
        }

        let weight_sum: f64 = weights.iter().sum();
        if weight_sum == 0.0 {
            return self.mean(scores);
        }

        scores
            .iter()
            .zip(weights.iter())
            .map(|(s, w)| s * w)
            .sum::<f64>()
            / weight_sum
    }

    /// Calculate median
    fn median(&self, scores: &[f64]) -> f64 {
        if scores.is_empty() {
            return 0.0;
        }

        let mut sorted: Vec<f64> = scores.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    }

    /// Calculate geometric mean
    fn geometric_mean(&self, scores: &[f64]) -> f64 {
        if scores.is_empty() {
            return 0.0;
        }

        // Use log-sum-exp for numerical stability
        let log_sum: f64 = scores
            .iter()
            .filter(|&&s| s > 0.0)
            .map(|s| s.ln())
            .sum();
        let valid_count = scores.iter().filter(|&&s| s > 0.0).count();

        if valid_count == 0 {
            return 0.0;
        }

        (log_sum / valid_count as f64).exp()
    }

    /// Calculate harmonic mean
    fn harmonic_mean(&self, scores: &[f64]) -> f64 {
        if scores.is_empty() {
            return 0.0;
        }

        let reciprocal_sum: f64 = scores
            .iter()
            .filter(|&&s| s > 0.0)
            .map(|s| 1.0 / s)
            .sum();
        let valid_count = scores.iter().filter(|&&s| s > 0.0).count();

        if valid_count == 0 || reciprocal_sum == 0.0 {
            return 0.0;
        }

        valid_count as f64 / reciprocal_sum
    }

    /// Calculate percentile
    fn percentile(&self, scores: &[f64], p: f64) -> f64 {
        if scores.is_empty() {
            return 0.0;
        }

        let mut sorted: Vec<f64> = scores.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p = p.clamp(0.0, 100.0) / 100.0;
        let idx = (sorted.len() as f64 * p).floor() as usize;
        let idx = idx.min(sorted.len() - 1);

        sorted[idx]
    }

    /// Normalize a score according to the configured method
    fn normalize_score(&self, score: f64, method: &ScoreNormalization) -> f64 {
        match method {
            ScoreNormalization::None => score,
            ScoreNormalization::MinMax { min, max } => {
                if max - min == 0.0 {
                    return 0.5;
                }
                (score - min) / (max - min)
            }
            ScoreNormalization::ZScore => {
                // Z-score normalization requires population stats
                // For single score, return as-is
                score
            }
            ScoreNormalization::Percentile => {
                // Percentile normalization requires population
                score
            }
            ScoreNormalization::LogScale => {
                if score <= 0.0 {
                    return 0.0;
                }
                score.ln()
            }
        }
    }

    /// Calculate standard deviation
    fn calculate_std_dev(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }

        let mean = self.mean(values);
        let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
            / (values.len() - 1) as f64;

        variance.sqrt()
    }

    /// Calculate confidence interval
    fn calculate_confidence_interval(&self, scores: &[f64], confidence_level: f64) -> ConfidenceInterval {
        let n = scores.len() as f64;
        let mean = self.mean(scores);
        let std_dev = self.calculate_std_dev(scores);
        let std_error = std_dev / n.sqrt();

        // Z-scores for common confidence levels
        let z = match confidence_level {
            l if (l - 0.90).abs() < 0.01 => 1.645,
            l if (l - 0.95).abs() < 0.01 => 1.96,
            l if (l - 0.99).abs() < 0.01 => 2.576,
            _ => 1.96, // Default to 95%
        };

        let margin = z * std_error;

        ConfidenceInterval {
            lower: mean - margin,
            upper: mean + margin,
            confidence_level,
        }
    }

    /// Calculate statistical significance metrics
    fn calculate_statistical_significance(&self, scores: &[f64]) -> StatisticalSignificance {
        let n = scores.len();
        let mean = self.mean(scores);
        let std_dev = self.calculate_std_dev(scores);

        // Calculate t-statistic for one-sample t-test against 0
        let t_stat = if std_dev > 0.0 {
            mean / (std_dev / (n as f64).sqrt())
        } else {
            0.0
        };

        // Approximate p-value using normal distribution (valid for large n)
        // For exact p-values, would need t-distribution tables
        let p_value = 2.0 * (1.0 - self.standard_normal_cdf(t_stat.abs()));

        // Cohen's d effect size
        let effect_size = if std_dev > 0.0 { mean / std_dev } else { 0.0 };

        StatisticalSignificance {
            p_value,
            effect_size,
            sample_size: n,
            test_used: "one-sample t-test".to_string(),
        }
    }

    /// Approximate standard normal CDF using error function approximation
    fn standard_normal_cdf(&self, x: f64) -> f64 {
        0.5 * (1.0 + self.erf(x / std::f64::consts::SQRT_2))
    }

    /// Error function approximation (Horner's method)
    fn erf(&self, x: f64) -> f64 {
        // Abramowitz and Stegun approximation
        let a1 = 0.254829592;
        let a2 = -0.284496736;
        let a3 = 1.421413741;
        let a4 = -1.453152027;
        let a5 = 1.061405429;
        let p = 0.3275911;

        let sign = if x < 0.0 { -1.0 } else { 1.0 };
        let x = x.abs();

        let t = 1.0 / (1.0 + p * x);
        let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

        sign * y
    }

    /// Detect outliers using Z-score method
    pub fn detect_outliers(&self, scores: &[f64]) -> Vec<usize> {
        if scores.len() < 3 {
            return vec![];
        }

        let mean = self.mean(scores);
        let std_dev = self.calculate_std_dev(scores);

        if std_dev == 0.0 {
            return vec![];
        }

        scores
            .iter()
            .enumerate()
            .filter(|(_, &score)| {
                let z_score = (score - mean).abs() / std_dev;
                z_score > self.config.outlier_z_threshold
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Score with outlier removal
    #[instrument(skip(self, request))]
    pub async fn score_with_outlier_removal(
        &self,
        request: &ScoringRequest,
    ) -> Result<(SubmissionResults, Vec<String>), ApplicationError> {
        // First pass: score all test cases
        let initial_results = self.score(request).await?;

        // Detect outliers from initial scores
        let scores: Vec<f64> = initial_results
            .test_case_results
            .iter()
            .map(|r| r.score)
            .collect();
        let outlier_indices = self.detect_outliers(&scores);

        if outlier_indices.is_empty() {
            return Ok((initial_results, vec![]));
        }

        // Remove outliers and re-score
        let outlier_ids: Vec<String> = outlier_indices
            .iter()
            .map(|&i| request.test_cases[i].id.clone())
            .collect();

        let filtered_test_cases: Vec<TestCaseInput> = request
            .test_cases
            .iter()
            .enumerate()
            .filter(|(i, _)| !outlier_indices.contains(i))
            .map(|(_, tc)| tc.clone())
            .collect();

        let filtered_request = ScoringRequest {
            test_cases: filtered_test_cases,
            criteria: request.criteria.clone(),
            metadata: request.metadata.clone(),
        };

        let filtered_results = self.score(&filtered_request).await?;

        info!(
            outliers_removed = outlier_ids.len(),
            "Completed scoring with outlier removal"
        );

        Ok((filtered_results, outlier_ids))
    }
}

/// Builder for ScoringEngine
pub struct ScoringEngineBuilder {
    config: ScoringEngineConfig,
    custom_evaluators: HashMap<String, Arc<dyn Evaluator>>,
}

impl ScoringEngineBuilder {
    pub fn new() -> Self {
        Self {
            config: ScoringEngineConfig::default(),
            custom_evaluators: HashMap::new(),
        }
    }

    pub fn config(mut self, config: ScoringEngineConfig) -> Self {
        self.config = config;
        self
    }

    pub fn confidence_level(mut self, level: f64) -> Self {
        self.config.default_confidence_level = level;
        self
    }

    pub fn min_test_cases_for_stats(mut self, min: usize) -> Self {
        self.config.min_test_cases_for_stats = min;
        self
    }

    pub fn max_concurrent_evaluations(mut self, max: usize) -> Self {
        self.config.max_concurrent_evaluations = max;
        self
    }

    pub fn evaluator(mut self, name: &str, evaluator: Arc<dyn Evaluator>) -> Self {
        self.custom_evaluators.insert(name.to_string(), evaluator);
        self
    }

    pub fn build(self) -> ScoringEngine {
        let mut engine = ScoringEngine::new(self.config);

        // Register custom evaluators
        for (name, evaluator) in self.custom_evaluators {
            engine.register_evaluator(&name, evaluator);
        }

        engine
    }
}

impl Default for ScoringEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_benchmark_domain::evaluation::{MetricDefinition, MetricType};

    fn make_test_criteria() -> EvaluationCriteria {
        EvaluationCriteria {
            primary_metric: MetricDefinition {
                name: "accuracy".to_string(),
                description: "Test accuracy".to_string(),
                metric_type: MetricType::Accuracy,
                unit: Some("%".to_string()),
                higher_is_better: true,
                range: None,
            },
            secondary_metrics: vec![],
            aggregation_method: AggregationMethod::Mean,
            score_normalization: ScoreNormalization::None,
            minimum_test_cases: 1,
            confidence_level: 0.95,
        }
    }

    fn make_test_case(id: &str, expected: &str, actual: &str) -> TestCaseInput {
        TestCaseInput {
            id: id.to_string(),
            expected: expected.to_string(),
            actual: actual.to_string(),
            context: HashMap::new(),
            latency_ms: Some(100),
            tokens_generated: Some(50),
            weight: 1.0,
        }
    }

    #[tokio::test]
    async fn test_scoring_exact_match() {
        let engine = ScoringEngine::new(ScoringEngineConfig::default());

        let request = ScoringRequest {
            test_cases: vec![
                make_test_case("1", "hello", "hello"),
                make_test_case("2", "world", "world"),
                make_test_case("3", "test", "wrong"),
            ],
            criteria: make_test_criteria(),
            metadata: HashMap::new(),
        };

        let results = engine.score(&request).await.unwrap();

        assert_eq!(results.test_case_results.len(), 3);
        assert!(results.test_case_results[0].passed);
        assert!(results.test_case_results[1].passed);
        assert!(!results.test_case_results[2].passed);

        // 2 out of 3 correct = 0.666... average
        assert!(results.aggregate_score > 0.6 && results.aggregate_score < 0.7);
    }

    #[tokio::test]
    async fn test_aggregation_methods() {
        let engine = ScoringEngine::new(ScoringEngineConfig::default());

        let scores = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let weights = vec![1.0; 5];

        // Mean
        assert_eq!(engine.mean(&scores), 3.0);

        // Median
        assert_eq!(engine.median(&scores), 3.0);

        // Geometric mean
        let gm = engine.geometric_mean(&scores);
        assert!((gm - 2.605).abs() < 0.01);

        // Harmonic mean
        let hm = engine.harmonic_mean(&scores);
        assert!((hm - 2.189).abs() < 0.01);

        // Weighted mean with equal weights
        assert_eq!(engine.weighted_mean(&scores, &weights), 3.0);

        // Percentile
        assert_eq!(engine.percentile(&scores, 50.0), 3.0);
        assert_eq!(engine.percentile(&scores, 0.0), 1.0);
        assert_eq!(engine.percentile(&scores, 100.0), 5.0);
    }

    #[tokio::test]
    async fn test_confidence_interval() {
        let engine = ScoringEngine::new(ScoringEngineConfig::default());

        let scores: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let ci = engine.calculate_confidence_interval(&scores, 0.95);

        assert!(ci.lower < ci.upper);
        assert_eq!(ci.confidence_level, 0.95);

        let mean = engine.mean(&scores);
        assert!(ci.lower < mean && mean < ci.upper);
    }

    #[tokio::test]
    async fn test_outlier_detection() {
        let engine = ScoringEngine::new(ScoringEngineConfig {
            outlier_z_threshold: 2.0,
            ..Default::default()
        });

        let scores = vec![1.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 100.0]; // 100 is an outlier
        let outliers = engine.detect_outliers(&scores);

        assert!(!outliers.is_empty());
        assert!(outliers.contains(&7)); // Index of 100.0
    }

    #[test]
    fn test_std_dev() {
        let engine = ScoringEngine::new(ScoringEngineConfig::default());

        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let std_dev = engine.calculate_std_dev(&values);

        // Expected std dev ~2.138
        assert!((std_dev - 2.138).abs() < 0.01);
    }

    #[test]
    fn test_builder() {
        let engine = ScoringEngineBuilder::new()
            .confidence_level(0.99)
            .min_test_cases_for_stats(50)
            .build();

        assert_eq!(engine.config.default_confidence_level, 0.99);
        assert_eq!(engine.config.min_test_cases_for_stats, 50);
    }
}
