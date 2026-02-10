# Technology Stack: Performance Optimization for 1.8M+ Rows

**Project:** Pretty Table Explorer
**Researched:** 2026-02-09
**Confidence:** HIGH

## Executive Summary

For optimizing 1.8M+ row datasets in a Rust TUI, focus on streaming I/O, lazy evaluation, and profiling infrastructure. The existing stack (ratatui v0.29, crossterm v0.28) is solid—no framework changes needed. Add targeted performance libraries: streaming parsers to avoid loading all data into memory, profiling tools to identify bottlenecks, and selective use of specialized allocators only where measurements prove necessary.

**Key principle:** Measure first, optimize second. The stdlib BufReader with tuned buffer sizes handles most streaming I/O. Only add complexity (arena allocators, parallel processing) after profiling shows specific bottlenecks.

---

## Core Performance Stack

### Streaming I/O & Parsing

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **std::io::BufReader** | stdlib | Buffered stdin reading | Built-in, zero dependencies. With `with_capacity(256KB-1MB)` handles 1.8M rows efficiently. **Use this first.** |
| **bytes** | 1.11.0 | Zero-copy byte slicing | For reference-counted buffers when sharing data between components without copying. Integrates with nom. |
| **csv** | 1.4.0 | CSV streaming parser | If parsing CSV format. Fast (241 MB/s raw, 146 MB/s byte records). Supports lazy iteration. **Already in use—verify streaming API usage.** |

**Integration note:** BufReader with 256KB-1MB buffer + lazy iterators eliminates memory spikes. The csv crate already provides streaming APIs—ensure code uses `Reader::records()` iterator, not collecting to `Vec`.

### Memory Efficiency

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **memmap2** | 0.9.9 | Memory-mapped I/O | **Consider only** if profiling shows I/O bottleneck. Most popular mmap crate (187M downloads). Cross-platform. **Trade-off:** Adds unsafe code, OS-dependent behavior. |
| **bumpalo** | 3.19.0 | Arena allocator | **Consider only** if profiling shows allocation bottleneck. 11 instructions/allocation vs stdlib overhead. **Trade-off:** No Drop, manual lifetime management. |
| **smallvec** | 1.15.1 | Stack-allocated vectors | For small, known-size collections (e.g., row cells). Avoids heap allocation for ≤N items. Use `SmallVec<[T; 8]>` for typical table rows. |
| **ahash** | 0.8.12 | Fast hashing for HashMap | If using HashMaps for data indexing. 1.5-5x faster than SipHash. DOS-resistant. Hardware-accelerated (AES-NI). |

**Recommendation:** Start without memmap2/bumpalo. Add only after:
1. Profiling shows allocations/I/O are bottleneck (use dhat/flamegraph)
2. Baseline optimization (BufReader tuning, lazy evaluation) exhausted
3. Complexity cost justified by >30% measured speedup

### Data Processing

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **rayon** | 1.11.0 | Parallel processing | **Optional.** If profiling shows CPU-bound parsing. Work-stealing parallelism. **Warning:** Overhead neutralizes gains for small workloads. Test `par_iter()` vs `iter()` with benchmarks. |
| **itertools** | 0.14.0 | Lazy iterator combinators | For chunking, windowing, batching without allocating intermediate collections. `chunks()`, `batching()` methods. Memory-efficient. |

**When to use rayon:** Only if:
- Parsing dominates (>50% CPU time in flamegraph)
- Input >1M rows
- Benchmark shows >20% improvement
- Thread pool overhead measured as acceptable

Otherwise, single-threaded lazy iteration is simpler and sufficient.

---

## Profiling & Benchmarking Infrastructure

**CRITICAL:** These are required dependencies. Performance optimization without measurement is guesswork.

### Benchmarking

| Technology | Version | Purpose | When to Use |
|------------|---------|---------|-------------|
| **criterion** | 0.8.1 | Statistical micro-benchmarks | **Required.** Measure specific functions (parsing, rendering). Detects regressions. Use `--quick` mode for fast iteration (50x faster). |

**Setup:**
```toml
[dev-dependencies]
criterion = { version = "0.8.1", features = ["html_reports"] }

[[bench]]
name = "parse_benchmark"
harness = false
```

**Usage:**
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn parse_large_input(c: &mut Criterion) {
    let input = /* 1.8M row sample */;
    c.bench_function("parse_1.8M_rows", |b| {
        b.iter(|| parse_input(black_box(&input)))
    });
}

