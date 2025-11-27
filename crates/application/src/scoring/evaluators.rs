//! Evaluator implementations for different evaluation methods.
//!
//! This module provides concrete implementations of various evaluation
//! methods used to score LLM outputs against expected results.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, instrument};

/// Result of an evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    /// Score between 0.0 and 1.0
    pub score: f64,
    /// Whether the test case passed
    pub passed: bool,
    /// Detailed breakdown of the evaluation
    pub details: HashMap<String, serde_json::Value>,
    /// Optional error message
    pub error: Option<String>,
}

impl EvaluationResult {
    /// Create a successful result.
    pub fn success(score: f64) -> Self {
        Self {
            score: score.clamp(0.0, 1.0),
            passed: score >= 0.5,
            details: HashMap::new(),
            error: None,
        }
    }

    /// Create a successful result with custom pass threshold.
    pub fn success_with_threshold(score: f64, threshold: f64) -> Self {
        Self {
            score: score.clamp(0.0, 1.0),
            passed: score >= threshold,
            details: HashMap::new(),
            error: None,
        }
    }

    /// Create a failed result with an error.
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            score: 0.0,
            passed: false,
            details: HashMap::new(),
            error: Some(error.into()),
        }
    }

    /// Add a detail to the result.
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.details.insert(key.into(), v);
        }
        self
    }
}

/// Evaluator trait for scoring outputs.
#[async_trait]
pub trait Evaluator: Send + Sync {
    /// Evaluate an actual output against expected output.
    async fn evaluate(
        &self,
        actual: &str,
        expected: Option<&str>,
        config: &EvaluatorConfig,
    ) -> EvaluationResult;

    /// Get the evaluator type name.
    fn name(&self) -> &'static str;
}

/// Configuration for evaluators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatorConfig {
    /// Threshold for passing (default 0.5)
    pub pass_threshold: f64,
    /// Case sensitivity for string comparisons
    pub case_sensitive: bool,
    /// Whether to trim whitespace
    pub trim_whitespace: bool,
    /// Additional parameters
    pub params: HashMap<String, serde_json::Value>,
}

impl Default for EvaluatorConfig {
    fn default() -> Self {
        Self {
            pass_threshold: 0.5,
            case_sensitive: true,
            trim_whitespace: true,
            params: HashMap::new(),
        }
    }
}

impl EvaluatorConfig {
    /// Get a parameter value.
    pub fn get_param<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.params.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Set a parameter value.
    pub fn set_param<T: Serialize>(mut self, key: impl Into<String>, value: T) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.params.insert(key.into(), v);
        }
        self
    }
}

/// Exact match evaluator.
pub struct ExactMatchEvaluator;

#[async_trait]
impl Evaluator for ExactMatchEvaluator {
    #[instrument(skip(self, actual, expected))]
    async fn evaluate(
        &self,
        actual: &str,
        expected: Option<&str>,
        config: &EvaluatorConfig,
    ) -> EvaluationResult {
        let expected = match expected {
            Some(e) => e,
            None => return EvaluationResult::failure("No expected output provided"),
        };

        let (actual_normalized, expected_normalized) = if config.trim_whitespace {
            (actual.trim(), expected.trim())
        } else {
            (actual, expected)
        };

        let matches = if config.case_sensitive {
            actual_normalized == expected_normalized
        } else {
            actual_normalized.eq_ignore_ascii_case(expected_normalized)
        };

        let score = if matches { 1.0 } else { 0.0 };

        debug!(matches = matches, "Exact match evaluation");

        EvaluationResult::success_with_threshold(score, config.pass_threshold)
            .with_detail("match_type", "exact")
            .with_detail("case_sensitive", config.case_sensitive)
    }

    fn name(&self) -> &'static str {
        "exact_match"
    }
}

/// Fuzzy match evaluator using Levenshtein distance.
pub struct FuzzyMatchEvaluator;

impl FuzzyMatchEvaluator {
    /// Calculate Levenshtein distance between two strings.
    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let len1 = s1.chars().count();
        let len2 = s2.chars().count();

        if len1 == 0 {
            return len2;
        }
        if len2 == 0 {
            return len1;
        }

        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        for (i, row) in matrix.iter_mut().enumerate().take(len1 + 1) {
            row[0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                    0
                } else {
                    1
                };

                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[len1][len2]
    }

    /// Calculate similarity score (0.0 to 1.0) from Levenshtein distance.
    fn similarity(s1: &str, s2: &str) -> f64 {
        let distance = Self::levenshtein_distance(s1, s2);
        let max_len = s1.len().max(s2.len());

        if max_len == 0 {
            return 1.0;
        }

        1.0 - (distance as f64 / max_len as f64)
    }
}

