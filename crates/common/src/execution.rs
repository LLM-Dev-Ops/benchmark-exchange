//! Agentics Execution Span Infrastructure
//!
//! This module implements the Foundational Execution Unit contract for the
//! benchmark-exchange repository. It provides structured, hierarchical execution
//! spans that integrate into an ExecutionGraph produced by a Core orchestrator.
//!
//! ## Span Hierarchy
//!
//! ```text
//! Core (external caller)
//!   └─ Repo span (benchmark-exchange)
//!       ├─ Agent span (BenchmarkAgent)
//!       ├─ Agent span (PublicationAgent)
//!       └─ ...
//! ```
//!
//! ## Invariants
//!
//! - Every externally-invoked operation MUST have a `parent_span_id`
//! - A repo-level span is created on entry
//! - Every agent that executes logic MUST emit its own span
//! - Execution MUST NOT succeed if no agent spans were emitted
//! - On failure, all emitted spans are still returned
//! - Spans are append-only and causally ordered via `parent_span_id`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// The repository name used in all spans emitted by this crate.
pub const REPO_NAME: &str = "benchmark-exchange";

// =============================================================================
// Span Status
// =============================================================================

/// Status of an execution span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpanStatus {
    Running,
    Completed,
    Failed,
}

impl fmt::Display for SpanStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpanStatus::Running => write!(f, "RUNNING"),
            SpanStatus::Completed => write!(f, "COMPLETED"),
            SpanStatus::Failed => write!(f, "FAILED"),
        }
    }
}

// =============================================================================
// Span Type
// =============================================================================

/// Type discriminator for spans in the hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanType {
    Repo,
    Agent,
}

// =============================================================================
// Artifact
// =============================================================================

/// An artifact produced by an agent, attached to the agent's span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Unique identifier for this artifact.
    pub artifact_id: Uuid,
    /// Type descriptor (e.g., "benchmark_created", "publication_dto", "score_result").
    pub artifact_type: String,
    /// Stable reference: entity ID, URI, hash, or filename.
    pub stable_ref: String,
    /// Optional SHA256 hash of the artifact content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    /// Timestamp when the artifact was created.
    pub created_at: DateTime<Utc>,
    /// Optional inline content for small artifacts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
}

impl Artifact {
    /// Create a new artifact with the given type and stable reference.
    pub fn new(artifact_type: impl Into<String>, stable_ref: impl Into<String>) -> Self {
        Self {
            artifact_id: Uuid::new_v4(),
            artifact_type: artifact_type.into(),
            stable_ref: stable_ref.into(),
            content_hash: None,
            created_at: Utc::now(),
            content: None,
        }
    }

    /// Attach a content hash to this artifact.
    pub fn with_hash(mut self, hash: impl Into<String>) -> Self {
        self.content_hash = Some(hash.into());
        self
    }

    /// Attach inline content to this artifact.
    pub fn with_content(mut self, content: serde_json::Value) -> Self {
        self.content = Some(content);
        self
    }
}

// =============================================================================
// Execution Span
// =============================================================================

/// A single execution span in the causal tree.
///
/// Spans are either repo-level (one per entry-point invocation) or agent-level
/// (one per agent that executes logic). Agent spans are children of the repo span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSpan {
    /// Unique identifier for this span.
    pub span_id: Uuid,
    /// ID of the parent span (Core span for repo, repo span for agents).
    pub parent_span_id: Uuid,
    /// Whether this is a repo-level or agent-level span.
    pub span_type: SpanType,
    /// Repository name (always "benchmark-exchange").
    pub repo_name: String,
    /// Agent name (set only for SpanType::Agent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    /// Current status of this span.
    pub status: SpanStatus,
    /// When this span started.
    pub start_time: DateTime<Utc>,
    /// When this span ended (None if still running).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,
    /// Error message if the span failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Artifacts produced during this span's execution.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<Artifact>,
    /// Additional metadata key-value pairs.
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

// =============================================================================
// Execution Result (Output Contract)
// =============================================================================

/// The serializable output returned from this repository.
///
/// Contains the repo-level span and all nested agent-level spans.
/// This is the JSON-serializable execution graph fragment produced by
/// this Foundational Execution Unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// The execution ID linking all spans in this invocation.
    pub execution_id: Uuid,
    /// The repo-level span for benchmark-exchange.
    pub repo_span: ExecutionSpan,
    /// All agent-level spans nested under the repo span.
    pub agent_spans: Vec<ExecutionSpan>,
}

