//! Integration tests for search/filter functionality.
//!
//! These tests verify that search filtering correctly identifies matching rows
//! and handles edge cases like case-insensitivity, empty filters, and empty input.

use pretty_table_explorer::parser::parse_psql;
use pretty_table_explorer::render::build_pane_render_data;
use pretty_table_explorer::workspace::{Tab, ViewMode};

/// Sample psql output with varied content for testing search functionality
const SAMPLE_PSQL_OUTPUT: &str = r#"
 id | name    | age | city
----+---------+-----+-------------
 1  | Alice   | 30  | New York
 2  | Bob     | 25  | Seattle
 3  | Charlie | 35  | Portland
 4  | alice   | 28  | Boston
 5  | David   | 42  | Chicago
 6  | Eve     | 31  | Austin
 7  | Frank   | 29  | Denver
 8  | Grace   | 33  | Miami
 9  | Henry   | 27  | Phoenix
 10 | Ivy     | 36  | Dallas
(10 rows)
"#;

#[test]
fn test_parse_and_filter_matching_rows() {
    // Parse sample data
    let table_data = parse_psql(SAMPLE_PSQL_OUTPUT).expect("Failed to parse sample data");

    // Create Tab with filter for "alice" (should match rows 1 and 4)
    let mut tab = Tab::new("Test".to_string(), table_data, ViewMode::PipeData);
    tab.filter_text = "alice".to_string();

    // Build render data and check filtered results
    let render_data = build_pane_render_data(&tab, usize::MAX);

    assert_eq!(
        render_data.displayed_row_count, 2,
        "Should match 2 rows containing 'alice'"
    );
    assert_eq!(render_data.total_rows, 10, "Total rows should be 10");
}

#[test]
fn test_filter_case_insensitive() {
    let table_data = parse_psql(SAMPLE_PSQL_OUTPUT).expect("Failed to parse sample data");

    let mut tab = Tab::new("Test".to_string(), table_data, ViewMode::PipeData);
    // Search for lowercase "alice" should match uppercase "Alice" in row 1
    tab.filter_text = "alice".to_string();

    let render_data = build_pane_render_data(&tab, usize::MAX);

    assert_eq!(
        render_data.displayed_row_count, 2,
        "Case-insensitive search should match both 'Alice' and 'alice'"
    );

    // Verify the actual matches
    assert!(render_data
        .display_rows
        .iter()
        .any(|row| row.iter().any(|cell| cell.eq_ignore_ascii_case("Alice"))));
    assert!(render_data
        .display_rows
        .iter()
        .any(|row| row.iter().any(|cell| cell.eq_ignore_ascii_case("alice"))));
}

#[test]
fn test_filter_no_matches() {
    let table_data = parse_psql(SAMPLE_PSQL_OUTPUT).expect("Failed to parse sample data");

    let mut tab = Tab::new("Test".to_string(), table_data, ViewMode::PipeData);
    tab.filter_text = "zzzznonexistent".to_string();

    let render_data = build_pane_render_data(&tab, usize::MAX);

    assert_eq!(
        render_data.displayed_row_count, 0,
        "Filter with no matches should return 0 rows"
    );
    assert_eq!(render_data.display_rows.len(), 0);
}

#[test]
fn test_filter_empty_string() {
    let table_data = parse_psql(SAMPLE_PSQL_OUTPUT).expect("Failed to parse sample data");

    let mut tab = Tab::new("Test".to_string(), table_data, ViewMode::PipeData);
    tab.filter_text = "".to_string();

    let render_data = build_pane_render_data(&tab, usize::MAX);

    assert_eq!(
        render_data.displayed_row_count, 10,
        "Empty filter should return all rows"
    );
    assert_eq!(render_data.total_rows, 10);
}

#[test]
fn test_filter_partial_match() {
    let table_data = parse_psql(SAMPLE_PSQL_OUTPUT).expect("Failed to parse sample data");

    let mut tab = Tab::new("Test".to_string(), table_data, ViewMode::PipeData);
    // Filter "ali" should match "Alice" and "alice"
    tab.filter_text = "ali".to_string();

    let render_data = build_pane_render_data(&tab, usize::MAX);

    assert_eq!(
        render_data.displayed_row_count, 2,
        "Partial match 'ali' should match 'Alice' and 'alice'"
    );
}

#[test]
fn test_parse_empty_input() {
    let result = parse_psql("");
    assert!(result.is_none(), "Empty input should return None");
}

#[test]
fn test_parse_single_row() {
    let input = r#"
 id | name
----+------
 1  | Test
(1 row)
"#;

    let table_data = parse_psql(input).expect("Failed to parse single row");

    assert_eq!(table_data.headers.len(), 2);
    assert_eq!(table_data.headers[0], "id");
    assert_eq!(table_data.headers[1], "name");
    assert_eq!(table_data.rows.len(), 1);
    assert_eq!(table_data.resolve(&table_data.rows[0][0]), "1");
    assert_eq!(table_data.resolve(&table_data.rows[0][1]), "Test");
}

#[test]
fn test_parse_multiline_psql() {
    // Verify that the footer line "(N rows)" is excluded from data rows
    let table_data = parse_psql(SAMPLE_PSQL_OUTPUT).expect("Failed to parse multiline psql output");

    assert_eq!(
        table_data.rows.len(),
        10,
        "Should have exactly 10 data rows (footer excluded)"
    );

    // Verify footer is not in the data
    for row in &table_data.rows {
        for cell in row {
            let cell_str = table_data.resolve(cell);
            assert!(
                !cell_str.contains("(10 rows)"),
                "Footer should not appear in data rows"
            );
        }
    }
}

#[test]
fn test_filter_matches_any_column() {
    let table_data = parse_psql(SAMPLE_PSQL_OUTPUT).expect("Failed to parse sample data");

    let mut tab = Tab::new("Test".to_string(), table_data, ViewMode::PipeData);
    // Filter by city name - should match across different columns
    tab.filter_text = "seattle".to_string();

    let render_data = build_pane_render_data(&tab, usize::MAX);

    assert_eq!(
        render_data.displayed_row_count, 1,
        "Should match 1 row with 'Seattle'"
    );
    assert!(render_data.display_rows[0]
        .iter()
        .any(|cell| cell.to_lowercase().contains("seattle")));
}

#[test]
fn test_filter_with_numbers() {
    let table_data = parse_psql(SAMPLE_PSQL_OUTPUT).expect("Failed to parse sample data");

    let mut tab = Tab::new("Test".to_string(), table_data, ViewMode::PipeData);
    // Filter by age
    tab.filter_text = "30".to_string();

    let render_data = build_pane_render_data(&tab, usize::MAX);

    // Should match row with age 30
    assert!(
        render_data.displayed_row_count >= 1,
        "Should match at least one row with '30'"
    );
}
