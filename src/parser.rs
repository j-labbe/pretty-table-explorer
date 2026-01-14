/// Represents parsed table data from psql output.
#[derive(Debug, Clone)]
pub struct TableData {
    /// Column headers from the first row
    pub headers: Vec<String>,
    /// Data rows (each row is a vector of cell values)
    pub rows: Vec<Vec<String>>,
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
    let mut rows: Vec<Vec<String>> = Vec::new();

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

        // Parse row by splitting on |
        let row: Vec<String> = line.split('|').map(|s| s.trim().to_string()).collect();

        rows.push(row);
    }

    Some(TableData { headers, rows })
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
        assert_eq!(table.rows[0], vec!["1", "Alice", "30"]);
        assert_eq!(table.rows[1], vec!["2", "Bob", "25"]);
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
        assert_eq!(table.rows[0], vec!["1", "2"]);
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
}