// =============================================================================
// Execution Error
// =============================================================================

/// Errors specific to execution graph operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ExecutionError {
    /// No agent spans were emitted during execution.
    #[error("No agent spans were emitted during execution")]
    NoAgentSpans,
    /// parent_span_id was required but not provided.
    #[error("parent_span_id is required but was not provided")]
    MissingParentSpanId,
}

// =============================================================================
// Execution Graph (Internal)
// =============================================================================

/// Internal append-only span store shared across clones of ExecutionContext.
#[derive(Debug, Default)]
struct ExecutionGraphInner {
    spans: Vec<ExecutionSpan>,
}

// =============================================================================
// Execution Context
// =============================================================================

/// The execution context carried through every operation in an agentics-managed
/// invocation.
///
/// Created once at the entry point (REST middleware, gRPC interceptor, worker
/// handler, or CLI entrypoint). Cloning this struct shares the underlying span
/// store via `Arc<Mutex<...>>`, so multiple services can append to the same graph.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// The execution ID from the Core orchestrator.
    pub execution_id: Uuid,
    /// The parent span ID provided by the caller (Core-level span).
    pub parent_span_id: Uuid,
    /// The repo-level span ID created on entry.
    pub repo_span_id: Uuid,
    /// Shared, append-only span store.
    graph: Arc<Mutex<ExecutionGraphInner>>,
}

impl ExecutionContext {
    /// Create a new execution context and immediately create the repo-level span.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - The execution ID from the Core orchestrator.
    /// * `parent_span_id` - The span ID of the Core-level span (the caller).
    pub fn new(execution_id: Uuid, parent_span_id: Uuid) -> Self {
        let repo_span_id = Uuid::new_v4();
        let repo_span = ExecutionSpan {
            span_id: repo_span_id,
            parent_span_id,
            span_type: SpanType::Repo,
            repo_name: REPO_NAME.to_string(),
            agent_name: None,
            status: SpanStatus::Running,
            start_time: Utc::now(),
            end_time: None,
            error_message: None,
            artifacts: Vec::new(),
            metadata: serde_json::Map::new(),
        };

        let graph = ExecutionGraphInner {
            spans: vec![repo_span],
        };

        Self {
            execution_id,
            parent_span_id,
            repo_span_id,
            graph: Arc::new(Mutex::new(graph)),
        }
    }

    /// Begin an agent-level span. Returns the span ID of the new agent span.
    ///
    /// The agent span's `parent_span_id` is set to `self.repo_span_id`.
    pub fn begin_agent_span(&self, agent_name: &str) -> Uuid {
        let span_id = Uuid::new_v4();
        let span = ExecutionSpan {
            span_id,
            parent_span_id: self.repo_span_id,
            span_type: SpanType::Agent,
            repo_name: REPO_NAME.to_string(),
            agent_name: Some(agent_name.to_string()),
            status: SpanStatus::Running,
            start_time: Utc::now(),
            end_time: None,
            error_message: None,
            artifacts: Vec::new(),
            metadata: serde_json::Map::new(),
        };

        let mut graph = self.graph.lock().expect("execution graph lock poisoned");
        graph.spans.push(span);
        span_id
    }

    /// End an agent span with success status.
    pub fn end_agent_span(&self, span_id: Uuid) {
        let mut graph = self.graph.lock().expect("execution graph lock poisoned");
        if let Some(span) = graph.spans.iter_mut().find(|s| s.span_id == span_id) {
            span.status = SpanStatus::Completed;
            span.end_time = Some(Utc::now());
        }
    }

    /// End an agent span with failure status and an error message.
    pub fn fail_agent_span(&self, span_id: Uuid, error: &str) {
        let mut graph = self.graph.lock().expect("execution graph lock poisoned");
        if let Some(span) = graph.spans.iter_mut().find(|s| s.span_id == span_id) {
            span.status = SpanStatus::Failed;
            span.end_time = Some(Utc::now());
            span.error_message = Some(error.to_string());
        }
    }

