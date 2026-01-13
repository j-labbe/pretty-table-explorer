//! Database module for PostgreSQL connections.
//!
//! Provides synchronous database operations using the postgres crate.
//! Uses NoTls for connections (suitable for local development).

use crate::parser::TableData;
use postgres::{Client, NoTls};

/// Connect to a PostgreSQL database.
///
/// Accepts connection strings in either key=value format:
///   `"host=localhost user=postgres dbname=mydb"`
/// Or URI format:
///   `"postgresql://user:pass@host/db"`
///
/// Returns a connected Client or an error.
pub fn connect(connection_string: &str) -> Result<Client, postgres::Error> {
    Client::connect(connection_string, NoTls)
}

/// Execute a SQL query and convert results to TableData.
///
/// Returns TableData with column headers and row data.
/// Empty results return TableData with headers only (if available).
pub fn execute_query(
    client: &mut Client,
    query: &str,
) -> Result<TableData, Box<dyn std::error::Error>> {
    let rows = client.query(query, &[])?;

    // Extract column names from query result columns
    let headers: Vec<String> = if !rows.is_empty() {
        rows[0]
            .columns()
            .iter()
            .map(|col| col.name().to_string())
            .collect()
    } else {
        // For empty results, we can still get column info from a prepared statement
        // But for simplicity, return empty headers for truly empty results
        Vec::new()
    };

    // Convert rows to string vectors
    let data_rows: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            (0..row.columns().len())
                .map(|i| {
                    // Try to get value as string, handling NULL values
                    // postgres crate allows getting most types as String via Display
                    row.try_get::<_, Option<String>>(i)
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| {
                            // Try other common types if String fails
                            row.try_get::<_, Option<i32>>(i)
                                .ok()
                                .flatten()
                                .map(|v| v.to_string())
                                .or_else(|| {
                                    row.try_get::<_, Option<i64>>(i)
                                        .ok()
                                        .flatten()
                                        .map(|v| v.to_string())
                                })
                                .or_else(|| {
                                    row.try_get::<_, Option<f64>>(i)
                                        .ok()
                                        .flatten()
                                        .map(|v| v.to_string())
                                })
                                .or_else(|| {
                                    row.try_get::<_, Option<bool>>(i)
                                        .ok()
                                        .flatten()
                                        .map(|v| v.to_string())
                                })
                                .unwrap_or_else(|| "NULL".to_string())
                        })
                })
                .collect()
        })
        .collect();

    Ok(TableData {
        headers,
        rows: data_rows,
    })
}