#[async_trait]
impl Evaluator for FuzzyMatchEvaluator {
    #[instrument(skip(self, actual, expected))]
    async fn evaluate(
        &self,
        actual: &str,
        expected: Option<&str>,
        config: &EvaluatorConfig,
    ) -> EvaluationResult {
        let expected = match expected {
            Some(e) => e,
            None => return EvaluationResult::failure("No expected output provided"),
        };

        let (actual_normalized, expected_normalized) = if config.trim_whitespace {
            (actual.trim().to_string(), expected.trim().to_string())
        } else {
            (actual.to_string(), expected.to_string())
        };

        let (a, e) = if config.case_sensitive {
            (actual_normalized, expected_normalized)
        } else {
            (actual_normalized.to_lowercase(), expected_normalized.to_lowercase())
        };

        let similarity = Self::similarity(&a, &e);
        let threshold = config.get_param::<f64>("threshold").unwrap_or(0.8);

        debug!(similarity = similarity, threshold = threshold, "Fuzzy match evaluation");

        EvaluationResult::success_with_threshold(similarity, threshold)
            .with_detail("match_type", "fuzzy")
            .with_detail("similarity", similarity)
            .with_detail("threshold", threshold)
    }

    fn name(&self) -> &'static str {
        "fuzzy_match"
    }
}

/// Numeric comparison evaluator with tolerance.
pub struct NumericToleranceEvaluator;

#[async_trait]
impl Evaluator for NumericToleranceEvaluator {
    #[instrument(skip(self, actual, expected))]
    async fn evaluate(
        &self,
        actual: &str,
        expected: Option<&str>,
        config: &EvaluatorConfig,
    ) -> EvaluationResult {
        let expected = match expected {
            Some(e) => e,
            None => return EvaluationResult::failure("No expected output provided"),
        };

        let actual_num: f64 = match actual.trim().parse() {
            Ok(n) => n,
            Err(_) => return EvaluationResult::failure("Could not parse actual output as number"),
        };

        let expected_num: f64 = match expected.trim().parse() {
            Ok(n) => n,
            Err(_) => return EvaluationResult::failure("Could not parse expected output as number"),
        };

        let tolerance = config.get_param::<f64>("tolerance").unwrap_or(0.01);
        let relative = config.get_param::<bool>("relative").unwrap_or(false);

        let diff = (actual_num - expected_num).abs();
        let within_tolerance = if relative {
            let relative_diff = if expected_num.abs() > f64::EPSILON {
                diff / expected_num.abs()
            } else {
                diff
            };
            relative_diff <= tolerance
        } else {
            diff <= tolerance
        };

        let score = if within_tolerance {
            1.0
        } else {
            // Gradual decay based on how far from tolerance
            let max_diff = if relative {
                expected_num.abs() * tolerance * 10.0
            } else {
                tolerance * 10.0
            };
            (1.0 - diff / max_diff).max(0.0)
        };

        debug!(
            actual = actual_num,
            expected = expected_num,
            diff = diff,
            within_tolerance = within_tolerance,
            "Numeric tolerance evaluation"
        );

        EvaluationResult::success_with_threshold(score, config.pass_threshold)
            .with_detail("match_type", "numeric_tolerance")
            .with_detail("actual_value", actual_num)
            .with_detail("expected_value", expected_num)
            .with_detail("difference", diff)
            .with_detail("tolerance", tolerance)
            .with_detail("within_tolerance", within_tolerance)
    }

    fn name(&self) -> &'static str {
        "numeric_tolerance"
    }
}

/// Contains match evaluator - checks if output contains required substrings.
pub struct ContainsEvaluator;

