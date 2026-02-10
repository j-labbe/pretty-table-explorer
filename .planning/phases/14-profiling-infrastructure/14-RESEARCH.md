# Phase 14: Profiling Infrastructure - Research

**Researched:** 2026-02-10
**Domain:** Rust profiling, benchmarking, and testing infrastructure
**Confidence:** HIGH

## Summary

Profiling infrastructure for Rust applications is mature and well-established, with a clear standard stack. The ecosystem provides three primary measurement tools: Criterion for statistical benchmarking, flamegraph/samply for CPU profiling, and dhat for heap profiling. All work on stable Rust and integrate cleanly with Cargo workflows.

The critical insight for TUI applications is that panic hooks MUST restore terminal state before propagating panics, otherwise terminal corruption occurs. Ratatui already handles this in recent versions, but explicit hooks are needed for crash safety.

For this phase, the primary challenge is not tool selection (the stack is well-established) but rather determining what to measure. The phase requirements specify parsing, rendering, and scroll benchmarks plus integration tests for search, export, and column operations. This is actionable and maps cleanly to Criterion's benchmark group structure.

**Primary recommendation:** Use Criterion 0.8+ for benchmarks, cargo-flamegraph for CPU profiling, dhat 0.3+ for heap profiling, and ratatui's built-in panic hook support. Structure benchmarks in `benches/` directory with groups for parsing, rendering, and scrolling operations.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| criterion | 0.8+ | Statistical benchmarking | De facto standard in Rust ecosystem, 1.4M downloads/90 days, statistics-driven regression detection |
| cargo-flamegraph | latest | CPU profiling via flamegraphs | Standard tool for visualizing hot paths, integrates with perf/DTrace/Windows tooling |
| dhat | 0.3+ | Heap profiling | Cross-platform (unlike Valgrind DHAT), minimal overhead, supports heap usage testing |
| std::panic hooks | stdlib | Terminal restoration on crash | Built into Rust stdlib, ratatui provides recipes for TUI apps |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| cargo-criterion | latest | Enhanced Criterion runner | Historical performance graphs, machine-readable output, no baselines needed |
| samply | 0.13+ | Modern CPU profiler | Alternative to flamegraph, uses Firefox Profiler UI, better macOS support |
| ratatui-testlib | latest | TUI integration testing | PTY-based testing, real terminal emulation, snapshot testing |
| cargo-samply | latest | Cargo wrapper for samply | Profile specific binary/benchmark with `cargo samply --bench` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Criterion | Built-in cargo bench | Criterion has better statistics and regression detection, cargo bench is simpler but less rigorous |
| flamegraph | samply | Samply has better UI (Firefox Profiler) and macOS support, flamegraph is simpler and more widely documented |
| dhat | Valgrind DHAT | Valgrind DHAT has more features but Linux-only, slower, requires no code changes vs dhat requires source modifications |
| ratatui-testlib | Manual PTY testing | ratatui-testlib handles PTY lifecycle automatically, manual testing gives more control but more complexity |

**Installation:**
```bash
# Core benchmarking
cargo install cargo-criterion

# CPU profiling
cargo install flamegraph
# OR
cargo install samply

# No install needed for dhat (library, not tool)
```

## Architecture Patterns

### Recommended Project Structure
```
project/
├── benches/               # Criterion benchmarks
│   ├── parsing.rs        # Parse operations (LOAD requirements)
│   ├── rendering.rs      # Render operations (REND requirements)
│   └── scrolling.rs      # Scroll operations (REND requirements)
├── tests/                # Integration tests
│   ├── search_tests.rs   # Search operations
│   ├── export_tests.rs   # Export operations
│   └── column_tests.rs   # Column operations
├── src/
│   └── main.rs           # Panic hook initialization
└── Cargo.toml            # Profile configurations
```

### Pattern 1: Criterion Benchmark Structure
**What:** Group related benchmarks, use `black_box` to prevent optimization, parameterize over input sizes
**When to use:** All performance-critical operations that need regression detection
**Example:**
```rust
// Source: https://bheisler.github.io/criterion.rs/book/getting_started.html
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

pub fn parse_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing");

    // Parameterize over dataset sizes for LOAD-01 testing
    for size in [1000, 10_000, 100_000, 1_000_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let data = generate_csv_data(size);
            b.iter(|| parse_psql(black_box(&data)));
        });
    }

    group.finish();
}

criterion_group!(benches, parse_benchmarks);
criterion_main!(benches);
```

