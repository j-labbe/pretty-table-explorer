//! Integration tests for column configuration operations.
//!
//! These tests verify that column hide/show/reorder/resize operations work correctly
//! and that column visibility properly integrates with export functionality.

use pretty_table_explorer::column::ColumnConfig;
use pretty_table_explorer::export::{export_table, ExportFormat};
use pretty_table_explorer::parser::TableData;

#[test]
fn test_column_config_new() {
    let config = ColumnConfig::new(5);

    assert_eq!(config.visible_count(), 5, "All columns should be visible initially");
    let visible_indices = config.visible_indices();
    assert_eq!(visible_indices, vec![0, 1, 2, 3, 4], "Visible indices should be [0,1,2,3,4]");
}

#[test]
fn test_column_hide() {
    let mut config = ColumnConfig::new(5);

    config.hide(2);

    assert_eq!(config.visible_count(), 4, "Visible count should be 4 after hiding one column");
    let visible_indices = config.visible_indices();
    assert_eq!(visible_indices, vec![0, 1, 3, 4], "Column 2 should be excluded from visible indices");
    assert!(!visible_indices.contains(&2), "Hidden column should not be in visible indices");
}

#[test]
fn test_column_show_all() {
    let mut config = ColumnConfig::new(5);

    // Hide columns 1 and 3
    config.hide(1);
    config.hide(3);
    assert_eq!(config.visible_count(), 3, "Should have 3 visible columns");

    // Show all
    config.show_all();
    assert_eq!(config.visible_count(), 5, "All columns should be visible after show_all");
    let visible_indices = config.visible_indices();
    assert_eq!(visible_indices, vec![0, 1, 2, 3, 4], "All indices should be visible");
}

#[test]
fn test_column_hide_show_preserves_order() {
    let mut config = ColumnConfig::new(5);

    // Hide column 2
    config.hide(2);
    let visible_indices = config.visible_indices();
    assert_eq!(visible_indices, vec![0, 1, 3, 4], "Order should be preserved with column 2 hidden");

    // Show all
    config.show_all();
    let visible_indices = config.visible_indices();
    assert_eq!(visible_indices, vec![0, 1, 2, 3, 4], "Order should be restored after show_all");
}

#[test]
fn test_column_reorder_swap() {
    let mut config = ColumnConfig::new(5);

    // Swap display positions 1 and 3
    config.swap_display(1, 3);

    // Check that display positions changed
    assert_eq!(config.display_position(1), Some(3), "Column 1 should be at display position 3");
    assert_eq!(config.display_position(3), Some(1), "Column 3 should be at display position 1");

    // Original positions should be unchanged
    assert_eq!(config.display_position(0), Some(0), "Column 0 should remain at position 0");
    assert_eq!(config.display_position(2), Some(2), "Column 2 should remain at position 2");
    assert_eq!(config.display_position(4), Some(4), "Column 4 should remain at position 4");
}

#[test]
fn test_column_resize_width() {
    let mut config = ColumnConfig::new(3);

    // Adjust column 0 width by +5 from auto width of 10
    config.adjust_width(0, 5, 10);

    assert_eq!(config.get_width(0), Some(15), "Width should be 10 + 5 = 15");
    assert_eq!(config.get_width(1), None, "Column 1 should still be auto-sized");
}

#[test]
fn test_column_resize_minimum() {
    let mut config = ColumnConfig::new(2);

    // Try to adjust to below minimum (3)
    config.adjust_width(0, -20, 15); // 15 - 20 = -5, should clamp to 3

    assert_eq!(config.get_width(0), Some(3), "Width should clamp to minimum of 3");
}

#[test]
fn test_column_resize_no_override() {
    let config = ColumnConfig::new(3);

    // Fresh config should have no overrides
    assert_eq!(config.get_width(0), None, "Fresh config should return None (auto-size)");
    assert_eq!(config.get_width(1), None, "Fresh config should return None (auto-size)");
    assert_eq!(config.get_width(2), None, "Fresh config should return None (auto-size)");
}

#[test]
fn test_column_reset() {
    let mut config = ColumnConfig::new(5);

    // Modify several columns
    config.hide(1);
    config.hide(3);
    config.adjust_width(0, 10, 15);
    config.adjust_width(2, 5, 20);
    config.swap_display(0, 4);

    // Verify modifications
    assert_eq!(config.visible_count(), 3, "Should have 3 visible columns");
    assert_eq!(config.get_width(0), Some(25), "Column 0 should have width override");
    assert_ne!(config.display_position(0), Some(0), "Display order should be modified");

    // Reset
    config.reset();

    // Verify everything back to defaults
    assert_eq!(config.visible_count(), 5, "All columns should be visible after reset");
    assert_eq!(config.get_width(0), None, "Width overrides should be cleared");
    assert_eq!(config.get_width(2), None, "Width overrides should be cleared");
    assert_eq!(config.display_position(0), Some(0), "Display order should be restored");
    assert_eq!(config.display_position(4), Some(4), "Display order should be restored");
}

