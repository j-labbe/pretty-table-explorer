use lasso::{Rodeo, Spur};

/// Represents parsed table data from psql output.
pub struct TableData {
    /// Column headers from the first row
    pub headers: Vec<String>,
    /// Data rows (each row is a vector of interned symbols)
    pub rows: Vec<Vec<Spur>>,
    /// String interner for this table's data
    pub interner: Rodeo,
}

impl TableData {
    /// Returns the number of columns in the table.
    #[allow(dead_code)]
    pub fn column_count(&self) -> usize {
        self.headers.len()
    }

    /// Returns the number of data rows in the table.
    #[allow(dead_code)]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Resolve a Spur symbol to its string value.
    pub fn resolve(&self, spur: &Spur) -> &str {
        self.interner.resolve(spur)
    }

    /// Resolve all symbols in a row to owned Strings.
    /// Used for export operations.
    #[allow(dead_code)]
    pub fn resolve_row(&self, row: &[Spur]) -> Vec<String> {
        row.iter().map(|s| self.resolve(s).to_string()).collect()
    }
}

impl Clone for TableData {
    fn clone(&self) -> Self {
        let mut new_interner = Rodeo::default();
        let new_rows: Vec<Vec<Spur>> = self
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|spur| {
                        let s = self.interner.resolve(spur);
                        new_interner.get_or_intern(s)
                    })
                    .collect()
            })
            .collect();
        TableData {
            headers: self.headers.clone(),
            rows: new_rows,
            interner: new_interner,
        }
    }
}

impl std::fmt::Debug for TableData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TableData")
            .field("headers", &self.headers)
            .field("rows_count", &self.rows.len())
            .finish()
    }
}

/// Parse psql header from the first few lines of output.
///
/// Returns `Some((headers, data_start_index))` where:
/// - `headers` is the parsed column names
/// - `data_start_index` is the line index after the separator where data rows begin
///
/// Returns `None` if headers or separator are missing/malformed.
pub fn parse_psql_header(lines: &[&str]) -> Option<(Vec<String>, usize)> {
    if lines.is_empty() {
        return None;
    }

    // Find the first non-empty line (header row)
    let mut line_iter = lines.iter().enumerate();
    let (header_idx, header_line) = line_iter.find(|(_, line)| !line.trim().is_empty())?;

    // Parse headers by splitting on |
    let headers: Vec<String> = header_line
        .split('|')
        .map(|s| s.trim().to_string())
        .collect();

    if headers.is_empty() || headers.iter().all(|h| h.is_empty()) {
        return None;
    }

    // The next line should be the separator (contains ---)
    let separator_idx = header_idx + 1;
    if separator_idx >= lines.len() {
        return None;
    }

    let separator_line = lines[separator_idx];
    if !separator_line.contains("---") {
        return None;
    }

    // Data starts after the separator
    let data_start_index = separator_idx + 1;

    Some((headers, data_start_index))
}

/// Parse a single data row from psql output.
///
/// Returns `None` for:
/// - Empty lines
/// - Footer lines (e.g., "(2 rows)")
///
/// The `column_count` parameter is accepted but not strictly enforced
/// to match existing parse_psql behavior.
pub fn parse_psql_line(line: &str, _column_count: usize) -> Option<Vec<String>> {
    let trimmed = line.trim();

    // Skip empty lines
    if trimmed.is_empty() {
        return None;
    }

    // Skip footer lines (e.g., "(2 rows)")
    if trimmed.starts_with('(') && trimmed.ends_with(')') && trimmed.contains("row") {
        return None;
    }

    // Parse row by splitting on |
    let row: Vec<String> = line.split('|').map(|s| s.trim().to_string()).collect();

    Some(row)
}