### Pattern 2: Panic Hook for Terminal Restoration
**What:** Install panic hook early in main() to restore terminal state on crash
**When to use:** Any TUI application using alternate screen or raw mode
**Example:**
```rust
// Source: https://ratatui.rs/recipes/apps/panic-hooks/
use std::panic::{set_hook, take_hook};
use crossterm::{execute, terminal::{disable_raw_mode, LeaveAlternateScreen}};
use std::io::stdout;

fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        // Restore terminal (ignore errors to preserve panic message)
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}

fn main() {
    init_panic_hook(); // MUST be before terminal init
    // ... rest of application
}
```

### Pattern 3: dhat Heap Profiling
**What:** Wrap main with dhat profiler, use feature flags to enable/disable
**When to use:** Investigating memory allocations for MEM-01 requirement
**Example:**
```rust
// Source: https://docs.rs/dhat/latest/dhat/
#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    // Run application
    run_app();

    // Profiler drops here, writes dhat-heap.json
}
```

### Pattern 4: Integration Test Structure
**What:** Use standard Rust test organization in `tests/` directory
**When to use:** Testing multi-module workflows (search, export, column operations)
**Example:**
```rust
// Source: https://doc.rust-lang.org/book/ch11-03-test-organization.html
// tests/search_tests.rs
use pretty_table_explorer::{parser, state};

#[test]
fn test_search_filters_rows() {
    let table = parser::parse_psql(SAMPLE_DATA).unwrap();
    let mut app_state = state::create_state(table);

    // Test search operation
    state::apply_search(&mut app_state, "Alice");

    assert_eq!(app_state.filtered_rows.len(), 1);
    assert!(app_state.filtered_rows[0].contains("Alice"));
}
```

### Pattern 5: Cargo Profile Configuration
**What:** Custom profiles for benchmarking and profiling with debug symbols
**When to use:** Always - needed for meaningful flamegraphs and dhat output
**Example:**
```toml
# Source: https://nnethercote.github.io/perf-book/profiling.html
[profile.release]
debug = "line-tables-only"  # Symbols for profiling without full debug info

[profile.bench]
inherits = "release"
debug = true  # Full debug info for detailed flamegraphs

# Optional: dedicated profiling profile
[profile.profiling]
inherits = "release"
debug = true
lto = false  # Disable LTO for more readable profiles
```

### Anti-Patterns to Avoid
- **Forgetting black_box in benchmarks:** Compiler optimizes away the code being measured, results are meaningless
- **Panic hook after terminal init:** If panic occurs during init, terminal is corrupted
- **Profiling debug builds:** Performance characteristics are completely different, misleading results
- **Not using groups in Criterion:** Benchmark output becomes cluttered, comparisons are difficult
- **Strip symbols in release:** Profiling tools cannot generate meaningful output

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Statistical benchmarking | Custom timing loops with `Instant::now()` | Criterion | Handles warmup, statistical analysis, regression detection, outlier filtering, prevents compiler optimizations |
| Flamegraph generation | Manual perf + grep + awk | cargo-flamegraph or samply | Handles platform differences (perf/DTrace/Windows), proper symbol demangling, SVG generation |
| Heap profiling | Manual allocation tracking | dhat | Tracks allocation lifetimes, identifies leak sources, handles backtrace collection |
| Terminal panic handling | Custom signal handlers | std::panic hooks | Integrates with Rust panic machinery, preserves panic messages, composable with other hooks |
| Integration tests | Custom test harness | Built-in Cargo test framework | Automatic discovery, parallel execution, filtering, IDE integration |

**Key insight:** Profiling and testing infrastructure involves subtle correctness issues. Custom solutions miss edge cases like compiler optimizations in benchmarks, symbol demangling differences across platforms, allocation backtrace complexity, and panic propagation semantics. Use battle-tested tools.

## Common Pitfalls

