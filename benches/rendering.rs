use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use pretty_table_explorer::parser::TableData;
use pretty_table_explorer::render::{build_pane_render_data, calculate_auto_widths};
use pretty_table_explorer::workspace::{Tab, ViewMode};

/// Create a TableData struct directly for benchmarking (no parsing overhead).
fn create_test_table(num_rows: usize, num_cols: usize) -> TableData {
    let headers: Vec<String> = (1..=num_cols).map(|i| format!("col_{}", i)).collect();

    let rows: Vec<Vec<String>> = (1..=num_rows)
        .map(|row| {
            (1..=num_cols)
                .map(|col| format!("val_{}_{}", row, col))
                .collect()
        })
        .collect();

    TableData { headers, rows }
}

/// Benchmark column width calculation (scanning all cells for max width).
fn bench_column_width_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("column_width_calculation");

    // Test with different row counts (fixed 10 columns)
    for num_rows in [1_000, 10_000, 100_000] {
        let data = create_test_table(num_rows, 10);

        group.bench_with_input(
            BenchmarkId::new("rows", num_rows),
            &data,
            |b, data| {
                b.iter(|| {
                    let widths = calculate_auto_widths(black_box(data));
                    black_box(widths)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark building render data (full render preparation path).
fn bench_build_render_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_render_data");

    // Test with different row counts (fixed 10 columns)
    for num_rows in [1_000, 10_000, 100_000] {
        let data = create_test_table(num_rows, 10);
        let tab = Tab::new("benchmark".to_string(), data, ViewMode::PipeData);

        group.bench_with_input(BenchmarkId::new("rows", num_rows), &tab, |b, tab| {
            b.iter(|| {
                let render_data = build_pane_render_data(black_box(tab), usize::MAX);
                black_box(render_data)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_column_width_calculation,
    bench_build_render_data
);
criterion_main!(benches);