#[async_trait]
impl Evaluator for ContainsEvaluator {
    #[instrument(skip(self, actual, expected, config))]
    async fn evaluate(
        &self,
        actual: &str,
        expected: Option<&str>,
        config: &EvaluatorConfig,
    ) -> EvaluationResult {
        let required: Vec<String> = config.get_param("required").unwrap_or_default();
        let forbidden: Vec<String> = config.get_param("forbidden").unwrap_or_default();

        if required.is_empty() && forbidden.is_empty() {
            // Fallback to checking if actual contains expected
            if let Some(exp) = expected {
                let contains = if config.case_sensitive {
                    actual.contains(exp)
                } else {
                    actual.to_lowercase().contains(&exp.to_lowercase())
                };
                return EvaluationResult::success_with_threshold(
                    if contains { 1.0 } else { 0.0 },
                    config.pass_threshold,
                )
                .with_detail("contains_expected", contains);
            }
            return EvaluationResult::failure("No required substrings specified");
        }

        let actual_normalized = if config.case_sensitive {
            actual.to_string()
        } else {
            actual.to_lowercase()
        };

        // Check required substrings
        let mut matched_required = 0;
        let mut missing_required = Vec::new();
        for req in &required {
            let req_normalized = if config.case_sensitive {
                req.clone()
            } else {
                req.to_lowercase()
            };
            if actual_normalized.contains(&req_normalized) {
                matched_required += 1;
            } else {
                missing_required.push(req.clone());
            }
        }

        // Check forbidden substrings
        let mut found_forbidden = Vec::new();
        for forb in &forbidden {
            let forb_normalized = if config.case_sensitive {
                forb.clone()
            } else {
                forb.to_lowercase()
            };
            if actual_normalized.contains(&forb_normalized) {
                found_forbidden.push(forb.clone());
            }
        }

        // Calculate score
        let required_score = if required.is_empty() {
            1.0
        } else {
            matched_required as f64 / required.len() as f64
        };

        let forbidden_penalty = if forbidden.is_empty() {
            0.0
        } else {
            found_forbidden.len() as f64 / forbidden.len() as f64
        };

        let score = (required_score - forbidden_penalty * 0.5).max(0.0);
        let passed = missing_required.is_empty() && found_forbidden.is_empty();

        debug!(
            matched_required = matched_required,
            total_required = required.len(),
            found_forbidden = found_forbidden.len(),
            score = score,
            "Contains evaluation"
        );

        EvaluationResult {
            score,
            passed,
            details: {
                let mut d = HashMap::new();
                d.insert("match_type".to_string(), serde_json::json!("contains"));
                d.insert("matched_required".to_string(), serde_json::json!(matched_required));
                d.insert("missing_required".to_string(), serde_json::json!(missing_required));
                d.insert("found_forbidden".to_string(), serde_json::json!(found_forbidden));
                d
            },
            error: None,
        }
    }

    fn name(&self) -> &'static str {
        "contains"
    }
}

/// Regex match evaluator.
pub struct RegexMatchEvaluator;

#[async_trait]
impl Evaluator for RegexMatchEvaluator {
    #[instrument(skip(self, actual, expected, config))]
    async fn evaluate(
        &self,
        actual: &str,
        expected: Option<&str>,
        config: &EvaluatorConfig,
    ) -> EvaluationResult {
        let pattern = config
            .get_param::<String>("pattern")
            .or_else(|| expected.map(String::from));

        let pattern = match pattern {
            Some(p) => p,
            None => return EvaluationResult::failure("No regex pattern provided"),
        };

        let regex = match regex::Regex::new(&pattern) {
            Ok(r) => r,
            Err(e) => return EvaluationResult::failure(format!("Invalid regex pattern: {}", e)),
        };

        let actual_normalized = if config.trim_whitespace {
            actual.trim()
        } else {
            actual
        };

        let is_match = regex.is_match(actual_normalized);
        let captures: Vec<String> = regex
            .captures_iter(actual_normalized)
            .flat_map(|cap| {
                cap.iter()
                    .filter_map(|m| m.map(|m| m.as_str().to_string()))
                    .collect::<Vec<_>>()
            })
            .collect();

        debug!(is_match = is_match, captures = ?captures, "Regex match evaluation");

        EvaluationResult::success_with_threshold(
            if is_match { 1.0 } else { 0.0 },
            config.pass_threshold,
        )
        .with_detail("match_type", "regex")
        .with_detail("pattern", pattern)
        .with_detail("is_match", is_match)
        .with_detail("captures", captures)
    }

    fn name(&self) -> &'static str {
        "regex_match"
    }
}

/// JSON schema validator evaluator.
pub struct JsonSchemaEvaluator;

