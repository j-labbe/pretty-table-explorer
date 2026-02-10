use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use pretty_table_explorer::column::ColumnConfig;
use pretty_table_explorer::parser::TableData;
use pretty_table_explorer::render::build_pane_render_data;
use pretty_table_explorer::workspace::{Tab, ViewMode};

/// Create a test table with some rows containing a special marker for filtering.
fn create_test_table_with_filter(num_rows: usize, num_cols: usize) -> TableData {
    use lasso::Rodeo;
    let headers: Vec<String> = (1..=num_cols).map(|i| format!("col_{}", i)).collect();

    let mut interner = Rodeo::default();
    let rows = (1..=num_rows)
        .map(|row| {
            (1..=num_cols)
                .map(|col| {
                    // Mark ~10% of rows with "special_value" for filtering
                    if row % 10 == 0 {
                        interner.get_or_intern(format!("special_value_{}_{}", row, col))
                    } else {
                        interner.get_or_intern(format!("val_{}_{}", row, col))
                    }
                })
                .collect()
        })
        .collect();

    TableData {
        headers,
        rows,
        interner,
    }
}

/// Benchmark row filtering performance (search/filter operations).
/// Measures how quickly we can filter display_rows based on search text.
fn bench_row_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("row_filtering");

    // Test with different row counts (fixed 10 columns)
    for num_rows in [1_000, 10_000, 100_000] {
        let data = create_test_table_with_filter(num_rows, 10);
        let mut tab = Tab::new("benchmark".to_string(), data, ViewMode::PipeData);
        // Set filter to match ~10% of rows
        tab.filter_text = "special".to_string();

        group.bench_with_input(BenchmarkId::new("rows", num_rows), &tab, |b, tab| {
            b.iter(|| {
                // build_pane_render_data performs the filtering internally
                let render_data = build_pane_render_data(black_box(tab), usize::MAX);
                black_box(render_data)
            });
        });
    }

    group.finish();
}

/// Benchmark column configuration operations.
/// These are typically fast but establish baseline for column count scaling.
fn bench_column_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("column_operations");

    // Test with different column counts
    for num_cols in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("new", num_cols),
            &num_cols,
            |b, &num_cols| {
                b.iter(|| {
                    let config = ColumnConfig::new(black_box(num_cols));
                    black_box(config)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("visible_indices", num_cols),
            &num_cols,
            |b, &num_cols| {
                let config = ColumnConfig::new(num_cols);
                b.iter(|| {
                    let indices = black_box(&config).visible_indices();
                    black_box(indices)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("hide_show_cycle", num_cols),
            &num_cols,
            |b, &num_cols| {
                b.iter(|| {
                    let mut config = ColumnConfig::new(num_cols);
                    // Hide first half, then show all
                    for i in 0..num_cols / 2 {
                        black_box(&mut config).hide(i);
                    }
                    black_box(&mut config).show_all();
                    black_box(config)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_row_filtering, bench_column_operations);
criterion_main!(benches);