    /// Attach an artifact to a specific agent span.
    pub fn attach_artifact(&self, span_id: Uuid, artifact: Artifact) {
        let mut graph = self.graph.lock().expect("execution graph lock poisoned");
        if let Some(span) = graph.spans.iter_mut().find(|s| s.span_id == span_id) {
            span.artifacts.push(artifact);
        }
    }

    /// Create an `AgentSpanGuard` that automatically manages the lifecycle
    /// of an agent span. The span is failed on drop unless `.complete()` is called.
    pub fn agent_guard(&self, agent_name: &str) -> AgentSpanGuard {
        let span_id = self.begin_agent_span(agent_name);
        AgentSpanGuard {
            exec_ctx: self.clone(),
            span_id,
            completed: false,
        }
    }

    /// Finalize the execution: marks the repo span as completed or failed,
    /// and returns the `ExecutionResult`.
    ///
    /// # Invariant Enforcement
    ///
    /// - Returns `Err(ExecutionError::NoAgentSpans)` if no agent spans were emitted.
    /// - If any agent span has status `Failed`, the repo span is marked `Failed`.
    pub fn finalize(self) -> Result<ExecutionResult, ExecutionError> {
        let mut graph = self.graph.lock().expect("execution graph lock poisoned");

        // Separate repo span from agent spans
        let agent_spans: Vec<&ExecutionSpan> = graph
            .spans
            .iter()
            .filter(|s| s.span_type == SpanType::Agent)
            .collect();

        // Invariant: at least one agent span must exist
        if agent_spans.is_empty() {
            // Mark repo span as failed before returning error
            if let Some(repo) = graph.spans.iter_mut().find(|s| s.span_type == SpanType::Repo) {
                repo.status = SpanStatus::Failed;
                repo.end_time = Some(Utc::now());
                repo.error_message = Some("No agent spans were emitted during execution".to_string());
            }
            return Err(ExecutionError::NoAgentSpans);
        }

        // If any agent span failed, repo span is failed
        let any_failed = agent_spans.iter().any(|s| s.status == SpanStatus::Failed);

        // Finalize repo span
        if let Some(repo) = graph.spans.iter_mut().find(|s| s.span_type == SpanType::Repo) {
            repo.status = if any_failed {
                SpanStatus::Failed
            } else {
                SpanStatus::Completed
            };
            repo.end_time = Some(Utc::now());
        }

        // Build result
        let repo_span = graph
            .spans
            .iter()
            .find(|s| s.span_type == SpanType::Repo)
            .cloned()
            .expect("repo span must exist");

        let agent_spans: Vec<ExecutionSpan> = graph
            .spans
            .iter()
            .filter(|s| s.span_type == SpanType::Agent)
            .cloned()
            .collect();

        Ok(ExecutionResult {
            execution_id: self.execution_id,
            repo_span,
            agent_spans,
        })
    }

    /// Finalize as failed with a specific error reason.
    ///
    /// Unlike `finalize()`, this always succeeds and returns all emitted spans
    /// regardless of invariant violations. The repo span is marked `FAILED`.
    pub fn finalize_failed(self, error: &str) -> ExecutionResult {
        let mut graph = self.graph.lock().expect("execution graph lock poisoned");

        // Mark repo span as failed
        if let Some(repo) = graph.spans.iter_mut().find(|s| s.span_type == SpanType::Repo) {
            repo.status = SpanStatus::Failed;
            repo.end_time = Some(Utc::now());
            repo.error_message = Some(error.to_string());
        }

        let repo_span = graph
            .spans
            .iter()
            .find(|s| s.span_type == SpanType::Repo)
            .cloned()
            .expect("repo span must exist");

        let agent_spans: Vec<ExecutionSpan> = graph
            .spans
            .iter()
            .filter(|s| s.span_type == SpanType::Agent)
            .cloned()
            .collect();

        ExecutionResult {
            execution_id: self.execution_id,
            repo_span,
            agent_spans,
        }
    }
}

// =============================================================================
// Agent Span Guard (RAII)
// =============================================================================

/// RAII guard for agent-level execution spans.
///
/// Automatically marks the span as `FAILED` on drop unless `.complete()` is
/// called. This ensures that early returns via `?` properly mark spans as failed.
pub struct AgentSpanGuard {
    exec_ctx: ExecutionContext,
    span_id: Uuid,
    completed: bool,
}

