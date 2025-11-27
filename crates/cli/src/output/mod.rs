//! Output formatting for CLI

use anyhow::Result;
use serde::{Deserialize, Serialize};

mod formatters;
mod table;

pub use formatters::{JsonFormatter, PlainFormatter};
pub use table::TableFormatter;

/// Output format enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// JSON output
    Json,
    /// Table output (default)
    #[default]
    Table,
    /// Plain text output
    Plain,
}

impl OutputFormat {
    /// Parse output format from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "table" => Some(Self::Table),
            "plain" => Some(Self::Plain),
            _ => None,
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json => write!(f, "json"),
            Self::Table => write!(f, "table"),
            Self::Plain => write!(f, "plain"),
        }
    }
}

/// Trait for types that can be formatted for output
pub trait Formattable {
    /// Format as JSON
    fn format_json(&self) -> Result<String>;

    /// Format as table
    fn format_table(&self) -> Result<String>;

    /// Format as plain text
    fn format_plain(&self) -> Result<String>;

    /// Format using the specified format
    fn format(&self, format: OutputFormat) -> Result<String> {
        match format {
            OutputFormat::Json => self.format_json(),
            OutputFormat::Table => self.format_table(),
            OutputFormat::Plain => self.format_plain(),
        }
    }
}

/// Default implementation for serializable types
impl<T: Serialize> Formattable for T {
    fn format_json(&self) -> Result<String> {
        JsonFormatter::format(self)
    }

    fn format_table(&self) -> Result<String> {
        // Default table formatting - override for custom implementations
        self.format_plain()
    }

    fn format_plain(&self) -> Result<String> {
        PlainFormatter::format(self)
    }
}

/// Color helpers
pub mod colors {
    use colored::*;

    pub fn success(s: &str) -> ColoredString {
        s.green()
    }

    pub fn error(s: &str) -> ColoredString {
        s.red()
    }

    pub fn warning(s: &str) -> ColoredString {
        s.yellow()
    }

    pub fn info(s: &str) -> ColoredString {
        s.blue()
    }

    pub fn dim(s: &str) -> ColoredString {
        s.dimmed()
    }

    pub fn bold(s: &str) -> ColoredString {
        s.bold()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::from_str("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::from_str("JSON"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::from_str("table"), Some(OutputFormat::Table));
        assert_eq!(OutputFormat::from_str("plain"), Some(OutputFormat::Plain));
        assert_eq!(OutputFormat::from_str("invalid"), None);
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(OutputFormat::Json.to_string(), "json");
        assert_eq!(OutputFormat::Table.to_string(), "table");
        assert_eq!(OutputFormat::Plain.to_string(), "plain");
    }

    #[test]
    fn test_output_format_serialization() {
        let json = OutputFormat::Json;
        let serialized = serde_json::to_string(&json).unwrap();
        assert_eq!(serialized, "\"json\"");

        let deserialized: OutputFormat = serde_json::from_str("\"table\"").unwrap();
        assert_eq!(deserialized, OutputFormat::Table);
    }
}