### Pitfall 1: Compiler Optimizations in Benchmarks
**What goes wrong:** Benchmark code is optimized away, measuring nothing. Classic example: benchmarking `Vec::push` without using the result - compiler eliminates the entire operation.
**Why it happens:** LLVM sees the result is unused and removes "dead code"
**How to avoid:** Wrap inputs/outputs in `criterion::black_box()` to force computation
**Warning signs:** Benchmark shows unrealistically fast times (nanoseconds for complex operations), performance doesn't change when making code worse

### Pitfall 2: Missing Debug Symbols in Release Builds
**What goes wrong:** Flamegraphs show memory addresses or mangled names instead of function names. dhat output is unusable.
**Why it happens:** Default release profile strips debug info for smaller binaries
**How to avoid:** Add `debug = "line-tables-only"` or `debug = true` to `[profile.release]` in Cargo.toml
**Warning signs:** Profiler output shows `_ZN` prefixes or hex addresses, source line numbers missing

### Pitfall 3: Frame Pointer Optimization
**What goes wrong:** Stack traces in profilers are incomplete or incorrect, missing intermediate function calls
**Why it happens:** Rust compiler optimizes away frame pointers to save a register
**How to avoid:** Build with `RUSTFLAGS="-C force-frame-pointers=yes"` for profiling
**Warning signs:** Flamegraph shows flat structure when code has deep call stacks, functions appear to call unrelated functions

### Pitfall 4: Panic Hook Order
**What goes wrong:** Terminal remains in corrupted state after panic (raw mode enabled, alternate screen active, cursor hidden)
**Why it happens:** Panic hook installed after terminal initialization, or restoration errors are propagated
**How to avoid:** Call `init_panic_hook()` BEFORE any terminal initialization, ignore restoration errors (`let _ = restore()`)
**Warning signs:** Terminal needs `reset` command after crash, panic messages are invisible, cursor disappears

### Pitfall 5: System Call Sampling
**What goes wrong:** Flamegraph shows minimal CPU usage when application is clearly busy with I/O
**Why it happens:** Default perf sampling misses time in kernel space (system calls)
**How to avoid:** Use `cargo flamegraph --root` to sample with sudo privileges
**Warning signs:** Flamegraph shows gaps, I/O-heavy code doesn't appear proportionally large

### Pitfall 6: Lockstep Sampling
**What goes wrong:** Profiler samples the same code location repeatedly instead of representative coverage
**Why it happens:** Sampling frequency aligns with program's loop frequency
**How to avoid:** Use default sampling rates (profilers handle this), avoid very low frequencies
**Warning signs:** Flamegraph shows single hot spot that doesn't match expected behavior

### Pitfall 7: Benchmarking Without Warmup
**What goes wrong:** First benchmark iteration is much slower due to cold caches, lazy initialization
**Why it happens:** Criterion handles this automatically, but custom benchmarks often don't
**How to avoid:** Use Criterion (it does warmup automatically), or implement manual warmup iterations
**Warning signs:** High variance in benchmark results, first run significantly slower

### Pitfall 8: Integration Test Isolation
**What goes wrong:** Tests pass individually but fail when run together due to shared state
**Why it happens:** Tests modify global state, file system, or environment variables
**How to avoid:** Each test should be self-contained, use temporary directories, avoid global state
**Warning signs:** Tests fail non-deterministically, `cargo test -- --test-threads=1` fixes failures

## Code Examples

Verified patterns from official sources:

### Benchmark with Input Parameterization
```rust
// Source: https://bheisler.github.io/criterion.rs/book/getting_started.html
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_parse_with_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_psql");

    for size in [100, 1_000, 10_000, 100_000, 1_000_000, 1_800_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                let input = generate_test_data(size);
                b.iter(|| {
                    let table = parse_psql(black_box(&input));
                    black_box(table); // Prevent optimization of result
                });
            }
        );
    }

    group.finish();
}

criterion_group!(benches, bench_parse_with_sizes);
criterion_main!(benches);
```

