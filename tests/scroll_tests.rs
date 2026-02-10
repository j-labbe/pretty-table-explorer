//! Integration tests for scroll viewport boundary conditions.
//!
//! These tests verify that viewport windowing handles all boundary conditions correctly:
//! - Top of dataset (row 0)
//! - Bottom of dataset (last row)
//! - Middle positions
//! - Empty datasets
//! - Single row datasets
//! - Small datasets (smaller than viewport)
//! - Filtered data at boundaries
//! - Offset consistency across positions

use lasso::Rodeo;
use pretty_table_explorer::parser::TableData;
use pretty_table_explorer::render::build_pane_render_data;
use pretty_table_explorer::workspace::{Tab, ViewMode};

/// Helper function to create test data with specified dimensions.
fn create_test_data(num_rows: usize, num_cols: usize) -> TableData {
    let headers: Vec<String> = (1..=num_cols).map(|i| format!("col_{}", i)).collect();
    let mut interner = Rodeo::default();
    let rows = (0..num_rows)
        .map(|row| {
            (0..num_cols)
                .map(|col| interner.get_or_intern(format!("r{}_c{}", row, col)))
                .collect()
        })
        .collect();
    TableData {
        headers,
        rows,
        interner,
    }
}

#[test]
fn test_viewport_at_top() {
    // Test viewport at row 0 (top of dataset)
    let data = create_test_data(1000, 5);
    let mut tab = Tab::new("Test".to_string(), data, ViewMode::PipeData);
    tab.table_state.select(Some(0));

    let render_data = build_pane_render_data(&tab, 25);

    assert_eq!(
        render_data.viewport_row_offset, 0,
        "viewport_row_offset should be 0 at top of dataset"
    );
    assert_eq!(
        render_data.displayed_row_count, 1000,
        "displayed_row_count should equal total row count (no filter)"
    );
    assert!(
        !render_data.display_rows.is_empty(),
        "display_rows should not be empty"
    );
    // Verify the first row's data is present
    assert_eq!(
        render_data.display_rows[0][0], "r0_c0",
        "First row should start with r0_c0"
    );
}

#[test]
fn test_viewport_at_bottom() {
    // Test viewport at last row (bottom of dataset)
    let total_rows = 1000;
    let data = create_test_data(total_rows, 5);
    let mut tab = Tab::new("Test".to_string(), data, ViewMode::PipeData);
    tab.table_state.select(Some(total_rows - 1));

    let render_data = build_pane_render_data(&tab, 25);

    // The last row should be present in display_rows
    let last_row_content = format!("r{}_c0", total_rows - 1);
    let found_last_row = render_data
        .display_rows
        .iter()
        .any(|row| row[0] == last_row_content);
    assert!(
        found_last_row,
        "Last row content '{}' should be in display_rows",
        last_row_content
    );
    assert_eq!(
        render_data.displayed_row_count, total_rows,
        "displayed_row_count should equal total rows"
    );
}

#[test]
fn test_viewport_at_middle() {
    // Test viewport at middle of dataset
    let total_rows = 10000;
    let data = create_test_data(total_rows, 5);
    let mut tab = Tab::new("Test".to_string(), data, ViewMode::PipeData);
    let middle = total_rows / 2;
    tab.table_state.select(Some(middle));

    let viewport_height = 25;
    let buffer = viewport_height * 2; // 50
    let render_data = build_pane_render_data(&tab, viewport_height);

    // viewport_row_offset should be approximately (middle - buffer)
    let expected_offset = middle.saturating_sub(buffer);
    assert_eq!(
        render_data.viewport_row_offset, expected_offset,
        "viewport_row_offset should be middle - buffer"
    );

    // The selected row should be present in the viewport window
    let selected_row_content = format!("r{}_c0", middle);
    let found_selected_row = render_data
        .display_rows
        .iter()
        .any(|row| row[0] == selected_row_content);
    assert!(
        found_selected_row,
        "Selected row content '{}' should be in display_rows",
        selected_row_content
    );
}

#[test]
fn test_viewport_row_count_unfiltered() {
    // Test various dataset sizes
    let test_cases = vec![10, 100, 10000];
    let viewport_height = 25;
    let buffer = viewport_height * 2; // 50

    for total_rows in test_cases {
        let data = create_test_data(total_rows, 5);
        let mut tab = Tab::new("Test".to_string(), data, ViewMode::PipeData);
        tab.table_state.select(Some(0));

        let render_data = build_pane_render_data(&tab, viewport_height);

        assert_eq!(
            render_data.displayed_row_count, total_rows,
            "displayed_row_count should equal total row count for {} rows",
            total_rows
        );

        // display_rows.len() depends on position and total_rows
        // At position 0: start=0, end=min(0+buffer, total) = min(50, total)
        // So for 10 rows: 10, for 100 rows: 50, for 10000 rows: 50
        let selected: usize = 0;
        let start = selected.saturating_sub(buffer);
        let end = selected.saturating_add(buffer).min(total_rows);
        let expected_display_len = end - start;
        assert_eq!(
            render_data.display_rows.len(),
            expected_display_len,
            "display_rows length should match viewport window for {} rows",
            total_rows
        );
    }
}

#[test]
fn test_viewport_empty_dataset() {
    // Test with zero rows
    let data = create_test_data(0, 5);
    let tab = Tab::new("Test".to_string(), data, ViewMode::PipeData);

    let render_data = build_pane_render_data(&tab, 25);

    assert_eq!(
        render_data.display_rows.len(),
        0,
        "display_rows should be empty for empty dataset"
    );
    assert_eq!(
        render_data.displayed_row_count, 0,
        "displayed_row_count should be 0 for empty dataset"
    );
    assert_eq!(
        render_data.viewport_row_offset, 0,
        "viewport_row_offset should be 0 for empty dataset"
    );
    // Should not panic - test passes if it reaches this point
}

