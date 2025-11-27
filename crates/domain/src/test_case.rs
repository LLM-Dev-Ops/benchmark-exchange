//! Test case types for benchmarks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Individual test case within a benchmark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input: TestInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_output: Option<ExpectedOutput>,
    pub evaluation_method: EvaluationMethod,
    pub weight: f64,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<DifficultyLevel>,
}

/// Test case input specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestInput {
    pub prompt_template: String,
    pub variables: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    pub few_shot_examples: Vec<FewShotExample>,
    pub input_format: InputFormat,
}

/// Few-shot example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewShotExample {
    pub input: String,
    pub output: String,
}

/// Input format specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum InputFormat {
    PlainText,
    Markdown,
    Json,
    Code { language: String },
    MultiModal { modalities: Vec<Modality> },
}

/// Modality for multi-modal inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    Text,
    Image,
    Audio,
    Video,
}

/// Expected output specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_output: Option<String>,
    pub acceptable_outputs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
    pub constraints: Vec<OutputConstraint>,
}

/// Output constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputConstraint {
    MaxLength { chars: usize },
    MinLength { chars: usize },
    ContainsAll { substrings: Vec<String> },
    ContainsNone { substrings: Vec<String> },
    MatchesRegex { pattern: String },
    ValidJson,
    ValidCode { language: String },
}

/// Difficulty classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DifficultyLevel {
    Easy,
    Medium,
    Hard,
    Expert,
}

/// Evaluation method
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvaluationMethod {
    ExactMatch,
    FuzzyMatch { threshold: f64 },
    SemanticSimilarity { model: String, threshold: f64 },
    RegexMatch { pattern: String },
    NumericComparison { tolerance: f64 },
    CodeExecution { runtime: String, test_cases: Vec<CodeTestCase> },
    LlmJudge { judge_prompt: String, judge_model: Option<String> },
    HumanEvaluation { rubric: String },
    Custom { evaluator_id: String, config: serde_json::Value },
}

/// Code test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeTestCase {
    pub input: String,
    pub expected_output: String,
    pub timeout_ms: u64,
}