### dhat Heap Profiling with Feature Flag
```rust
// Source: https://docs.rs/dhat/latest/dhat/
// Cargo.toml
// [features]
// dhat-heap = ["dhat"]
//
// [dependencies]
// dhat = { version = "0.3", optional = true }

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    run_application();

    // On drop, profiler writes dhat-heap.json
    // View at: https://nnethercote.github.io/dh_view/dh_view.html
}

// Run with: cargo run --features dhat-heap
```

### Panic Hook for TUI Terminal Restoration
```rust
// Source: https://ratatui.rs/recipes/apps/panic-hooks/
use crossterm::{
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};
use std::io::{self, stdout};
use std::panic::{set_hook, take_hook};

pub fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal(); // Ignore errors
        original_hook(panic_info);
    }));
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn main() {
    init_panic_hook(); // FIRST - before terminal init

    let mut terminal = init_terminal().unwrap();
    // ... application code
    restore_terminal(&mut terminal).unwrap();
}
```

### Integration Test for Search Operation
```rust
// Source: https://doc.rust-lang.org/book/ch11-03-test-organization.html
// tests/search_integration.rs

use pretty_table_explorer::parser::parse_psql;
use pretty_table_explorer::state::SearchState;

#[test]
fn test_search_finds_matching_rows() {
    // Arrange
    let input = " id | name  | age
----+-------+-----
 1  | Alice | 30
 2  | Bob   | 25
 3  | Alice | 28
(3 rows)";

    let table = parse_psql(input).expect("Parse should succeed");

    // Act
    let results = search_table(&table, "Alice");

    // Assert
    assert_eq!(results.len(), 2);
    assert!(results[0].contains("Alice"));
    assert!(results[1].contains("Alice"));
}

#[test]
fn test_search_case_insensitive() {
    let table = create_test_table();
    let results = search_table(&table, "alice");
    assert_eq!(results.len(), 2);
}

#[test]
fn test_search_no_matches() {
    let table = create_test_table();
    let results = search_table(&table, "xyz");
    assert_eq!(results.len(), 0);
}
```

### Cargo Profile Configuration
```toml
# Source: https://nnethercote.github.io/perf-book/profiling.html
[profile.release]
debug = "line-tables-only"  # Include line info for profiling

[profile.bench]
inherits = "release"
debug = true  # Full debug for flamegraphs

# Custom profile for detailed profiling
[profile.profiling]
inherits = "release"
debug = true
lto = false           # Disable LTO for readable profiles
codegen-units = 1     # Better optimization

# Build with: cargo build --profile profiling
# Profile with: samply record ./target/profiling/pte
```

