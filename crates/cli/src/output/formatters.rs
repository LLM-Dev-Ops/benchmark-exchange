//! Output formatters

use anyhow::Result;
use serde::Serialize;

/// JSON formatter
pub struct JsonFormatter;

impl JsonFormatter {
    /// Format a value as pretty JSON
    pub fn format<T: Serialize>(value: &T) -> Result<String> {
        Ok(serde_json::to_string_pretty(value)?)
    }
}

/// Plain text formatter
pub struct PlainFormatter;

impl PlainFormatter {
    /// Format a value as debug output
    pub fn format<T: Serialize>(value: &T) -> Result<String> {
        // Convert to JSON first, then pretty-print
        let json = serde_json::to_value(value)?;
        Ok(Self::format_value(&json, 0))
    }

    fn format_value(value: &serde_json::Value, indent: usize) -> String {
        let indent_str = "  ".repeat(indent);
        match value {
            serde_json::Value::Null => "null".to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Array(arr) => {
                let items: Vec<String> = arr
                    .iter()
                    .map(|v| format!("{}  - {}", indent_str, Self::format_value(v, indent + 1)))
                    .collect();
                items.join("\n")
            }
            serde_json::Value::Object(obj) => {
                let items: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| {
                        format!("{}{}: {}", indent_str, k, Self::format_value(v, indent + 1))
                    })
                    .collect();
                items.join("\n")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestData {
        name: String,
        count: i32,
    }

    #[test]
    fn test_json_formatter() {
        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };
        let result = JsonFormatter::format(&data);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("test"));
    }

    #[test]
    fn test_plain_formatter() {
        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };
        let result = PlainFormatter::format(&data);
        assert!(result.is_ok());
    }
}