criterion_group!(benches, parse_large_input);
criterion_main!(benches);
```

### Profiling Tools

| Tool | Version | Purpose | Installation |
|------|---------|---------|--------------|
| **cargo-flamegraph** | 0.6.11 | CPU profiling, flame graphs | `cargo install flamegraph` |
| **inferno** | 0.12.3 | Flamegraph generation (used by cargo-flamegraph) | Auto-installed as dependency |
| **dhat** | 0.3.3 | Heap profiling, allocation tracking | Add as dev-dependency |
| **pprof** | 0.15.0 | CPU profiling with criterion integration | Optional, if criterion integration needed |

**Profiling workflow:**

1. **CPU profiling:**
```bash
# Linux (uses perf)
cargo flamegraph --bin pretty-table-explorer -- < large_input.txt

# Generates flamegraph.svg showing hot paths
```

2. **Heap profiling:**
```rust
#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    // Your code here
}
```
Run: `cargo run --features dhat-heap < large_input.txt`
Output: `dhat-heap.json` (view in DHAT viewer)

3. **Integration testing:**
```rust
#[cfg(test)]
mod tests {
    use dhat::{HeapStats, Profiler};

    #[test]
    fn test_memory_usage() {
        let _profiler = Profiler::new_heap();

        parse_large_input();

        let stats = HeapStats::get();
        assert!(stats.curr_bytes < 100_000_000); // <100MB
        assert!(stats.curr_blocks < 10_000);      // <10K allocations
    }
}
```

---

## Ratatui-Specific Performance Patterns

### Virtual Scrolling for Large Tables

**Problem:** ratatui's `Table` widget converts iterators to `Vec` during rendering, causing lag with >10K items.

**Solution:** Only render visible rows.

```rust
use ratatui::widgets::{Table, Row, TableState};

fn render_table(frame: &mut Frame, area: Rect, state: &mut TableState, data: &[RowData]) {
    let visible_rows = calculate_visible_rows(area.height);
    let offset = state.offset();

    // Only create Row objects for visible data
    let rows: Vec<Row> = data
        .iter()
        .skip(offset)
        .take(visible_rows)
        .map(|row_data| Row::new(row_data.cells.clone()))
        .collect();

    let table = Table::new(rows, /* widths */);
    frame.render_stateful_widget(table, area, state);
}
```

**Ratatui performance characteristics:**
- Sub-millisecond rendering via intelligent diffing
- Minimal ANSI sequences
- 60+ FPS with complex layouts (Ratatui vs Bubbletea benchmarks show 30-40% less memory, 15% less CPU)

**Avoid:**
- Full dataset iteration per frame
- Cloning all data into `Vec<Row>` upfront
- Rendering off-screen rows

### State Management for Large Datasets

**Pattern:** Separate data storage from UI state.

```rust
struct AppState {
    // Full dataset (streaming iterator or lazy structure)
    data_source: Box<dyn Iterator<Item = RowData>>,

    // UI state (lightweight)
    table_state: TableState,
    scroll_offset: usize,

    // Cached visible window (only what's rendered)
    visible_cache: VecDeque<RowData>,
    cache_start: usize,
}

impl AppState {
    fn update_visible_cache(&mut self, area_height: u16) {
        let visible_count = area_height as usize;
        let target_offset = self.table_state.offset();

        // Only fetch/cache visible rows + small buffer
        if !self.cache_contains(target_offset, visible_count) {
            self.refill_cache(target_offset, visible_count + 20);
        }
    }
}
```

**Key insight:** ratatui's immediate-mode rendering means you control data access per frame. Use this to implement lazy loading.

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| **Memory-mapped I/O** | memmap2 0.9.9 | mmap-io, mmap-rs | memmap2: most popular (187M downloads), battle-tested. Others: less adoption, similar features. |
| **Arena allocator** | bumpalo 3.19.0 | typed-arena | bumpalo: most downloaded, fastest (11 instructions). typed-arena: type-safe but slower. |
| **Parallel processing** | rayon 1.11.0 | crossbeam, std::thread | rayon: simplest API (`par_iter()`), work-stealing. Manual threading unnecessary complexity. |
| **Hashing** | ahash 0.8.12 | fxhash, seahash | ahash: best balance of speed + DOS resistance. fxhash: faster but no DOS protection. seahash: slower. |
| **Bytes** | bytes 1.11.0 | std::sync::Arc<[u8]> | bytes: purpose-built zero-copy slicing. Arc: manual, less ergonomic. |
| **Mutex** | parking_lot 0.12.5 | std::sync::Mutex | parking_lot: 1.5-5x faster, smaller (1 byte). **Consider if** profiling shows lock contention. |

**Not recommended:**
- **nom** (parser combinators): Overkill for line-based psql output. BufReader + `lines()` or csv crate sufficient.
- **crossbeam-channel**: Only needed for multi-producer/consumer patterns. TUI is single-threaded event loop.
- **memmap2** (initial implementation): Start with BufReader. Add mmap only if I/O bottleneck proven.

---

## Installation

### Core Performance (add selectively after profiling)

```toml
[dependencies]
# Existing dependencies (keep as-is)
ratatui = "0.29"
crossterm = "0.28"
postgres = "0.19"
clap = "4"
ureq = "2"
csv = "1"
serde = "1"
serde_json = "1"

