use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use pretty_table_explorer::parser::parse_psql;

/// Generate realistic psql-formatted output for benchmarking.
///
/// Format:
/// ```text
///  col_1 | col_2 | col_3 | ...
/// -------+-------+-------+---
///  val_1_1 | val_1_2 | val_1_3 | ...
///  val_2_1 | val_2_2 | val_2_3 | ...
/// (N rows)
/// ```
fn generate_psql_output(num_rows: usize, num_cols: usize) -> String {
    let mut output = String::new();

    // Generate header row
    let headers: Vec<String> = (1..=num_cols)
        .map(|i| format!("col_{}", i))
        .collect();
    output.push_str(" ");
    output.push_str(&headers.join(" | "));
    output.push('\n');

    // Generate separator row
    let separator: Vec<String> = (1..=num_cols)
        .map(|_| "-------".to_string())
        .collect();
    output.push_str(&separator.join("+"));
    output.push('\n');

    // Generate data rows with varying cell content lengths (3-20 chars)
    for row in 1..=num_rows {
        let cells: Vec<String> = (1..=num_cols)
            .map(|col| {
                // Vary content length based on position for realism
                let base = format!("value_{}_{}", row, col);
                if col % 3 == 0 {
                    // Some cells are shorter
                    format!("v{}", row)
                } else if col % 5 == 0 {
                    // Some cells are longer
                    format!("longer_value_row{}_col{}", row, col)
                } else {
                    base
                }
            })
            .collect();
        output.push_str(" ");
        output.push_str(&cells.join(" | "));
        output.push('\n');
    }

    // Generate footer
    output.push_str(&format!("({} rows)\n", num_rows));

    output
}

/// Benchmark parsing with varying row counts (fixed 10 columns).
fn bench_parse_rows(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_psql");

    // Test with different row counts (skip 1M for CI speed)
    for num_rows in [100, 1_000, 10_000, 100_000] {
        let input = generate_psql_output(num_rows, 10);

        group.bench_with_input(
            BenchmarkId::new("rows", num_rows),
            &input,
            |b, input| {
                b.iter(|| {
                    // black_box prevents compiler from optimizing away the call
                    let result = parse_psql(black_box(input));
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark parsing with varying column counts (fixed 10,000 rows).
fn bench_parse_cols(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_psql_varying_cols");

    // Test with different column counts
    for num_cols in [3, 10, 25, 50] {
        let input = generate_psql_output(10_000, num_cols);

        group.bench_with_input(
            BenchmarkId::new("cols", num_cols),
            &input,
            |b, input| {
                b.iter(|| {
                    let result = parse_psql(black_box(input));
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_parse_rows, bench_parse_cols);
criterion_main!(benches);
