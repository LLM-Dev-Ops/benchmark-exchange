//! Publication Domain Types for LLM Benchmark Exchange
//!
//! This module defines domain entities for the Benchmark Publication Agent.
//! The Publication Agent is responsible for publishing, validating, and normalizing
//! LLM benchmark results as authoritative, reproducible artifacts.
//!
//! ## Agent Classification
//!
//! This is a BENCHMARK PUBLICATION agent that:
//! - MAY: Publish benchmark results, normalize metrics, validate methodology, emit confidence signals
//! - MUST NOT: Execute benchmarks, invoke models, rank outcomes, enforce policies
//!
//! ## Persistence
//!
//! ALL persistence is handled via ruvector-service. This agent NEVER connects
//! directly to any database or executes SQL.

use crate::identifiers::{BenchmarkId, SubmissionId, UserId};
use crate::version::SemanticVersion;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// =============================================================================
// Publication Identifier
// =============================================================================

/// Unique identifier for publications (UUID v7 for time-ordering)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PublicationId(Uuid);

impl PublicationId {
    /// Create a new ID with a time-ordered UUID v7
    #[inline]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Create an ID from an existing UUID
    #[inline]
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get a reference to the underlying UUID
    #[inline]
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Convert to the underlying UUID
    #[inline]
    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for PublicationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PublicationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for PublicationId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<PublicationId> for Uuid {
    fn from(id: PublicationId) -> Self {
        id.0
    }
}

impl std::str::FromStr for PublicationId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

// =============================================================================
// Publication Status
// =============================================================================

/// Publication lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationStatus {
    /// Publication is being prepared/drafted
    Draft,
    /// Publication is pending validation
    PendingValidation,
    /// Publication has been validated and is being normalized
    Normalizing,
    /// Publication has been published and is active
    Published,
    /// Publication has been superseded by a newer version
    Superseded,
    /// Publication has been retracted (errors discovered)
    Retracted,
    /// Publication has been archived
    Archived,
}

impl PublicationStatus {
    /// Get the display name for the status
    pub fn display_name(&self) -> &'static str {
        match self {
            PublicationStatus::Draft => "Draft",
            PublicationStatus::PendingValidation => "Pending Validation",
            PublicationStatus::Normalizing => "Normalizing",
            PublicationStatus::Published => "Published",
            PublicationStatus::Superseded => "Superseded",
            PublicationStatus::Retracted => "Retracted",
            PublicationStatus::Archived => "Archived",
        }
    }

    /// Check if this is a terminal status
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PublicationStatus::Retracted | PublicationStatus::Archived
        )
    }

    /// Check if the publication is visible/active
    pub fn is_active(&self) -> bool {
        matches!(self, PublicationStatus::Published)
    }

    /// Valid transitions from this status
    pub fn valid_transitions(&self) -> Vec<PublicationStatus> {
        match self {
            PublicationStatus::Draft => vec![
                PublicationStatus::PendingValidation,
                PublicationStatus::Archived,
            ],
            PublicationStatus::PendingValidation => vec![
                PublicationStatus::Normalizing,
                PublicationStatus::Draft,
                PublicationStatus::Retracted,
            ],
            PublicationStatus::Normalizing => vec![
                PublicationStatus::Published,
                PublicationStatus::Draft,
                PublicationStatus::Retracted,
            ],
            PublicationStatus::Published => vec![
                PublicationStatus::Superseded,
                PublicationStatus::Retracted,
                PublicationStatus::Archived,
            ],
            PublicationStatus::Superseded => vec![
                PublicationStatus::Archived,
            ],
            PublicationStatus::Retracted => vec![
                PublicationStatus::Archived,
            ],
            PublicationStatus::Archived => vec![],
        }
    }

    /// Check if transition to target status is valid
    pub fn can_transition_to(&self, target: PublicationStatus) -> bool {
        self.valid_transitions().contains(&target)
    }
}

impl Default for PublicationStatus {
    fn default() -> Self {
        PublicationStatus::Draft
    }
}

