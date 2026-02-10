use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use pretty_table_explorer::parser::TableData;
use pretty_table_explorer::render::{build_pane_render_data, calculate_auto_widths};
use pretty_table_explorer::workspace::{Tab, ViewMode};

/// Create a TableData struct directly for benchmarking (no parsing overhead).
fn create_test_table(num_rows: usize, num_cols: usize) -> TableData {
    use lasso::Rodeo;
    let headers: Vec<String> = (1..=num_cols).map(|i| format!("col_{}", i)).collect();

    let mut interner = Rodeo::default();
    let rows = (1..=num_rows)
        .map(|row| {
            (1..=num_cols)
                .map(|col| interner.get_or_intern(format!("val_{}_{}", row, col)))
                .collect()
        })
        .collect();

    TableData {
        headers,
        rows,
        interner,
    }
}

/// Benchmark column width calculation (scanning all cells for max width).
fn bench_column_width_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("column_width_calculation");

    // Test with different row counts (fixed 10 columns)
    for num_rows in [1_000, 10_000, 100_000] {
        let data = create_test_table(num_rows, 10);

        group.bench_with_input(BenchmarkId::new("rows", num_rows), &data, |b, data| {
            b.iter(|| {
                let widths = calculate_auto_widths(black_box(data));
                black_box(widths)
            });
        });
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

/// Benchmark viewport-windowed render data at different dataset sizes.
/// Proves render time is O(viewport_size), not O(dataset_size).
/// All tests use viewport_height=50 (typical terminal height).
fn bench_viewport_render_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("viewport_render_scaling");
    let viewport_height: usize = 50; // Typical terminal height

    // Test with drastically different dataset sizes
    // If viewport windowing works, all should take similar time
    for num_rows in [1_000, 10_000, 100_000, 500_000] {
        let data = create_test_table(num_rows, 10);
        let mut tab = Tab::new("benchmark".to_string(), data, ViewMode::PipeData);
        // Initialize cached widths (as the app would in practice)
        tab.update_cached_widths();

        // Position cursor at middle of dataset
        tab.table_state.select(Some(num_rows / 2));

        group.bench_with_input(BenchmarkId::new("rows", num_rows), &tab, |b, tab| {
            b.iter(|| {
                let render_data = build_pane_render_data(black_box(tab), viewport_height);
                black_box(render_data)
            });
        });
    }

    group.finish();
}

/// Benchmark render data at different scroll positions (top, middle, bottom).
/// Verifies no performance degradation at dataset boundaries.
fn bench_viewport_render_at_boundaries(c: &mut Criterion) {
    let mut group = c.benchmark_group("viewport_render_boundaries");
    let viewport_height: usize = 50;
    let num_rows: usize = 100_000;
    let data = create_test_table(num_rows, 10);

    for (label, position) in [
        ("top", 0usize),
        ("middle", num_rows / 2),
        ("bottom", num_rows - 1),
    ] {
        let mut tab = Tab::new("benchmark".to_string(), data.clone(), ViewMode::PipeData);
        tab.update_cached_widths();
        tab.table_state.select(Some(position));

        group.bench_with_input(BenchmarkId::new("position", label), &tab, |b, tab| {
            b.iter(|| {
                let render_data = build_pane_render_data(black_box(tab), viewport_height);
                black_box(render_data)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_column_width_calculation,
    bench_build_render_data,
    bench_viewport_render_scaling,
    bench_viewport_render_at_boundaries
);
criterion_main!(benches);
