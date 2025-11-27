//! CLI configuration management
//!
//! Handles loading and saving configuration from ~/.llm-benchmark/config.toml

use crate::output::OutputFormat;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// API endpoint URL
    #[serde(default = "default_api_endpoint")]
    pub api_endpoint: String,

    /// Authentication token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,

    /// Default output format
    #[serde(default)]
    pub output_format: OutputFormat,

    /// Enable colored output
    #[serde(default = "default_colored")]
    pub colored: bool,

    /// Default timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Enable debug logging
    #[serde(default)]
    pub debug: bool,
}

fn default_api_endpoint() -> String {
    "https://api.llm-benchmark.org".to_string()
}

fn default_colored() -> bool {
    true
}

fn default_timeout() -> u64 {
    30
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_endpoint: default_api_endpoint(),
            auth_token: None,
            output_format: OutputFormat::default(),
            colored: default_colored(),
            timeout_seconds: default_timeout(),
            debug: false,
        }
    }
}

impl Config {
    /// Get the config directory path
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        Ok(home.join(".llm-benchmark"))
    }

    /// Get the config file path
    pub fn config_file() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Load configuration from file, or create default if it doesn't exist
    pub fn load() -> Result<Self> {
        // Check environment variables first
        let mut config = Self::load_from_file()?;

        // Override with environment variables
        if let Ok(api_url) = std::env::var("LLM_BENCHMARK_API_URL") {
            config.api_endpoint = api_url;
        }
        if let Ok(token) = std::env::var("LLM_BENCHMARK_TOKEN") {
            config.auth_token = Some(token);
        }
        if let Ok(format) = std::env::var("LLM_BENCHMARK_OUTPUT_FORMAT") {
            config.output_format = match format.to_lowercase().as_str() {
                "json" => OutputFormat::Json,
                "plain" => OutputFormat::Plain,
                _ => OutputFormat::Table,
            };
        }
        if std::env::var("LLM_BENCHMARK_DEBUG").is_ok() {
            config.debug = true;
        }
        if std::env::var("NO_COLOR").is_ok() {
            config.colored = false;
        }

        Ok(config)
    }

    /// Load configuration from file only
    fn load_from_file() -> Result<Self> {
        let config_file = Self::config_file()?;

        if !config_file.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&config_file).context("Failed to read config file")?;
        let config: Config = toml::from_str(&contents).context("Failed to parse config file")?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
        }

        let config_file = Self::config_file()?;
        let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(&config_file, contents).context("Failed to write config file")?;

        Ok(())
    }

    /// Set authentication token
    pub fn set_auth_token(&mut self, token: String) -> Result<()> {
        self.auth_token = Some(token);
        self.save()
    }

    /// Clear authentication token
    pub fn clear_auth_token(&mut self) -> Result<()> {
        self.auth_token = None;
        self.save()
    }

    /// Check if authenticated
    pub fn is_authenticated(&self) -> bool {
        self.auth_token.is_some()
    }

    /// Get a configuration value by key
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "api_endpoint" | "api-endpoint" | "api_url" => Some(self.api_endpoint.clone()),
            "output_format" | "output-format" | "format" => Some(format!("{:?}", self.output_format)),
            "colored" | "color" => Some(self.colored.to_string()),
            "timeout" | "timeout_seconds" => Some(self.timeout_seconds.to_string()),
            "debug" => Some(self.debug.to_string()),
            "auth_token" | "token" => self.auth_token.clone(),
            _ => None,
        }
    }

    /// Set a configuration value by key
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "api_endpoint" | "api-endpoint" | "api_url" => {
                self.api_endpoint = value.to_string();
            }
            "output_format" | "output-format" | "format" => {
                self.output_format = match value.to_lowercase().as_str() {
                    "json" => OutputFormat::Json,
                    "plain" => OutputFormat::Plain,
                    "table" => OutputFormat::Table,
                    _ => anyhow::bail!("Invalid output format: {}. Use json, table, or plain", value),
                };
            }
            "colored" | "color" => {
                self.colored = value.parse().context("Invalid boolean value")?;
            }
            "timeout" | "timeout_seconds" => {
                self.timeout_seconds = value.parse().context("Invalid timeout value")?;
            }
            "debug" => {
                self.debug = value.parse().context("Invalid boolean value")?;
            }
            _ => anyhow::bail!("Unknown configuration key: {}", key),
        }
        self.save()
    }

    /// Reset configuration to defaults
    pub fn reset(&mut self) -> Result<()> {
        *self = Self::default();
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.api_endpoint, "https://api.llm-benchmark.org");
        assert_eq!(config.output_format, OutputFormat::Table);
        assert!(config.colored);
        assert!(config.auth_token.is_none());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.api_endpoint, deserialized.api_endpoint);
    }

    #[test]
    fn test_config_get() {
        let config = Config::default();
        assert_eq!(config.get("api_endpoint"), Some("https://api.llm-benchmark.org".to_string()));
        assert_eq!(config.get("colored"), Some("true".to_string()));
        assert_eq!(config.get("unknown"), None);
    }
}