#[async_trait]
impl Evaluator for JsonSchemaEvaluator {
    #[instrument(skip(self, actual, expected, config))]
    async fn evaluate(
        &self,
        actual: &str,
        expected: Option<&str>,
        config: &EvaluatorConfig,
    ) -> EvaluationResult {
        // Parse the actual output as JSON
        let actual_json: serde_json::Value = match serde_json::from_str(actual) {
            Ok(v) => v,
            Err(e) => {
                return EvaluationResult::failure(format!("Invalid JSON in output: {}", e))
                    .with_detail("parse_error", e.to_string());
            }
        };

        // Get the schema from config or expected
        let schema: serde_json::Value = if let Some(schema) = config.get_param("schema") {
            schema
        } else if let Some(exp) = expected {
            match serde_json::from_str(exp) {
                Ok(v) => v,
                Err(e) => {
                    return EvaluationResult::failure(format!("Invalid schema JSON: {}", e));
                }
            }
        } else {
            return EvaluationResult::failure("No JSON schema provided");
        };

        // Simple type checking (not full JSON Schema validation)
        let type_match = match (&actual_json, &schema) {
            (serde_json::Value::Object(a), serde_json::Value::Object(s)) => {
                // Check if all required keys from schema exist
                let required_keys: Vec<String> = s
                    .get("required")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let missing_keys: Vec<&String> = required_keys
                    .iter()
                    .filter(|k| !a.contains_key(*k))
                    .collect();

                (missing_keys.is_empty(), missing_keys.len())
            }
            (serde_json::Value::Array(_), serde_json::Value::Object(s)) => {
                let expected_type = s.get("type").and_then(|v| v.as_str());
                (expected_type == Some("array"), 0)
            }
            _ => (true, 0),
        };

        let score = if type_match.0 { 1.0 } else { 0.5 };

        debug!(type_match = type_match.0, "JSON schema evaluation");

        EvaluationResult::success_with_threshold(score, config.pass_threshold)
            .with_detail("match_type", "json_schema")
            .with_detail("valid_structure", type_match.0)
            .with_detail("parsed_json", actual_json)
    }

    fn name(&self) -> &'static str {
        "json_schema"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_exact_match() {
        let evaluator = ExactMatchEvaluator;
        let config = EvaluatorConfig::default();

        let result = evaluator.evaluate("hello", Some("hello"), &config).await;
        assert!(result.passed);
        assert_eq!(result.score, 1.0);

        let result = evaluator.evaluate("hello", Some("world"), &config).await;
        assert!(!result.passed);
        assert_eq!(result.score, 0.0);
    }

    #[tokio::test]
    async fn test_exact_match_case_insensitive() {
        let evaluator = ExactMatchEvaluator;
        let config = EvaluatorConfig {
            case_sensitive: false,
            ..Default::default()
        };

        let result = evaluator.evaluate("Hello", Some("hello"), &config).await;
        assert!(result.passed);
        assert_eq!(result.score, 1.0);
    }

    #[tokio::test]
    async fn test_fuzzy_match() {
        let evaluator = FuzzyMatchEvaluator;
        let config = EvaluatorConfig::default().set_param("threshold", 0.7);

        let result = evaluator.evaluate("hello", Some("hallo"), &config).await;
        assert!(result.score > 0.7);
    }

    #[tokio::test]
    async fn test_levenshtein_distance() {
        assert_eq!(FuzzyMatchEvaluator::levenshtein_distance("", ""), 0);
        assert_eq!(FuzzyMatchEvaluator::levenshtein_distance("abc", ""), 3);
        assert_eq!(FuzzyMatchEvaluator::levenshtein_distance("", "abc"), 3);
        assert_eq!(FuzzyMatchEvaluator::levenshtein_distance("abc", "abc"), 0);
        assert_eq!(FuzzyMatchEvaluator::levenshtein_distance("kitten", "sitting"), 3);
    }

    #[tokio::test]
    async fn test_numeric_tolerance() {
        let evaluator = NumericToleranceEvaluator;
        let config = EvaluatorConfig::default().set_param("tolerance", 0.1);

        let result = evaluator.evaluate("3.14", Some("3.14"), &config).await;
        assert!(result.passed);
        assert_eq!(result.score, 1.0);

        let result = evaluator.evaluate("3.15", Some("3.14"), &config).await;
        assert!(result.passed);

        let result = evaluator.evaluate("5.0", Some("3.14"), &config).await;
        assert!(!result.passed);
    }

    #[tokio::test]
    async fn test_contains() {
        let evaluator = ContainsEvaluator;
        let config = EvaluatorConfig::default()
            .set_param("required", vec!["hello", "world"])
            .set_param("forbidden", vec!["error"]);

        let result = evaluator
            .evaluate("hello beautiful world", None, &config)
            .await;
        assert!(result.passed);

        let result = evaluator
            .evaluate("hello beautiful world error", None, &config)
            .await;
        assert!(!result.passed);
    }

    #[tokio::test]
    async fn test_regex_match() {
        let evaluator = RegexMatchEvaluator;
        let config = EvaluatorConfig::default().set_param("pattern", r"\d{3}-\d{4}");

        let result = evaluator.evaluate("Call 555-1234", None, &config).await;
        assert!(result.passed);

        let result = evaluator.evaluate("No phone number", None, &config).await;
        assert!(!result.passed);
    }
}