#[test]
fn test_viewport_single_row() {
    // Test with exactly one row
    let data = create_test_data(1, 5);
    let mut tab = Tab::new("Test".to_string(), data, ViewMode::PipeData);
    tab.table_state.select(Some(0));

    let render_data = build_pane_render_data(&tab, 25);

    assert_eq!(
        render_data.display_rows.len(),
        1,
        "display_rows should have exactly 1 row"
    );
    assert_eq!(
        render_data.viewport_row_offset, 0,
        "viewport_row_offset should be 0 for single row"
    );
    assert_eq!(
        render_data.display_rows[0][0], "r0_c0",
        "Content should match expected value"
    );
}

#[test]
fn test_viewport_small_dataset() {
    // Test with dataset smaller than viewport (5 rows, viewport=25)
    let data = create_test_data(5, 5);
    let mut tab = Tab::new("Test".to_string(), data, ViewMode::PipeData);
    tab.table_state.select(Some(0));

    let render_data = build_pane_render_data(&tab, 25);

    assert_eq!(
        render_data.display_rows.len(),
        5,
        "All 5 rows should be in display_rows"
    );
    assert_eq!(
        render_data.viewport_row_offset, 0,
        "viewport_row_offset should be 0 for small dataset"
    );
    assert_eq!(
        render_data.displayed_row_count, 5,
        "displayed_row_count should be 5"
    );
}

#[test]
fn test_viewport_with_filter_at_boundaries() {
    // Test with 1000 rows, filter matching ~10% (rows starting with r0, r10-r19, r100-r199, etc.)
    let data = create_test_data(1000, 5);
    let mut tab = Tab::new("Test".to_string(), data, ViewMode::PipeData);
    tab.filter_text = "r0".to_string();

    // Select first filtered match (row 0)
    tab.table_state.select(Some(0));
    let render_data = build_pane_render_data(&tab, 25);

    assert!(
        render_data.displayed_row_count > 0,
        "Filter should match some rows"
    );
    assert!(
        !render_data.display_rows.is_empty(),
        "display_rows should contain filtered data at top"
    );
    // Verify filtered data contains "r0"
    assert!(
        render_data.display_rows[0][0].contains("r0"),
        "First filtered row should contain 'r0'"
    );

    // Select last filtered match
    let last_filtered_idx = render_data.displayed_row_count - 1;
    tab.table_state.select(Some(last_filtered_idx));
    let render_data_bottom = build_pane_render_data(&tab, 25);

    // Should not panic at bottom of filtered data
    assert!(
        !render_data_bottom.display_rows.is_empty(),
        "display_rows should contain filtered data at bottom"
    );
}

#[test]
fn test_viewport_offset_consistency() {
    // Test offset consistency at various positions in a large dataset
    let total_rows = 10000;
    let data = create_test_data(total_rows, 5);
    let viewport_height = 25;
    let positions = vec![0, 100, 500, 5000, 9999];

    for selected_row in positions {
        let mut tab = Tab::new("Test".to_string(), data.clone(), ViewMode::PipeData);
        tab.table_state.select(Some(selected_row));

        let render_data = build_pane_render_data(&tab, viewport_height);

        // Verify viewport_row_offset + display_rows.len() does not exceed total_rows
        let viewport_end = render_data.viewport_row_offset + render_data.display_rows.len();
        assert!(
            viewport_end <= total_rows,
            "viewport_end ({}) should not exceed total_rows ({}) at position {}",
            viewport_end,
            total_rows,
            selected_row
        );

        // Verify viewport_row_offset <= selected row index
        assert!(
            render_data.viewport_row_offset <= selected_row,
            "viewport_row_offset ({}) should be <= selected row ({}) at position {}",
            render_data.viewport_row_offset,
            selected_row,
            selected_row
        );

        // Verify the selected row is within the display window
        let buffer = viewport_height * 2;
        let expected_start = selected_row.saturating_sub(buffer);
        let expected_end = selected_row.saturating_add(buffer).min(total_rows);
        let window_size = expected_end - expected_start;
        assert_eq!(
            render_data.display_rows.len(),
            window_size,
            "display_rows length should match window size at position {}",
            selected_row
        );
    }
}

#[test]
fn test_viewport_filter_no_matches() {
    // Test filter that matches nothing
    let data = create_test_data(100, 5);
    let mut tab = Tab::new("Test".to_string(), data, ViewMode::PipeData);
    tab.filter_text = "nonexistent_xyz".to_string();
    tab.table_state.select(Some(0));

    let render_data = build_pane_render_data(&tab, 25);

    assert_eq!(
        render_data.displayed_row_count, 0,
        "displayed_row_count should be 0 when filter matches nothing"
    );
    assert_eq!(
        render_data.display_rows.len(),
        0,
        "display_rows should be empty when filter matches nothing"
    );
    // Should not panic
}

#[test]
fn test_viewport_selected_beyond_filtered_count() {
    // Test when selected row index exceeds filtered result count
    // (This simulates user being at row 500, then filter reduces to 10 matches)
    let data = create_test_data(1000, 5);
    let mut tab = Tab::new("Test".to_string(), data, ViewMode::PipeData);
    tab.filter_text = "r999".to_string(); // Only matches row 999
    tab.table_state.select(Some(500)); // Selected is beyond filtered count

    let render_data = build_pane_render_data(&tab, 25);

    // Should handle gracefully - either clamp or produce empty display_rows
    assert_eq!(
        render_data.displayed_row_count, 1,
        "Should have 1 filtered row"
    );
    // The viewport calculation with selected=500 and total=1 means start > end, so empty slice is correct
    // This is handled by the calling code (handlers should clamp selected), but render should not panic
}
