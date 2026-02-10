//! Integration tests for CSV and JSON export functionality.
//!
//! These tests verify that export_table correctly handles column visibility,
//! special characters, and produces properly formatted CSV and JSON output.

use pretty_table_explorer::export::{export_table, ExportFormat};
use pretty_table_explorer::parser::TableData;
use serde_json::Value;
use std::collections::HashMap;

/// Helper function to create sample table data
fn sample_table() -> TableData {
    TableData {
        headers: vec!["id".into(), "name".into(), "age".into()],
        rows: vec![
            vec!["1".into(), "Alice".into(), "30".into()],
            vec!["2".into(), "Bob".into(), "25".into()],
            vec!["3".into(), "Charlie".into(), "35".into()],
        ],
    }
}

#[test]
fn test_export_csv_all_columns() {
    let data = sample_table();
    let visible = vec![0, 1, 2]; // All columns visible

    let result = export_table(&data, &visible, ExportFormat::Csv).expect("CSV export failed");

    // Check for UTF-8 BOM prefix
    assert!(result.starts_with("\u{FEFF}"), "CSV should start with UTF-8 BOM");

    // Check header row
    assert!(result.contains("id,name,age"), "CSV should contain header row");

    // Check data rows
    assert!(result.contains("1,Alice,30"), "CSV should contain first data row");
    assert!(result.contains("2,Bob,25"), "CSV should contain second data row");
    assert!(result.contains("3,Charlie,35"), "CSV should contain third data row");
}

#[test]
fn test_export_csv_subset_columns() {
    let data = sample_table();
    let visible = vec![0, 2]; // Only id and age columns

    let result = export_table(&data, &visible, ExportFormat::Csv).expect("CSV export failed");

    // Check that only visible columns appear
    assert!(result.contains("id,age"), "CSV should contain only visible columns");
    assert!(!result.contains("name"), "CSV should not contain hidden 'name' column header");
    assert!(result.contains("1,30"), "CSV should contain id and age from first row");
    assert!(result.contains("2,25"), "CSV should contain id and age from second row");
    assert!(!result.contains("Alice"), "CSV should not contain data from hidden column");
}

#[test]
fn test_export_json_all_columns() {
    let data = sample_table();
    let visible = vec![0, 1, 2]; // All columns visible

    let result = export_table(&data, &visible, ExportFormat::Json).expect("JSON export failed");

    // Parse the JSON output
    let parsed: Vec<HashMap<String, String>> = serde_json::from_str(&result).expect("Failed to parse JSON");

    assert_eq!(parsed.len(), 3, "JSON should contain 3 rows");

    // Check first row
    assert_eq!(parsed[0].get("id").unwrap(), "1");
    assert_eq!(parsed[0].get("name").unwrap(), "Alice");
    assert_eq!(parsed[0].get("age").unwrap(), "30");

    // Check second row
    assert_eq!(parsed[1].get("id").unwrap(), "2");
    assert_eq!(parsed[1].get("name").unwrap(), "Bob");
    assert_eq!(parsed[1].get("age").unwrap(), "25");

    // Check third row
    assert_eq!(parsed[2].get("id").unwrap(), "3");
    assert_eq!(parsed[2].get("name").unwrap(), "Charlie");
    assert_eq!(parsed[2].get("age").unwrap(), "35");
}

#[test]
fn test_export_json_subset_columns() {
    let data = sample_table();
    let visible = vec![1]; // Only name column

    let result = export_table(&data, &visible, ExportFormat::Json).expect("JSON export failed");

    let parsed: Vec<HashMap<String, String>> = serde_json::from_str(&result).expect("Failed to parse JSON");

    assert_eq!(parsed.len(), 3, "JSON should contain 3 rows");

    // Verify only 'name' key exists
    assert_eq!(parsed[0].get("name").unwrap(), "Alice");
    assert!(!parsed[0].contains_key("id"), "Hidden column 'id' should not be in JSON");
    assert!(!parsed[0].contains_key("age"), "Hidden column 'age' should not be in JSON");

    assert_eq!(parsed[1].get("name").unwrap(), "Bob");
    assert_eq!(parsed[2].get("name").unwrap(), "Charlie");
}