// =============================================================================
// Decision Type (per constitution requirements)
// =============================================================================

/// Decision types for the Publication Agent
/// Per constitution: decision_type semantics must be explicitly defined
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationDecisionType {
    /// Publishing a new benchmark result
    BenchmarkPublish,
    /// Updating an existing publication
    BenchmarkUpdate,
    /// Validating a benchmark submission
    BenchmarkValidate,
    /// Normalizing benchmark metrics
    BenchmarkNormalize,
    /// Retracting a publication
    BenchmarkRetract,
    /// Inspecting publication metadata
    BenchmarkInspect,
}

impl PublicationDecisionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PublicationDecisionType::BenchmarkPublish => "benchmark_publish",
            PublicationDecisionType::BenchmarkUpdate => "benchmark_update",
            PublicationDecisionType::BenchmarkValidate => "benchmark_validate",
            PublicationDecisionType::BenchmarkNormalize => "benchmark_normalize",
            PublicationDecisionType::BenchmarkRetract => "benchmark_retract",
            PublicationDecisionType::BenchmarkInspect => "benchmark_inspect",
        }
    }
}

// =============================================================================
// Confidence Metrics (reproducibility, sample size, variance)
// =============================================================================

/// Confidence metrics for benchmark publication
/// Per constitution: confidence semantics must be explicitly defined
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationConfidence {
    /// Overall reproducibility score (0.0 - 1.0)
    /// Higher values indicate more reproducible results
    pub reproducibility_score: f64,

    /// Sample size used for benchmark execution
    pub sample_size: u32,

    /// Statistical variance in results
    pub variance: f64,

    /// Standard deviation of scores
    pub std_dev: Option<f64>,

    /// Confidence interval (95%)
    pub confidence_interval: Option<(f64, f64)>,

    /// Number of independent reproductions
    pub reproduction_count: u32,

    /// Coefficient of variation (std_dev / mean)
    pub coefficient_of_variation: Option<f64>,
}

impl PublicationConfidence {
    /// Create confidence metrics with minimum required fields
    pub fn new(reproducibility_score: f64, sample_size: u32, variance: f64) -> Self {
        Self {
            reproducibility_score: reproducibility_score.clamp(0.0, 1.0),
            sample_size,
            variance,
            std_dev: None,
            confidence_interval: None,
            reproduction_count: 1,
            coefficient_of_variation: None,
        }
    }

    /// Check if confidence meets minimum publication threshold
    pub fn meets_threshold(&self, min_reproducibility: f64, min_samples: u32) -> bool {
        self.reproducibility_score >= min_reproducibility && self.sample_size >= min_samples
    }

    /// Calculate confidence level category
    pub fn confidence_level(&self) -> ConfidenceLevel {
        match self.reproducibility_score {
            r if r >= 0.95 => ConfidenceLevel::VeryHigh,
            r if r >= 0.85 => ConfidenceLevel::High,
            r if r >= 0.70 => ConfidenceLevel::Medium,
            r if r >= 0.50 => ConfidenceLevel::Low,
            _ => ConfidenceLevel::VeryLow,
        }
    }
}

impl Default for PublicationConfidence {
    fn default() -> Self {
        Self::new(0.0, 0, 0.0)
    }
}

/// Confidence level categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    VeryHigh,
    High,
    Medium,
    Low,
    VeryLow,
}

// =============================================================================
// Constraints Applied (methodology, dataset scope, model version)
// =============================================================================

/// Constraints applied to the benchmark publication
/// Per constitution: constraints_applied semantics must be explicitly defined
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationConstraints {
    /// Methodology constraints applied
    pub methodology: MethodologyConstraints,

    /// Dataset scope constraints
    pub dataset_scope: DatasetScopeConstraints,

    /// Model version constraints
    pub model_version: ModelVersionConstraints,

    /// Additional custom constraints
    #[serde(default)]
    pub custom_constraints: HashMap<String, String>,
}