### Running Profiling Tools
```bash
# Source: https://github.com/flamegraph-rs/flamegraph
# Benchmark with Criterion
cargo bench

# Generate flamegraph for entire binary
cargo flamegraph

# Profile specific benchmark
cargo flamegraph --bench parsing -- --bench

# Use samply (alternative)
cargo install samply
cargo samply --bench parsing

# Heap profiling with dhat
cargo run --features dhat-heap
# Opens: https://nnethercote.github.io/dh_view/dh_view.html
# Load: dhat-heap.json

# Profile with root for system calls
cargo flamegraph --root

# Integration tests
cargo test --test search_integration
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual timing with Instant::now() | Criterion statistical benchmarking | ~2019 | Automatic regression detection, statistical rigor |
| Valgrind DHAT | dhat-rs crate | 2020 | Cross-platform support, faster profiling |
| perf with manual flamegraph generation | cargo-flamegraph | ~2019 | One-command profiling, automatic platform detection |
| Custom panic handlers | Ratatui panic hook recipes | 2023 | Standardized patterns, less error-prone |
| No TUI integration testing | ratatui-testlib | 2024-2025 | PTY-based testing, snapshot support |
| cargo bench (built-in) | Criterion | Ongoing adoption | Better stats, but cargo bench still useful for simple cases |
| flamegraph-only | flamegraph AND samply | 2024+ | samply offers better UI via Firefox Profiler |

**Deprecated/outdated:**
- **cargo bench without criterion:** Still works but lacks statistical rigor, use Criterion for performance-sensitive code
- **Profiling without debug symbols:** Modern profilers require symbols for useful output
- **Manual perf invocation:** Use cargo-flamegraph or samply instead for better ergonomics
- **Global panic handlers without ratatui patterns:** Use ratatui's panic hook recipes for TUI apps

## Open Questions

1. **How to benchmark scroll performance with partial rendering?**
   - What we know: REND-02 requires constant-time rendering regardless of dataset size
   - What's unclear: How to measure rendering time in isolation from terminal I/O
   - Recommendation: Benchmark the render calculation (what to draw) separately from terminal output. Use Criterion to measure time to compute visible rows, not time to write to terminal.

2. **Should integration tests spawn actual TUI instances?**
   - What we know: ratatui-testlib supports PTY-based testing
   - What's unclear: Whether phase 14 requirements need visual testing or just logic testing
   - Recommendation: Start with logic-only tests (search filters rows, export generates CSV). Add PTY tests later if visual regressions occur.

3. **What baseline should benchmarks compare against?**
   - What we know: Criterion can track performance over time
   - What's unclear: Initial baseline for "acceptable" performance
   - Recommendation: Establish baseline in Phase 14 by measuring current performance, then track regressions. v1.4 targets (1 second load, smooth scroll) guide what's "acceptable".

4. **How to handle heap profiling overhead in benchmarks?**
   - What we know: dhat adds overhead, shouldn't be enabled during Criterion benchmarks
   - What's unclear: Best workflow for switching between benchmark and profile modes
   - Recommendation: Use feature flags (`dhat-heap`) for profiling, keep benchmarks clean. Profile when investigating MEM-01, benchmark for regression detection.

## Sources

### Primary (HIGH confidence)
- [Criterion.rs Getting Started](https://bheisler.github.io/criterion.rs/book/getting_started.html) - Benchmark setup and API
- [Criterion.rs crates.io](https://crates.io/crates/criterion) - Version 0.8.1
- [dhat crate documentation](https://docs.rs/dhat/latest/dhat/) - API and usage patterns
- [dhat crates.io](https://crates.io/crates/dhat) - Version 0.3.3
- [Rust Performance Book - Profiling](https://nnethercote.github.io/perf-book/profiling.html) - Profiling tools and setup
- [Rust Performance Book - Build Configuration](https://nnethercote.github.io/perf-book/build-configuration.html) - Cargo profile settings
- [Ratatui Panic Hooks Recipe](https://ratatui.rs/recipes/apps/panic-hooks/) - Terminal restoration pattern
- [Rust Book - Test Organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html) - Integration test structure
- [cargo-flamegraph GitHub](https://github.com/flamegraph-rs/flamegraph) - Installation and usage
- [samply GitHub](https://github.com/mstange/samply) - Installation and usage
- [samply crates.io](https://crates.io/crates/samply) - Latest version

### Secondary (MEDIUM confidence)
- [How to Profile Rust Applications with perf, flamegraph, and samply (2026-01-07)](https://oneuptime.com/blog/post/2026-01-07-rust-profiling-perf-flamegraph/view) - Verified current profiling practices
- [cargo-criterion documentation](https://bheisler.github.io/criterion.rs/book/cargo_criterion/cargo_criterion.html) - Enhanced reporting features
- [Avoiding benchmarking pitfalls with black_box](https://alic.dev/blog/blackbox) - Common benchmark mistakes
- [ratatui-testlib documentation](https://docs.rs/ratatui-testlib/latest/ratatui_testlib/) - Integration testing approach
- [How to Write Integration Tests for Rust APIs with Testcontainers (2026-01-07)](https://oneuptime.com/blog/post/2026-01-07-rust-testcontainers/view) - Modern integration testing patterns

### Tertiary (LOW confidence)
- None - all claims verified with official documentation or recent (2026) sources

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Criterion, flamegraph, and dhat are widely documented with stable APIs and recent usage confirmation
- Architecture: HIGH - Patterns verified from official documentation (Rust Book, Criterion docs, Ratatui recipes)
- Pitfalls: HIGH - Sourced from Rust Performance Book and verified blog posts about common issues
- Integration testing: MEDIUM - ratatui-testlib is newer (2024-2025), less widely adopted than core tools

**Research date:** 2026-02-10
**Valid until:** 2026-04-10 (60 days - stable tooling ecosystem, slow-moving changes)