# Add for performance (only after measuring need)
bytes = "1.11"           # If zero-copy sharing needed
smallvec = "1.15"        # For stack-allocated small vecs
ahash = "0.8"            # If using HashMaps for indexing
itertools = "0.14"       # For lazy iteration patterns

# Optional (add only if profiling shows need)
# memmap2 = "0.9"        # If I/O bottleneck proven
# bumpalo = "3.19"       # If allocation bottleneck proven
# rayon = "1.11"         # If CPU-bound parsing proven
# parking_lot = "0.12"   # If lock contention proven

[dev-dependencies]
# Required for optimization workflow
criterion = { version = "0.8.1", features = ["html_reports"] }
dhat = "0.3"

[[bench]]
name = "parse_benchmark"
harness = false
```

### Profiling Tools (CLI installation)

```bash
# Flamegraph generation
cargo install flamegraph

# Verify installation
cargo flamegraph --version  # Should show 0.6.11
```

**Platform notes:**
- **Linux:** Uses `perf`. May need `sudo sysctl kernel.perf_event_paranoid=1`
- **macOS:** Uses `dtrace`. May need SIP adjustments.
- **Windows:** Limited support. Use WSL or dhat only.

---

## Integration Strategy

### Phase 1: Measure Baseline
1. Add criterion benchmarks for current parsing code
2. Run `cargo flamegraph` on 1.8M row input
3. Run dhat heap profiling
4. Document hotspots: parsing? allocation? rendering?

### Phase 2: Low-Hanging Fruit
1. **BufReader tuning:**
   ```rust
   let stdin = io::stdin();
   let reader = BufReader::with_capacity(512 * 1024, stdin); // 512KB buffer
   ```
2. **Lazy evaluation:** Ensure iteration, not collection to `Vec`
3. **Virtual scrolling:** Render only visible rows (see Ratatui patterns above)

### Phase 3: Targeted Optimization (only if Phase 2 insufficient)
1. If allocation bottleneck → Add bumpalo for parsed row arena
2. If I/O bottleneck → Add memmap2 for large files
3. If CPU bottleneck → Add rayon for parallel parsing
4. Benchmark each addition with criterion

### Phase 4: Validation
1. Re-run flamegraph/dhat
2. Compare criterion benchmarks (before/after)
3. Ensure <100ms initial render, 60 FPS scrolling with 1.8M rows

---

## What NOT to Add

❌ **nom** — Parser combinators add complexity. BufReader + `lines()` or csv crate handles psql output.

❌ **tokio/async** — TUI is synchronous event loop. Async adds overhead without benefit.

❌ **crossbeam-channel** — No multi-producer/consumer pattern in TUI. Rayon handles parallel iteration if needed.

❌ **jemallocator** — Only beneficial for multi-threaded allocation-heavy workloads. TUI is single-threaded. Measure first.

❌ **serde_json streaming** — Only if parsing JSON (psql outputs table format). csv crate handles tabular data.

❌ **polars/datafusion** — Overkill. These are DataFrame libraries for analytics. TUI just needs lazy iteration.

---

## Performance Targets

Based on 1.8M rows piped from psql:

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Initial render** | <500ms | Time from stdin start to first frame |
| **Scroll FPS** | 60 FPS | Ratatui frame render time <16ms |
| **Memory usage** | <200MB peak | dhat curr_bytes |
| **Allocations** | <100K total | dhat curr_blocks |
| **Parsing throughput** | >50MB/s | criterion benchmark |

**Rationale:**
- Initial render: User perceives <500ms as "instant"
- 60 FPS: Smooth scrolling (16.6ms/frame)
- Memory: 200MB = ~110 bytes/row average (reasonable for metadata + visible cache)
- Allocations: Minimize churn, favor reuse
- Throughput: csv crate does 146MB/s for byte records—50MB/s leaves headroom for app logic

---

## Verification Checklist

Before considering optimization complete:

- [ ] Criterion benchmarks show <500ms parsing for 1.8M rows
- [ ] Flamegraph shows no single function >30% of CPU time
- [ ] dhat shows peak memory <200MB
- [ ] Scrolling maintains 60 FPS (16ms frame time) with full dataset
- [ ] No unnecessary allocations in hot path (check dhat allocation sites)
- [ ] BufReader buffer size tuned (tested 64KB, 256KB, 512KB, 1MB)
- [ ] Only visible rows rendered (verified in code review)
- [ ] Lazy iteration used (no `.collect::<Vec<_>>()` in hot path)

---

## Sources

### Streaming & Parsing
- [Rust csv crate documentation](https://docs.rs/csv)
- [BurntSushi CSV performance analysis](https://burntsushi.net/csv/)
- [Rust High Performance - Parsing byte streams](https://www.oreilly.com/library/view/rust-high-performance/9781788399487/20afc661-8303-4e80-8894-fb7ae14426e4.xhtml)
- [High-Performance JSON Parsing in Rust](https://elitedev.in/rust/high-performance-json-parsing-in-rust-memory-effi/)

### Memory Efficiency
- [memmap2 crate](https://crates.io/crates/memmap2)
- [bumpalo GitHub repository](https://github.com/fitzgen/bumpalo)
- [Guide to using arenas in Rust](https://blog.logrocket.com/guide-using-arenas-rust/)
- [bytes crate documentation](https://docs.rs/bytes)
- [Rust: Efficient Zero-Copy Parsing with nom and bytes](https://byteblog.medium.com/rust-efficient-zero-copy-parsing-with-nom-and-bytes-62e47d31221d)

### Parallel Processing
- [rayon GitHub repository](https://github.com/rayon-rs/rayon)
- [Implementing data parallelism with Rayon](https://blog.logrocket.com/implementing-data-parallelism-rayon-rust/)
- [Optimization adventures: making a parallel Rust workload 10x faster](https://gendignoux.com/blog/2024/11/18/rust-rayon-optimized.html)

### Profiling & Benchmarking
- [criterion crate](https://crates.io/crates/criterion)
- [How to Profile Rust Applications with perf, flamegraph, and samply](https://oneuptime.com/blog/post/2026-01-07-rust-profiling-perf-flamegraph/view)
- [Profiling Rust Like a Pro: Criterion, Flamegraphs, and Cachegrind](https://ritik-chopra28.medium.com/profiling-rust-like-a-pro-criterion-flamegraphs-and-cachegrind-9c117dc82a33)
- [cargo-flamegraph GitHub repository](https://github.com/flamegraph-rs/flamegraph)
- [inferno GitHub repository](https://github.com/jonhoo/inferno)
- [dhat-rs GitHub repository](https://github.com/nnethercote/dhat-rs)
- [The Rust Performance Book - Profiling](https://nnethercote.github.io/perf-book/profiling.html)

### Ratatui Performance
- [Ratatui official site](https://ratatui.rs/)
- [Go vs. Rust for TUI Development: Bubbletea and Ratatui](https://dev-tngsh.medium.com/go-vs-rust-for-tui-development-a-deep-dive-into-bubbletea-and-ratatui-9af65c0b535b)
- [Table performance issue #1004](https://github.com/ratatui/ratatui/issues/1004)
- [Ratatui StatefulWidget documentation](https://docs.rs/ratatui/latest/ratatui/widgets/trait.StatefulWidget.html)

### Hashing & Data Structures
- [ahash crate](https://crates.io/crates/ahash)
- [Hashing algorithms for HashMap in Rust](https://medium.com/@guleym/hashing-algorithms-for-hashmap-in-rust-a-deep-dive-into-performance-and-security-3ae181798bb9)
- [smallvec GitHub repository](https://github.com/servo/rust-smallvec)
- [parking_lot GitHub repository](https://github.com/Amanieu/parking_lot)
- [The Rust Performance Book - Hashing](https://nnethercote.github.io/perf-book/hashing.html)

### I/O Performance
- [Using BufRead for faster Rust I/O](https://blog.logrocket.com/using-bufread-faster-rust-io-speed/)
- [BufReader documentation](https://doc.rust-lang.org/std/io/struct.BufReader.html)
- [Standard I/O in Rust: Practical Patterns](https://thelinuxcode.com/standard-io-in-rust-practical-patterns-for-stdin-stdout-and-stderr/)

### Iterators
- [itertools crate](https://crates.io/crates/itertools)
- [The Rust Performance Book - Iterators](https://nnethercote.github.io/perf-book/iterators.html)
- [Performance in Loops vs. Iterators](https://doc.rust-lang.org/book/ch13-04-performance.html)