impl AgentSpanGuard {
    /// Get the span ID of this agent span.
    pub fn span_id(&self) -> Uuid {
        self.span_id
    }

    /// Attach an artifact to this agent's span.
    pub fn attach_artifact(&self, artifact: Artifact) {
        self.exec_ctx.attach_artifact(self.span_id, artifact);
    }

    /// Mark this agent span as successfully completed.
    ///
    /// Must be called before drop to prevent the span from being marked as failed.
    pub fn complete(mut self) {
        self.completed = true;
        self.exec_ctx.end_agent_span(self.span_id);
    }
}

impl Drop for AgentSpanGuard {
    fn drop(&mut self) {
        if !self.completed {
            self.exec_ctx
                .fail_agent_span(self.span_id, "Agent span dropped without completion");
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_context_creates_repo_span() {
        let exec_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let ctx = ExecutionContext::new(exec_id, parent_id);

        assert_eq!(ctx.execution_id, exec_id);
        assert_eq!(ctx.parent_span_id, parent_id);

        // Verify repo span was created
        let graph = ctx.graph.lock().unwrap();
        assert_eq!(graph.spans.len(), 1);
        assert_eq!(graph.spans[0].span_type, SpanType::Repo);
        assert_eq!(graph.spans[0].repo_name, REPO_NAME);
        assert_eq!(graph.spans[0].parent_span_id, parent_id);
        assert_eq!(graph.spans[0].status, SpanStatus::Running);
    }

    #[test]
    fn test_agent_span_lifecycle() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());

        let span_id = ctx.begin_agent_span("TestAgent");

        // Verify agent span was created
        {
            let graph = ctx.graph.lock().unwrap();
            assert_eq!(graph.spans.len(), 2);
            let agent = &graph.spans[1];
            assert_eq!(agent.span_type, SpanType::Agent);
            assert_eq!(agent.agent_name.as_deref(), Some("TestAgent"));
            assert_eq!(agent.parent_span_id, ctx.repo_span_id);
            assert_eq!(agent.status, SpanStatus::Running);
        }

        ctx.end_agent_span(span_id);

        // Verify it's completed
        let graph = ctx.graph.lock().unwrap();
        let agent = &graph.spans[1];
        assert_eq!(agent.status, SpanStatus::Completed);
        assert!(agent.end_time.is_some());
    }

