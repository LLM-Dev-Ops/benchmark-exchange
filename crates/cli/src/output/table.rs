//! Table formatting utilities

use anyhow::Result;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, *};

/// Table formatter
pub struct TableFormatter;

impl TableFormatter {
    /// Create a new table with default styling
    pub fn new() -> Table {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic);
        table
    }

    /// Create a simple table with headers and rows
    pub fn simple(headers: Vec<&str>, rows: Vec<Vec<String>>) -> Result<String> {
        let mut table = Self::new();
        table.set_header(headers);

        for row in rows {
            table.add_row(row);
        }

        Ok(table.to_string())
    }

    /// Create a key-value table
    pub fn key_value(items: Vec<(&str, String)>) -> Result<String> {
        let mut table = Self::new();

        for (key, value) in items {
            table.add_row(vec![key, &value]);
        }

        Ok(table.to_string())
    }
}

impl Default for TableFormatter {
    fn default() -> Self {
        Self::new();
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_table() {
        let headers = vec!["Name", "Age"];
        let rows = vec![
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ];
        let result = TableFormatter::simple(headers, rows);
        assert!(result.is_ok());
    }

    #[test]
    fn test_key_value_table() {
        let items = vec![
            ("Name", "Alice".to_string()),
            ("Age", "30".to_string()),
        ];
        let result = TableFormatter::key_value(items);
        assert!(result.is_ok());
    }
}
