//! CLI commands

pub mod auth;
pub mod benchmark;
pub mod init;
pub mod leaderboard;
pub mod proposal;
pub mod publication;
pub mod run;
pub mod submit;

use crate::client::ApiClient;
use crate::config::Config;
use anyhow::Result;

/// Context passed to all commands
pub struct CommandContext {
    pub config: Config,
    pub client: ApiClient,
}

impl CommandContext {
    /// Create a new command context
    pub fn new(config: Config) -> Result<Self> {
        let client = ApiClient::from_config(&config)?;
        Ok(Self { config, client })
    }

    /// Create a new command context with agentics execution context.
    ///
    /// When `execution_id` and `parent_span_id` are provided, the API client
    /// will include `X-Execution-Id` and `X-Parent-Span-Id` headers on every
    /// request, enabling execution span tracking in the API layer.
    pub fn new_with_execution(
        config: Config,
        execution_id: Option<String>,
        parent_span_id: Option<String>,
    ) -> Result<Self> {
        let client = ApiClient::from_config(&config)?
            .with_execution_context(execution_id, parent_span_id);
        Ok(Self { config, client })
    }

    /// Check if user is authenticated, return error if not
    pub fn require_auth(&self) -> Result<()> {
        if !self.config.is_authenticated() {
            anyhow::bail!("Not authenticated. Please run 'llm-benchmark auth login' first.");
        }
        Ok(())
    }
}