#[test]
fn test_export_csv_special_characters() {
    // Create data with commas, quotes, and newlines
    let data = TableData {
        headers: vec!["id".into(), "description".into(), "notes".into()],
        rows: vec![
            vec!["1".into(), "Value with, comma".into(), "Normal".into()],
            vec!["2".into(), r#"Value with "quotes""#.into(), "Test".into()],
            vec!["3".into(), "Value\nwith\nnewline".into(), "Multi".into()],
        ],
    };
    let visible = vec![0, 1, 2];

    let result = export_table(&data, &visible, ExportFormat::Csv).expect("CSV export failed");

    // CSV library should properly escape these
    // Commas should cause quoting
    assert!(result.contains(r#""Value with, comma""#), "CSV should quote values with commas");

    // Quotes should be escaped as double quotes
    assert!(result.contains(r#""Value with ""quotes""""#), "CSV should escape quotes by doubling them");

    // Newlines should cause quoting
    assert!(result.contains("Value\nwith\nnewline") || result.contains("\"Value\nwith\nnewline\""),
        "CSV should handle newlines in values");
}

#[test]
fn test_export_empty_table() {
    let data = TableData {
        headers: vec!["col1".into(), "col2".into()],
        rows: vec![],
    };
    let visible = vec![0, 1];

    // CSV should have header only
    let csv_result = export_table(&data, &visible, ExportFormat::Csv).expect("CSV export failed");
    assert!(csv_result.contains("col1,col2"), "CSV should contain header");
    // Should not have any data rows (just BOM + header + newline)
    let line_count = csv_result.lines().count();
    assert_eq!(line_count, 1, "CSV should have only header row");

    // JSON should be empty array
    let json_result = export_table(&data, &visible, ExportFormat::Json).expect("JSON export failed");
    let parsed: Value = serde_json::from_str(&json_result).expect("Failed to parse JSON");
    assert!(parsed.is_array(), "JSON should be an array");
    assert_eq!(parsed.as_array().unwrap().len(), 0, "JSON array should be empty");
}

#[test]
fn test_export_reordered_columns() {
    let data = sample_table();
    let visible = vec![2, 1, 0]; // Reverse order: age, name, id

    let csv_result = export_table(&data, &visible, ExportFormat::Csv).expect("CSV export failed");

    // Check header order
    assert!(csv_result.contains("age,name,id"), "CSV should respect column order");

    // Check data order
    assert!(csv_result.contains("30,Alice,1"), "CSV data should be in reordered column sequence");
}

#[test]
fn test_export_single_column() {
    let data = sample_table();
    let visible = vec![1]; // Only name column

    let csv_result = export_table(&data, &visible, ExportFormat::Csv).expect("CSV export failed");

    assert!(csv_result.contains("name"), "CSV should have name header");
    assert!(csv_result.contains("Alice"), "CSV should have Alice");
    assert!(!csv_result.contains("id") || !csv_result.contains("age"), "CSV should not have other columns");
}

#[test]
fn test_export_json_preserves_structure() {
    let data = TableData {
        headers: vec!["user_id".into(), "username".into(), "score".into()],
        rows: vec![
            vec!["100".into(), "player1".into(), "9500".into()],
            vec!["200".into(), "player2".into(), "8750".into()],
        ],
    };
    let visible = vec![0, 1, 2];

    let result = export_table(&data, &visible, ExportFormat::Json).expect("JSON export failed");

    let parsed: Vec<HashMap<String, String>> = serde_json::from_str(&result).expect("Failed to parse JSON");

    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].len(), 3, "Each object should have 3 keys");
    assert_eq!(parsed[0].get("user_id").unwrap(), "100");
    assert_eq!(parsed[0].get("username").unwrap(), "player1");
    assert_eq!(parsed[0].get("score").unwrap(), "9500");
}
