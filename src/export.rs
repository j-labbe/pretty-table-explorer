//! Data export functionality for CSV and JSON formats.
//!
//! Exports table data respecting column visibility and display order.

use crate::parser::TableData;
use std::collections::HashMap;

/// Export format selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Csv,
    Json,
}

/// Export table data to a string in the specified format.
///
/// Only exports visible columns in the order specified by `visible_cols`.
/// Returns the serialized string or an error message.
pub fn export_table(
    data: &TableData,
    visible_cols: &[usize],
    format: ExportFormat,
) -> Result<String, String> {
    match format {
        ExportFormat::Csv => export_csv(data, visible_cols),
        ExportFormat::Json => export_json(data, visible_cols),
    }
}

/// UTF-8 BOM (Byte Order Mark) for Excel compatibility
const UTF8_BOM: &str = "\u{FEFF}";

/// Export to CSV format with UTF-8 BOM for Excel compatibility
fn export_csv(data: &TableData, visible_cols: &[usize]) -> Result<String, String> {
    let mut wtr = csv::Writer::from_writer(Vec::new());

    // Write headers (only visible columns in order)
    let headers: Vec<&str> = visible_cols
        .iter()
        .filter_map(|&i| data.headers.get(i).map(|s| s.as_str()))
        .collect();
    wtr.write_record(&headers)
        .map_err(|e| format!("Failed to write CSV headers: {}", e))?;

    // Write data rows (only visible columns in order)
    for row in &data.rows {
        let values: Vec<&str> = visible_cols
            .iter()
            .map(|&i| row.get(i).map(|s| s.as_str()).unwrap_or(""))
            .collect();
        wtr.write_record(&values)
            .map_err(|e| format!("Failed to write CSV row: {}", e))?;
    }

    let bytes = wtr
        .into_inner()
        .map_err(|e| format!("Failed to finalize CSV: {}", e))?;

    let csv_content =
        String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8 in CSV output: {}", e))?;

    // Prepend UTF-8 BOM for Excel compatibility
    Ok(format!("{}{}", UTF8_BOM, csv_content))
}

/// Export to JSON format (array of objects)
fn export_json(data: &TableData, visible_cols: &[usize]) -> Result<String, String> {
    let mut rows_json: Vec<HashMap<&str, &str>> = Vec::new();

    for row in &data.rows {
        let mut row_obj: HashMap<&str, &str> = HashMap::new();
        for &col_idx in visible_cols {
            if let Some(header) = data.headers.get(col_idx) {
                let value = row.get(col_idx).map(|s| s.as_str()).unwrap_or("");
                row_obj.insert(header.as_str(), value);
            }
        }
        rows_json.push(row_obj);
    }

    serde_json::to_string_pretty(&rows_json)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))
}

/// Save content to a file
pub fn save_to_file(content: &str, path: &str) -> Result<(), String> {
    std::fs::write(path, content).map_err(|e| format!("Failed to write file '{}': {}", path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_table() -> TableData {
        TableData {
            headers: vec!["id".to_string(), "name".to_string(), "age".to_string()],
            rows: vec![
                vec!["1".to_string(), "Alice".to_string(), "30".to_string()],
                vec!["2".to_string(), "Bob".to_string(), "25".to_string()],
            ],
        }
    }

    #[test]
    fn test_export_csv_all_columns() {
        let data = sample_table();
        let visible = vec![0, 1, 2];
        let result = export_table(&data, &visible, ExportFormat::Csv).unwrap();

        assert!(result.contains("id,name,age"));
        assert!(result.contains("1,Alice,30"));
        assert!(result.contains("2,Bob,25"));
    }

    #[test]
    fn test_export_csv_subset_columns() {
        let data = sample_table();
        let visible = vec![1, 2]; // Only name and age
        let result = export_table(&data, &visible, ExportFormat::Csv).unwrap();

        assert!(result.contains("name,age"));
        assert!(result.contains("Alice,30"));
        assert!(result.contains("Bob,25"));
        assert!(!result.contains("id"));
    }

    #[test]
    fn test_export_csv_reordered_columns() {
        let data = sample_table();
        let visible = vec![2, 0]; // age, then id
        let result = export_table(&data, &visible, ExportFormat::Csv).unwrap();

        assert!(result.contains("age,id"));
        assert!(result.contains("30,1"));
        assert!(result.contains("25,2"));
    }

    #[test]
    fn test_export_json_all_columns() {
        let data = sample_table();
        let visible = vec![0, 1, 2];
        let result = export_table(&data, &visible, ExportFormat::Json).unwrap();

        // Parse to verify structure
        let parsed: Vec<HashMap<String, String>> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].get("id").unwrap(), "1");
        assert_eq!(parsed[0].get("name").unwrap(), "Alice");
        assert_eq!(parsed[1].get("name").unwrap(), "Bob");
    }

    #[test]
    fn test_export_json_subset_columns() {
        let data = sample_table();
        let visible = vec![1]; // Only name
        let result = export_table(&data, &visible, ExportFormat::Json).unwrap();

        let parsed: Vec<HashMap<String, String>> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].get("name").unwrap(), "Alice");
        assert!(parsed[0].get("id").is_none()); // Not included
    }

    #[test]
    fn test_export_empty_table() {
        let data = TableData {
            headers: vec!["col1".to_string()],
            rows: vec![],
        };
        let visible = vec![0];

        // CSV should have just headers
        let csv_result = export_table(&data, &visible, ExportFormat::Csv).unwrap();
        assert!(csv_result.contains("col1"));

        // JSON should be empty array
        let json_result = export_table(&data, &visible, ExportFormat::Json).unwrap();
        assert_eq!(json_result.trim(), "[]");
    }
}