/// Methodology constraints for benchmark execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyConstraints {
    /// Evaluation framework used (e.g., "lm-eval", "helm", "custom")
    pub framework: String,

    /// Specific evaluation method (e.g., "zero-shot", "few-shot", "chain-of-thought")
    pub evaluation_method: String,

    /// Prompt template version/hash
    pub prompt_template_hash: Option<String>,

    /// Scoring methodology (e.g., "exact_match", "f1", "bleu")
    pub scoring_method: String,

    /// Whether results are normalized
    pub normalized: bool,

    /// Normalization method if applicable
    pub normalization_method: Option<String>,
}

/// Dataset scope constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetScopeConstraints {
    /// Dataset identifier/name
    pub dataset_id: String,

    /// Dataset version/hash
    pub dataset_version: String,

    /// Subset used (if applicable)
    pub subset: Option<String>,

    /// Number of examples used
    pub example_count: u32,

    /// Split used (train/validation/test)
    pub split: String,

    /// Whether dataset is publicly available
    pub publicly_available: bool,
}

/// Model version constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersionConstraints {
    /// Model provider (e.g., "openai", "anthropic", "meta")
    pub provider: String,

    /// Model name
    pub model_name: String,

    /// Model version/checkpoint
    pub version: String,

    /// Model size (parameters)
    pub parameter_count: Option<u64>,

    /// Quantization method if applicable
    pub quantization: Option<String>,

    /// Context window size
    pub context_window: Option<u32>,
}

impl Default for PublicationConstraints {
    fn default() -> Self {
        Self {
            methodology: MethodologyConstraints {
                framework: "unknown".to_string(),
                evaluation_method: "unknown".to_string(),
                prompt_template_hash: None,
                scoring_method: "unknown".to_string(),
                normalized: false,
                normalization_method: None,
            },
            dataset_scope: DatasetScopeConstraints {
                dataset_id: "unknown".to_string(),
                dataset_version: "unknown".to_string(),
                subset: None,
                example_count: 0,
                split: "test".to_string(),
                publicly_available: false,
            },
            model_version: ModelVersionConstraints {
                provider: "unknown".to_string(),
                model_name: "unknown".to_string(),
                version: "unknown".to_string(),
                parameter_count: None,
                quantization: None,
                context_window: None,
            },
            custom_constraints: HashMap::new(),
        }
    }
}

// =============================================================================
// Normalized Metrics
// =============================================================================

/// Normalized benchmark metrics for cross-model comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedMetrics {
    /// Primary aggregate score (0.0 - 1.0 or 0.0 - 100.0)
    pub aggregate_score: f64,

    /// Normalized score (always 0.0 - 1.0)
    pub normalized_score: f64,

    /// Individual metric scores
    pub metric_scores: HashMap<String, MetricValue>,

    /// Percentile rank among all submissions for this benchmark
    pub percentile_rank: Option<f64>,

    /// Z-score relative to benchmark mean
    pub z_score: Option<f64>,
}

/// Individual metric value with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    /// Raw value
    pub value: f64,

    /// Normalized value (0.0 - 1.0)
    pub normalized: f64,

    /// Unit of measurement
    pub unit: Option<String>,

    /// Higher is better flag
    pub higher_is_better: bool,

    /// Min/max range for normalization
    pub range: Option<(f64, f64)>,
}

// =============================================================================
// Publication Entity
// =============================================================================

/// Core publication entity for benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Publication {
    /// Unique publication identifier
    pub id: PublicationId,

    /// Reference to the benchmark definition
    pub benchmark_id: BenchmarkId,

    /// Reference to the original submission (if any)
    pub submission_id: Option<SubmissionId>,

    /// Publication status
    pub status: PublicationStatus,

    /// Publication version (supports multiple versions of same result)
    pub version: SemanticVersion,

    /// Model information
    pub model_provider: String,
    pub model_name: String,
    pub model_version: String,

    /// Normalized metrics
    pub metrics: NormalizedMetrics,

    /// Confidence assessment
    pub confidence: PublicationConfidence,

    /// Constraints applied
    pub constraints: PublicationConstraints,

    /// Publisher information
    pub published_by: UserId,
    pub organization_id: Option<String>,

    /// Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,

    /// Citation information
    pub citation: Option<PublicationCitation>,

    /// Tags for categorization
    pub tags: Vec<String>,

    /// Whether this is the latest version
    pub is_latest: bool,

    /// Previous version ID (if this is an update)
    pub previous_version_id: Option<PublicationId>,
}

