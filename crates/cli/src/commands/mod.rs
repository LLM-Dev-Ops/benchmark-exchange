//! CLI commands

pub mod auth;
pub mod benchmark;
pub mod init;
pub mod leaderboard;
pub mod proposal;
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

    /// Check if user is authenticated, return error if not
    pub fn require_auth(&self) -> Result<()> {
        if !self.config.is_authenticated() {
            anyhow::bail!("Not authenticated. Please run 'llm-benchmark auth login' first.");
        }
        Ok(())
    }
}