#[test]
fn test_column_visibility_with_export() {
    // Create TableData
    use lasso::Rodeo;
    let mut interner = Rodeo::default();
    let data = TableData {
        headers: vec!["id".into(), "name".into(), "age".into(), "city".into()],
        rows: vec![
            vec![
                interner.get_or_intern("1"),
                interner.get_or_intern("Alice"),
                interner.get_or_intern("30"),
                interner.get_or_intern("New York"),
            ],
            vec![
                interner.get_or_intern("2"),
                interner.get_or_intern("Bob"),
                interner.get_or_intern("25"),
                interner.get_or_intern("Seattle"),
            ],
        ],
        interner,
    };

    // Create ColumnConfig and hide column 2 (age)
    let mut config = ColumnConfig::new(4);
    config.hide(2);

    // Get visible indices
    let visible_indices = config.visible_indices();
    assert_eq!(visible_indices, vec![0, 1, 3], "Age column should be hidden");

    // Export with visible columns only
    let csv_result = export_table(&data, &visible_indices, ExportFormat::Csv)
        .expect("CSV export failed");

    // Verify hidden column is excluded
    assert!(csv_result.contains("id,name,city"), "CSV should only contain visible columns");
    assert!(!csv_result.contains("age"), "CSV should not contain hidden 'age' column");
    assert!(csv_result.contains("1,Alice,New York"), "CSV should have data from visible columns");
    assert!(!csv_result.contains("30"), "CSV should not contain data from hidden column");

    // Test with JSON as well
    let json_result = export_table(&data, &visible_indices, ExportFormat::Json)
        .expect("JSON export failed");

    let parsed: Vec<std::collections::HashMap<String, String>> =
        serde_json::from_str(&json_result).expect("Failed to parse JSON");

    assert_eq!(parsed[0].len(), 3, "Each JSON object should have 3 keys (hidden column excluded)");
    assert!(parsed[0].contains_key("id"), "JSON should have id key");
    assert!(parsed[0].contains_key("name"), "JSON should have name key");
    assert!(parsed[0].contains_key("city"), "JSON should have city key");
    assert!(!parsed[0].contains_key("age"), "JSON should not have hidden 'age' key");
}

#[test]
fn test_column_multiple_operations() {
    let mut config = ColumnConfig::new(6);

    // Perform multiple operations
    config.hide(1);
    config.hide(4);
    config.adjust_width(0, 5, 20);
    config.adjust_width(5, -3, 15);
    config.swap_display(0, 2);

    // Verify compound state
    assert_eq!(config.visible_count(), 4, "Should have 4 visible columns");
    assert_eq!(config.get_width(0), Some(25), "Column 0 width override should persist");
    assert_eq!(config.get_width(5), Some(12), "Column 5 width override should persist");

    let visible_indices = config.visible_indices();
    assert_eq!(visible_indices.len(), 4, "Should have 4 visible indices");
    assert!(!visible_indices.contains(&1), "Column 1 should be hidden");
    assert!(!visible_indices.contains(&4), "Column 4 should be hidden");
}

#[test]
fn test_column_hide_out_of_bounds() {
    let mut config = ColumnConfig::new(3);

    // Try to hide column that doesn't exist
    config.hide(10);

    // Should not affect existing columns
    assert_eq!(config.visible_count(), 3, "All original columns should still be visible");
}

#[test]
fn test_column_resize_maximum() {
    let mut config = ColumnConfig::new(2);

    // Try to adjust to above maximum (100)
    config.adjust_width(0, 200, 10); // 10 + 200 = 210, should clamp to 100

    assert_eq!(config.get_width(0), Some(100), "Width should clamp to maximum of 100");
}

#[test]
fn test_column_visibility_reorder_integration() {
    let mut config = ColumnConfig::new(4);

    // Reorder: swap 0 and 3
    config.swap_display(0, 3);

    // Hide column 1
    config.hide(1);

    // Get visible indices - should respect both reorder and visibility
    let visible_indices = config.visible_indices();

    // After swap: display order is [3, 1, 2, 0]
    // After hide: visible display order is [3, 2, 0] (column 1 excluded)
    assert_eq!(visible_indices, vec![3, 2, 0], "Should respect both reorder and visibility");
    assert_eq!(visible_indices.len(), 3, "Should have 3 visible columns");
}