/// Citation information for publications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationCitation {
    /// DOI if available
    pub doi: Option<String>,

    /// ArXiv ID if available
    pub arxiv_id: Option<String>,

    /// BibTeX entry
    pub bibtex: Option<String>,

    /// Plain text citation
    pub plain_text: String,
}

// =============================================================================
// Publication Events
// =============================================================================

/// Publication domain events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PublicationEvent {
    /// New publication created
    PublicationCreated {
        publication_id: PublicationId,
        benchmark_id: BenchmarkId,
        model_name: String,
        model_provider: String,
        created_by: UserId,
    },

    /// Publication validated
    PublicationValidated {
        publication_id: PublicationId,
        validation_passed: bool,
        validation_errors: Vec<String>,
    },

    /// Publication normalized
    PublicationNormalized {
        publication_id: PublicationId,
        normalized_score: f64,
        percentile_rank: Option<f64>,
    },

    /// Publication published (made active)
    PublicationPublished {
        publication_id: PublicationId,
        published_at: DateTime<Utc>,
    },

    /// Publication updated
    PublicationUpdated {
        publication_id: PublicationId,
        new_version: SemanticVersion,
        updated_by: UserId,
        changelog: String,
    },

    /// Publication retracted
    PublicationRetracted {
        publication_id: PublicationId,
        reason: String,
        retracted_by: UserId,
    },

    /// Publication superseded
    PublicationSuperseded {
        publication_id: PublicationId,
        successor_id: PublicationId,
    },

    /// Publication archived
    PublicationArchived {
        publication_id: PublicationId,
        archived_by: UserId,
    },
}

// =============================================================================
// Agent Identity (Phase 7 Hardening)
// =============================================================================

/// Agent identity metadata for DecisionEvent traceability
/// Per Phase 7: Every DecisionEvent MUST include full agent identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    /// Source agent identifier (e.g., "benchmark-publication-agent")
    pub source_agent: String,

    /// Domain this agent operates in
    pub domain: String,

    /// Phase of agent execution (always "phase7" for Phase 7 agents)
    pub phase: String,

    /// Architectural layer (always "layer2" for Layer 2 agents)
    pub layer: String,

    /// Semantic version of the agent
    pub agent_version: String,
}

impl Default for AgentIdentity {
    fn default() -> Self {
        Self {
            source_agent: std::env::var("AGENT_NAME")
                .unwrap_or_else(|_| "benchmark-publication-agent".to_string()),
            domain: std::env::var("AGENT_DOMAIN")
                .unwrap_or_else(|_| "benchmark".to_string()),
            phase: std::env::var("AGENT_PHASE")
                .unwrap_or_else(|_| "phase7".to_string()),
            layer: std::env::var("AGENT_LAYER")
                .unwrap_or_else(|_| "layer2".to_string()),
            agent_version: std::env::var("AGENT_VERSION")
                .unwrap_or_else(|_| "1.0.0".to_string()),
        }
    }
}

impl AgentIdentity {
    /// Create identity for benchmark publication operations
    pub fn for_publication() -> Self {
        Self::default()
    }

    /// Validate that all required identity fields are present
    pub fn validate(&self) -> Result<(), String> {
        if self.source_agent.is_empty() {
            return Err("source_agent is required".to_string());
        }
        if self.domain.is_empty() {
            return Err("domain is required".to_string());
        }
        if self.phase.is_empty() {
            return Err("phase is required".to_string());
        }
        if self.layer.is_empty() {
            return Err("layer is required".to_string());
        }
        if self.agent_version.is_empty() {
            return Err("agent_version is required".to_string());
        }
        Ok(())
    }
}

