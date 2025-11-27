//! Benchmark definition types for the LLM Benchmark Exchange domain.

use crate::identifiers::{BenchmarkId, BenchmarkVersionId, UserId};
use crate::version::SemanticVersion;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

/// Top-level benchmark categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkCategory {
    Performance,
    Accuracy,
    Reliability,
    Safety,
    Cost,
    Capability,
}

impl BenchmarkCategory {
    pub fn all() -> &'static [BenchmarkCategory] {
        &[
            Self::Performance,
            Self::Accuracy,
            Self::Reliability,
            Self::Safety,
            Self::Cost,
            Self::Capability,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Performance => "Performance",
            Self::Accuracy => "Accuracy",
            Self::Reliability => "Reliability",
            Self::Safety => "Safety",
            Self::Cost => "Cost",
            Self::Capability => "Capability",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Performance => "Latency, throughput, tokens per second, time to first token",
            Self::Accuracy => "Task-specific correctness (QA, summarization, translation, coding)",
            Self::Reliability => "Consistency, hallucination rates, factual accuracy",
            Self::Safety => "Harmful content generation, jailbreak resistance, bias detection",
            Self::Cost => "Price per token, cost per task, cost-performance ratios",
            Self::Capability => "Context length, multi-modal support, function calling",
        }
    }
}

/// Subcategory for finer benchmark classification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BenchmarkSubcategory {
    pub parent: BenchmarkCategory,
    pub name: String,
    pub description: String,
}

/// Benchmark metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetadata {
    pub name: String,
    pub slug: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_description: Option<String>,
    pub tags: Vec<String>,
    pub license: LicenseType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation: Option<Citation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<Url>,
    pub maintainers: Vec<UserId>,
}

/// License types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseType {
    Apache2,
    MIT,
    #[serde(rename = "bsd_3_clause")]
    BSD3Clause,
    #[serde(rename = "cc_by_4_0")]
    CC_BY_4_0,
    #[serde(rename = "cc_by_sa_4_0")]
    CC_BY_SA_4_0,
    Custom(String),
}

/// Academic citation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub title: String,
    pub authors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub venue: Option<String>,
    pub year: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bibtex: Option<String>,
}

/// Benchmark lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkStatus {
    Draft,
    UnderReview,
    Active,
    Deprecated,
    Archived,
}

impl BenchmarkStatus {
    pub fn can_transition_to(&self, target: BenchmarkStatus) -> bool {
        matches!(
            (self, target),
            (Self::Draft, Self::UnderReview)
                | (Self::UnderReview, Self::Active)
                | (Self::UnderReview, Self::Draft)
                | (Self::Active, Self::Deprecated)
                | (Self::Deprecated, Self::Archived)
                | (Self::Deprecated, Self::Active)
        )
    }

    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Active | Self::Deprecated)
    }
}

/// Benchmark version lineage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkLineage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_version_id: Option<BenchmarkVersionId>,
    pub changelog: String,
    pub breaking_changes: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_notes: Option<String>,
}