/// Parse psql output format into structured TableData.
///
/// Expected format:
/// ```text
///  column1  | column2  | column3
/// ----------+----------+---------
///  value1   | value2   | value3
///  value4   | value5   | value6
/// (2 rows)
/// ```
///
/// Returns `None` if input is empty or malformed.
pub fn parse_psql(input: &str) -> Option<TableData> {
    let lines: Vec<&str> = input.lines().collect();

    if lines.is_empty() {
        return None;
    }

    // Find the first non-empty line (header row)
    let mut line_iter = lines.iter().enumerate();
    let (header_idx, header_line) = line_iter.find(|(_, line)| !line.trim().is_empty())?;

    // Parse headers by splitting on |
    let headers: Vec<String> = header_line
        .split('|')
        .map(|s| s.trim().to_string())
        .collect();

    if headers.is_empty() || headers.iter().all(|h| h.is_empty()) {
        return None;
    }

    // The next line should be the separator (contains ---)
    let separator_idx = header_idx + 1;
    if separator_idx >= lines.len() {
        return None;
    }

    let separator_line = lines[separator_idx];
    if !separator_line.contains("---") {
        return None;
    }

    // Parse data rows (everything after separator until footer)
    let mut interner = Rodeo::default();
    let mut rows: Vec<Vec<Spur>> = Vec::new();

    for line in lines.iter().skip(separator_idx + 1) {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Stop at footer line (e.g., "(2 rows)")
        if trimmed.starts_with('(') && trimmed.ends_with(')') && trimmed.contains("row") {
            break;
        }

        // Parse row by splitting on | and interning each cell
        let row: Vec<Spur> = line
            .split('|')
            .map(|s| interner.get_or_intern(s.trim()))
            .collect();

        rows.push(row);
    }

    Some(TableData {
        headers,
        rows,
        interner,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_table() {
        let input = " id | name  | age
----+-------+-----
 1  | Alice | 30
 2  | Bob   | 25
(2 rows)";

        let result = parse_psql(input);
        assert!(result.is_some());

        let table = result.unwrap();
        assert_eq!(table.headers, vec!["id", "name", "age"]);
        assert_eq!(table.row_count(), 2);
        assert_eq!(table.column_count(), 3);

        // Resolve symbols to compare with expected strings
        let row0: Vec<String> = table.rows[0]
            .iter()
            .map(|s| table.resolve(s).to_string())
            .collect();
        assert_eq!(row0, vec!["1", "Alice", "30"]);

        let row1: Vec<String> = table.rows[1]
            .iter()
            .map(|s| table.resolve(s).to_string())
            .collect();
        assert_eq!(row1, vec!["2", "Bob", "25"]);
    }

    #[test]
    fn test_parse_single_row() {
        let input = " a | b
---+---
 1 | 2
(1 rows)";

        let result = parse_psql(input);
        assert!(result.is_some());

        let table = result.unwrap();
        assert_eq!(table.headers, vec!["a", "b"]);
        assert_eq!(table.row_count(), 1);

        let row0: Vec<String> = table.rows[0]
            .iter()
            .map(|s| table.resolve(s).to_string())
            .collect();
        assert_eq!(row0, vec!["1", "2"]);
    }

    #[test]
    fn test_parse_empty_input() {
        let result = parse_psql("");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_whitespace_only() {
        let result = parse_psql("   \n  \n  ");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_no_separator() {
        let input = " a | b
 1 | 2";
        let result = parse_psql(input);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_empty_table() {
        let input = " id | name
----+------
(0 rows)";

        let result = parse_psql(input);
        assert!(result.is_some());

        let table = result.unwrap();
        assert_eq!(table.headers, vec!["id", "name"]);
        assert_eq!(table.row_count(), 0);
    }

    #[test]
    fn test_parse_with_leading_newlines() {
        let input = "

 a | b
---+---
 1 | 2
(1 rows)";

        let result = parse_psql(input);
        assert!(result.is_some());

        let table = result.unwrap();
        assert_eq!(table.headers, vec!["a", "b"]);
        assert_eq!(table.row_count(), 1);
    }

    #[test]
    fn test_parse_row_singular() {
        // psql uses "row" when count is 1
        let input = " x
---
 1
(1 row)";

        let result = parse_psql(input);
        assert!(result.is_some());

        let table = result.unwrap();
        assert_eq!(table.row_count(), 1);
    }

    #[test]
    fn test_parse_psql_header_valid() {
        let lines = vec![
            " id | name  | age",
            "----+-------+-----",
            " 1  | Alice | 30",
        ];

        let result = parse_psql_header(&lines);
        assert!(result.is_some());

        let (headers, data_start_index) = result.unwrap();
        assert_eq!(headers, vec!["id", "name", "age"]);
        assert_eq!(data_start_index, 2);
    }

    #[test]
    fn test_parse_psql_header_no_separator() {
        let lines = vec![" id | name  | age", " 1  | Alice | 30"];

        let result = parse_psql_header(&lines);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_psql_line_data_row() {
        let line = " 1  | Alice | 30";
        let result = parse_psql_line(line, 3);
        assert!(result.is_some());

        let row = result.unwrap();
        assert_eq!(row, vec!["1", "Alice", "30"]);
    }

    #[test]
    fn test_parse_psql_line_footer() {
        let line = "(2 rows)";
        let result = parse_psql_line(line, 3);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_psql_line_empty() {
        let line = "   ";
        let result = parse_psql_line(line, 3);
        assert!(result.is_none());
    }
}