// =============================================================================
// Phase 7 Signal Types
// =============================================================================

/// Phase 7 signal container for intelligence signals (NOT decisions)
/// Per Phase 7: Agents emit signals, NOT conclusions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Phase7Signals {
    /// Hypothesis signal - represents a testable hypothesis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hypothesis_signal: Option<HypothesisSignal>,

    /// Simulation outcome signal - results from simulation/prediction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simulation_outcome_signal: Option<SimulationOutcomeSignal>,

    /// Confidence delta signal - change in confidence from previous state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_delta_signal: Option<ConfidenceDeltaSignal>,
}

/// Hypothesis signal for benchmark decisions
/// Per Phase 7: Represents intelligence INPUT, not outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisSignal {
    /// Unique hypothesis identifier
    pub hypothesis_id: String,

    /// Human-readable hypothesis statement
    pub statement: String,

    /// Prior probability (before evidence) - 0.0 to 1.0
    pub prior_probability: f64,

    /// Posterior probability (after evidence) - 0.0 to 1.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub posterior_probability: Option<f64>,

    /// Evidence references (run IDs, telemetry IDs, dataset refs)
    pub evidence_refs: Vec<String>,

    /// Timestamp of hypothesis generation
    pub timestamp: DateTime<Utc>,
}

/// Simulation outcome signal
/// Per Phase 7: Represents simulation results, not decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationOutcomeSignal {
    /// Simulation run identifier
    pub simulation_id: String,

    /// Number of simulation iterations
    pub iterations: u32,

    /// Predicted outcome value
    pub predicted_value: f64,

    /// Confidence in prediction - 0.0 to 1.0
    pub confidence: f64,

    /// Prediction confidence interval (lower, upper)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_interval: Option<(f64, f64)>,

    /// Evidence references
    pub evidence_refs: Vec<String>,

    /// Timestamp of simulation
    pub timestamp: DateTime<Utc>,
}

/// Confidence delta signal - tracks confidence changes
/// Per Phase 7: Represents change signal, not final state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceDeltaSignal {
    /// Previous confidence value (0.0 to 1.0)
    pub previous_confidence: f64,

    /// Current confidence value (0.0 to 1.0)
    pub current_confidence: f64,

    /// Absolute delta (current - previous)
    pub delta: f64,

    /// Reason for confidence change
    pub reason: String,

    /// Evidence references supporting the change
    pub evidence_refs: Vec<String>,

    /// Timestamp of delta calculation
    pub timestamp: DateTime<Utc>,
}

impl Phase7Signals {
    /// Create empty signals container
    pub fn new() -> Self {
        Self::default()
    }

    /// Add hypothesis signal
    pub fn with_hypothesis(mut self, signal: HypothesisSignal) -> Self {
        self.hypothesis_signal = Some(signal);
        self
    }

    /// Add simulation outcome signal
    pub fn with_simulation_outcome(mut self, signal: SimulationOutcomeSignal) -> Self {
        self.simulation_outcome_signal = Some(signal);
        self
    }

    /// Add confidence delta signal
    pub fn with_confidence_delta(mut self, signal: ConfidenceDeltaSignal) -> Self {
        self.confidence_delta_signal = Some(signal);
        self
    }

    /// Check if at least one signal is present (Phase 7 requirement)
    pub fn has_signal(&self) -> bool {
        self.hypothesis_signal.is_some()
            || self.simulation_outcome_signal.is_some()
            || self.confidence_delta_signal.is_some()
    }
}

// =============================================================================
// DecisionEvent (per constitution requirements + Phase 7 hardening)
// =============================================================================