    #[test]
    fn test_agent_span_failure() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());
        let span_id = ctx.begin_agent_span("FailAgent");

        ctx.fail_agent_span(span_id, "something went wrong");

        let graph = ctx.graph.lock().unwrap();
        let agent = &graph.spans[1];
        assert_eq!(agent.status, SpanStatus::Failed);
        assert_eq!(agent.error_message.as_deref(), Some("something went wrong"));
    }

    #[test]
    fn test_artifact_attachment() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());
        let span_id = ctx.begin_agent_span("ArtifactAgent");

        let artifact = Artifact::new("test_result", "entity-123")
            .with_hash("abc123")
            .with_content(serde_json::json!({"score": 0.95}));

        ctx.attach_artifact(span_id, artifact);

        let graph = ctx.graph.lock().unwrap();
        let agent = &graph.spans[1];
        assert_eq!(agent.artifacts.len(), 1);
        assert_eq!(agent.artifacts[0].artifact_type, "test_result");
        assert_eq!(agent.artifacts[0].stable_ref, "entity-123");
        assert_eq!(agent.artifacts[0].content_hash.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_guard_complete() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());

        {
            let guard = ctx.agent_guard("GuardAgent");
            guard.attach_artifact(Artifact::new("output", "ref-1"));
            guard.complete();
        }

        let graph = ctx.graph.lock().unwrap();
        let agent = &graph.spans[1];
        assert_eq!(agent.status, SpanStatus::Completed);
        assert_eq!(agent.artifacts.len(), 1);
    }

    #[test]
    fn test_guard_drop_without_complete_marks_failed() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());

        {
            let _guard = ctx.agent_guard("DropAgent");
            // guard drops without complete()
        }

        let graph = ctx.graph.lock().unwrap();
        let agent = &graph.spans[1];
        assert_eq!(agent.status, SpanStatus::Failed);
        assert!(agent.error_message.is_some());
    }

    #[test]
    fn test_finalize_success() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());

        let guard = ctx.agent_guard("SuccessAgent");
        guard.complete();

        let result = ctx.finalize().expect("finalize should succeed");

        assert_eq!(result.repo_span.status, SpanStatus::Completed);
        assert_eq!(result.agent_spans.len(), 1);
        assert_eq!(result.agent_spans[0].status, SpanStatus::Completed);
        assert_eq!(
            result.agent_spans[0].agent_name.as_deref(),
            Some("SuccessAgent")
        );
    }

    #[test]
    fn test_finalize_fails_without_agent_spans() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());

        let err = ctx.finalize().unwrap_err();
        assert!(matches!(err, ExecutionError::NoAgentSpans));
    }

    #[test]
    fn test_finalize_marks_repo_failed_if_agent_failed() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());

        {
            let _guard = ctx.agent_guard("FailingAgent");
            // drops without complete → marked failed
        }

        let result = ctx.finalize().expect("finalize should succeed even with failed agents");

        assert_eq!(result.repo_span.status, SpanStatus::Failed);
        assert_eq!(result.agent_spans.len(), 1);
        assert_eq!(result.agent_spans[0].status, SpanStatus::Failed);
    }

    #[test]
    fn test_finalize_failed_returns_all_spans() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());

        let guard = ctx.agent_guard("PartialAgent");
        guard.complete();

        let result = ctx.finalize_failed("external error occurred");

        assert_eq!(result.repo_span.status, SpanStatus::Failed);
        assert_eq!(
            result.repo_span.error_message.as_deref(),
            Some("external error occurred")
        );
        assert_eq!(result.agent_spans.len(), 1);
    }

    #[test]
    fn test_multiple_agents() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());

        let g1 = ctx.agent_guard("Agent1");
        g1.attach_artifact(Artifact::new("type_a", "ref-a"));
        g1.complete();

        let g2 = ctx.agent_guard("Agent2");
        g2.attach_artifact(Artifact::new("type_b", "ref-b"));
        g2.complete();

        let result = ctx.finalize().expect("finalize should succeed");

        assert_eq!(result.repo_span.status, SpanStatus::Completed);
        assert_eq!(result.agent_spans.len(), 2);
        assert_eq!(
            result.agent_spans[0].agent_name.as_deref(),
            Some("Agent1")
        );
        assert_eq!(
            result.agent_spans[1].agent_name.as_deref(),
            Some("Agent2")
        );
    }

    #[test]
    fn test_cloned_context_shares_graph() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());
        let ctx2 = ctx.clone();

        let g1 = ctx.agent_guard("AgentFromCtx1");
        g1.complete();

        let g2 = ctx2.agent_guard("AgentFromCtx2");
        g2.complete();

        let result = ctx.finalize().expect("finalize should succeed");
        assert_eq!(result.agent_spans.len(), 2);
    }

    #[test]
    fn test_execution_result_is_json_serializable() {
        let ctx = ExecutionContext::new(Uuid::new_v4(), Uuid::new_v4());

        let guard = ctx.agent_guard("SerAgent");
        guard.attach_artifact(
            Artifact::new("benchmark_result", "bench-001")
                .with_content(serde_json::json!({"accuracy": 0.95})),
        );
        guard.complete();

        let result = ctx.finalize().expect("finalize should succeed");
        let json = serde_json::to_string_pretty(&result).expect("should serialize to JSON");
        assert!(json.contains("benchmark-exchange"));
        assert!(json.contains("SerAgent"));
        assert!(json.contains("bench-001"));

        // Round-trip
        let deserialized: ExecutionResult =
            serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(deserialized.agent_spans.len(), 1);
    }

    #[test]
    fn test_causal_ordering() {
        let parent_id = Uuid::new_v4();
        let ctx = ExecutionContext::new(Uuid::new_v4(), parent_id);

        let guard = ctx.agent_guard("CausalAgent");
        guard.complete();

        let result = ctx.finalize().unwrap();

        // Repo span's parent is the Core span
        assert_eq!(result.repo_span.parent_span_id, parent_id);
        // Agent span's parent is the repo span
        assert_eq!(
            result.agent_spans[0].parent_span_id,
            result.repo_span.span_id
        );
    }
}