/// DecisionEvent for ruvector-service persistence
/// Per constitution: MUST emit exactly ONE DecisionEvent per invocation
/// Per Phase 7: MUST include agent identity and at least one signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEvent {
    /// Agent identifier (legacy, kept for backward compatibility)
    pub agent_id: String,

    /// Agent version (legacy, kept for backward compatibility)
    pub agent_version: String,

    /// Full agent identity (Phase 7 requirement)
    pub agent_identity: AgentIdentity,

    /// Decision type (from PublicationDecisionType)
    pub decision_type: String,

    /// Hash of input data for reproducibility
    pub inputs_hash: String,

    /// Decision outputs
    pub outputs: DecisionOutputs,

    /// Confidence metrics (reproducibility / reliability)
    pub confidence: PublicationConfidence,

    /// Constraints applied during decision
    pub constraints_applied: PublicationConstraints,

    /// Execution reference (correlation ID)
    pub execution_ref: String,

    /// Timestamp (UTC)
    pub timestamp: DateTime<Utc>,

    /// Phase 7 signals (REQUIRED for Phase 7 agents)
    #[serde(default)]
    pub signals: Phase7Signals,
}

/// Decision outputs structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOutputs {
    /// Publication ID if created/updated
    pub publication_id: Option<PublicationId>,

    /// Result status
    pub status: String,

    /// Normalized metrics if applicable
    pub normalized_metrics: Option<NormalizedMetrics>,

    /// Validation results if applicable
    pub validation_results: Option<ValidationResults>,

    /// Additional output data
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Validation results from benchmark validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResults {
    /// Whether validation passed
    pub passed: bool,

    /// List of validation errors
    pub errors: Vec<ValidationError>,

    /// List of validation warnings
    pub warnings: Vec<ValidationWarning>,

    /// Validation score (0.0 - 1.0)
    pub score: f64,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
}

impl DecisionEvent {
    /// Agent ID constant for the Benchmark Publication Agent
    pub const AGENT_ID: &'static str = "benchmark-publication-agent";

    /// Current agent version
    pub const AGENT_VERSION: &'static str = "1.0.0";

    /// Create a new DecisionEvent with Phase 7 compliance
    pub fn new(
        decision_type: PublicationDecisionType,
        inputs_hash: String,
        outputs: DecisionOutputs,
        confidence: PublicationConfidence,
        constraints_applied: PublicationConstraints,
        execution_ref: String,
    ) -> Self {
        let agent_identity = AgentIdentity::for_publication();
        Self {
            agent_id: agent_identity.source_agent.clone(),
            agent_version: agent_identity.agent_version.clone(),
            agent_identity,
            decision_type: decision_type.as_str().to_string(),
            inputs_hash,
            outputs,
            confidence,
            constraints_applied,
            execution_ref,
            timestamp: Utc::now(),
            signals: Phase7Signals::default(),
        }
    }

    /// Create a new DecisionEvent with custom agent identity
    pub fn with_identity(
        agent_identity: AgentIdentity,
        decision_type: PublicationDecisionType,
        inputs_hash: String,
        outputs: DecisionOutputs,
        confidence: PublicationConfidence,
        constraints_applied: PublicationConstraints,
        execution_ref: String,
    ) -> Self {
        Self {
            agent_id: agent_identity.source_agent.clone(),
            agent_version: agent_identity.agent_version.clone(),
            agent_identity,
            decision_type: decision_type.as_str().to_string(),
            inputs_hash,
            outputs,
            confidence,
            constraints_applied,
            execution_ref,
            timestamp: Utc::now(),
            signals: Phase7Signals::default(),
        }
    }

    /// Add Phase 7 signals to the event
    pub fn with_signals(mut self, signals: Phase7Signals) -> Self {
        self.signals = signals;
        self
    }

    /// Add a hypothesis signal
    pub fn with_hypothesis_signal(mut self, signal: HypothesisSignal) -> Self {
        self.signals.hypothesis_signal = Some(signal);
        self
    }

    /// Add a simulation outcome signal
    pub fn with_simulation_signal(mut self, signal: SimulationOutcomeSignal) -> Self {
        self.signals.simulation_outcome_signal = Some(signal);
        self
    }

    /// Add a confidence delta signal
    pub fn with_confidence_delta_signal(mut self, signal: ConfidenceDeltaSignal) -> Self {
        self.signals.confidence_delta_signal = Some(signal);
        self
    }

    /// Validate Phase 7 compliance
    pub fn validate_phase7(&self) -> Result<(), String> {
        // Validate agent identity
        self.agent_identity.validate()?;

        // Phase 7 agents SHOULD emit at least one signal
        // This is a warning, not an error, to maintain backward compatibility
        if !self.signals.has_signal() {
            tracing::warn!(
                agent_id = %self.agent_id,
                decision_type = %self.decision_type,
                "DecisionEvent missing Phase 7 signals"
            );
        }

        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publication_id_creation() {
        let id = PublicationId::new();
        assert_ne!(id.to_string(), "");
    }

    #[test]
    fn test_publication_status_transitions() {
        let draft = PublicationStatus::Draft;
        assert!(draft.can_transition_to(PublicationStatus::PendingValidation));
        assert!(!draft.can_transition_to(PublicationStatus::Published));

        let published = PublicationStatus::Published;
        assert!(published.can_transition_to(PublicationStatus::Superseded));
        assert!(published.can_transition_to(PublicationStatus::Retracted));
    }

    #[test]
    fn test_confidence_metrics() {
        let confidence = PublicationConfidence::new(0.92, 1000, 0.05);
        assert_eq!(confidence.confidence_level(), ConfidenceLevel::High);
        assert!(confidence.meets_threshold(0.85, 500));
    }

    #[test]
    fn test_decision_event_creation() {
        let outputs = DecisionOutputs {
            publication_id: Some(PublicationId::new()),
            status: "success".to_string(),
            normalized_metrics: None,
            validation_results: None,
            metadata: HashMap::new(),
        };

        let event = DecisionEvent::new(
            PublicationDecisionType::BenchmarkPublish,
            "abc123".to_string(),
            outputs,
            PublicationConfidence::default(),
            PublicationConstraints::default(),
            "exec-ref-123".to_string(),
        );

        assert_eq!(event.agent_id, DecisionEvent::AGENT_ID);
        assert_eq!(event.decision_type, "benchmark_publish");
        // Phase 7: Verify agent identity is populated
        assert!(!event.agent_identity.source_agent.is_empty());
        assert!(!event.agent_identity.phase.is_empty());
        assert!(!event.agent_identity.layer.is_empty());
    }

    #[test]
    fn test_phase7_signals() {
        let hypothesis = HypothesisSignal {
            hypothesis_id: "hyp-001".to_string(),
            statement: "Model X outperforms Y on task Z".to_string(),
            prior_probability: 0.6,
            posterior_probability: Some(0.85),
            evidence_refs: vec!["run-123".to_string()],
            timestamp: Utc::now(),
        };

        let signals = Phase7Signals::new()
            .with_hypothesis(hypothesis);

        assert!(signals.has_signal());
        assert!(signals.hypothesis_signal.is_some());
        assert!(signals.simulation_outcome_signal.is_none());
        assert!(signals.confidence_delta_signal.is_none());
    }

    #[test]
    fn test_confidence_delta_signal() {
        let delta = ConfidenceDeltaSignal {
            previous_confidence: 0.75,
            current_confidence: 0.85,
            delta: 0.10,
            reason: "Additional reproductions".to_string(),
            evidence_refs: vec!["repro-456".to_string()],
            timestamp: Utc::now(),
        };

        let signals = Phase7Signals::new()
            .with_confidence_delta(delta);

        assert!(signals.has_signal());
        assert!(signals.confidence_delta_signal.is_some());
        assert_eq!(signals.confidence_delta_signal.as_ref().unwrap().delta, 0.10);
    }

    #[test]
    fn test_agent_identity_validation() {
        let identity = AgentIdentity::default();
        assert!(identity.validate().is_ok());

        let empty_identity = AgentIdentity {
            source_agent: "".to_string(),
            domain: "benchmark".to_string(),
            phase: "phase7".to_string(),
            layer: "layer2".to_string(),
            agent_version: "1.0.0".to_string(),
        };
        assert!(empty_identity.validate().is_err());
    }
}
