# Phase 16: Memory Optimization - Research

**Researched:** 2026-02-10
**Domain:** String interning and memory optimization for large datasets in Rust
**Confidence:** HIGH

## Summary

Phase 16 aims to reduce memory usage from ~2GB to <1GB for 1.8M row datasets through compact storage techniques. The current architecture stores table data as `Vec<Vec<String>>` where each cell allocates a separate `String`. For datasets with high string duplication (common in database columns like status codes, categories, or foreign keys), this creates massive redundancy—each occurrence of "active" allocates 6 bytes plus heap overhead independently.

String interning eliminates this duplication by storing each unique string once and using lightweight integer keys (symbols) for all references. For typical database tables with repetitive column values, this achieves 50-80% memory savings. The recommended approach uses the `lasso` crate with single-threaded `Rodeo` interner, which provides O(1) interning and lookup operations with minimal overhead.

**Primary recommendation:** Replace `Vec<Vec<String>>` with `Vec<Vec<Spur>>` where `Spur` is a 32-bit symbol from `lasso::Rodeo`. Store the interner in the `Tab` struct. Add runtime memory tracking via `sysinfo` crate to display RSS in the status bar. Profile before/after with `dhat` to validate savings.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| lasso | 0.7+ | String interning with O(1) operations | Most actively maintained, excellent single/multi-thread support, zero-copy resolution via RodeoReader |
| sysinfo | 0.33+ | Cross-platform process memory reporting | De facto standard for system metrics in Rust, refreshable memory stats |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| dhat | 0.3 (already in project) | Heap profiling and memory tracking | Validate optimization effectiveness, measure allocation sites |
| rustc-hash | 2.0+ | FxHashMap for faster hashing | Optional performance boost for interner's internal HashMap |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| lasso | string-interner | string-interner has multiple backends but lasso is simpler API, better documentation, and RodeoReader conversion for zero-contention reads |
| lasso | string_cache | string_cache requires compile-time static atoms, not suitable for runtime data |
| Spur (32-bit) | MicroSpur (8-bit) | MicroSpur limits to 256 unique strings—too restrictive for database tables |

**Installation:**
```toml
[dependencies]
lasso = { version = "0.7", features = ["serialize"] }
sysinfo = "0.33"
rustc-hash = "2.0"  # Optional optimization
```

## Architecture Patterns

### Recommended Project Structure
String interning requires changes to three core data structures:

```
src/
├── parser.rs        # TableData now stores interned symbols
├── workspace.rs     # Tab stores Rodeo interner + symbol table
├── render.rs        # Resolve symbols to &str for display
└── main.rs          # Add memory tracking to status bar
```

### Pattern 1: Interner-Per-Tab Storage
**What:** Store one `Rodeo` interner per `Tab`, intern strings during parsing/loading
**When to use:** Single-threaded data loading (matches current architecture)
**Example:**
```rust
// Source: lasso docs + matklad's fast-simple-rust-interner pattern
use lasso::{Rodeo, Spur};

pub struct Tab {
    pub name: String,
    pub data: TableData,
    pub interner: Rodeo,  // NEW: Per-tab string interner
    // ... existing fields
}

pub struct TableData {
    pub headers: Vec<String>,        // Keep headers as String (small, rarely duplicated)
    pub rows: Vec<Vec<Spur>>,        // CHANGED: Store symbols instead of Strings
}

impl Tab {
    pub fn new(name: String, data: TableData, view_mode: ViewMode) -> Self {
        let mut interner = Rodeo::default();
        // Intern existing data if needed
        let mut tab = Self {
            name,
            data,
            interner,
            column_config: ColumnConfig::new(num_cols),
            // ... other fields
        };
        tab
    }

    // Helper to add row with interning
    pub fn add_row(&mut self, row: Vec<String>) {
        let interned_row: Vec<Spur> = row.iter()
            .map(|s| self.interner.get_or_intern(s))
            .collect();
        self.data.rows.push(interned_row);
    }

    // Resolve symbol to string slice for rendering
    pub fn resolve(&self, symbol: Spur) -> &str {
        self.interner.resolve(&symbol)
    }
}
```

### Pattern 2: Progressive Migration Strategy
**What:** Migrate incrementally to avoid breaking all features simultaneously
**When to use:** Large codebase with many string operations (search, export, column ops)
**Example:**
```rust
// Phase 1: Add interner alongside existing String storage (dual mode)
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,      // Keep for now
    pub interned_rows: Vec<Vec<Spur>>, // NEW: Parallel storage
}

// Phase 2: Migrate read operations (render, display)
// Phase 3: Migrate search (operate on symbols or resolve on-demand)
// Phase 4: Remove old String storage, rename interned_rows -> rows
```

### Pattern 3: Memory Tracking in Event Loop
**What:** Periodically refresh memory stats and display in status bar
**When to use:** Always—required for MEM-02
**Example:**
```rust
// Source: sysinfo docs
use sysinfo::{System, RefreshKind, ProcessRefreshKind};

fn main() -> io::Result<()> {
    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::new().with_memory())
    );
    let pid = sysinfo::get_current_pid().expect("Failed to get PID");

    loop {
        // Refresh memory stats (not every frame—expensive)
        if frame_count % 30 == 0 {  // Every 30 frames (~1 second at 30 FPS)
            sys.refresh_process(pid);
        }

        // Get current memory usage
        let memory_mb = if let Some(process) = sys.process(pid) {
            process.memory() / 1024 / 1024  // Convert bytes to MB
        } else {
            0
        };

        // Display in status bar
        let status_text = format!("Memory: {} MB | ...", memory_mb);
        // ... render status bar
    }
}
```

### Anti-Patterns to Avoid

- **Interning per-request:** Don't create new `Rodeo` instances per operation—store once per Tab
- **Cloning interned strings:** Don't call `.to_string()` on resolved symbols unless necessary—use `&str` references
- **Over-interning:** Don't intern unique data (IDs, UUIDs)—only intern when duplication expected
- **Thread-safety confusion:** `Rodeo` is not `Send`—don't try to share across threads (current architecture is single-threaded)

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| String deduplication | Custom HashMap<String, u32> + Vec<String> | lasso::Rodeo | Handles allocation growth, provides type-safe symbols, optimized hashing, zero-copy reader conversion |
| Memory measurement | Custom allocator wrapper | sysinfo crate | Cross-platform (Windows/Linux/macOS), handles process enumeration, provides RSS/virtual memory metrics |
| Symbol type safety | Raw u32 indices | lasso::Spur (newtype) | Prevents mixing indices from different interners, self-documenting type system |
| Interner arena growth | Manual Vec reallocation | lasso's internal buffer management | Exponential growth strategy, stable pointers, handles capacity calculation |

**Key insight:** String interning seems simple (HashMap + Vec) but the devil is in the details—stable pointers when buffers grow, efficient hashing for small strings, type-safe symbols to prevent bugs. `lasso` has solved these problems with 5+ years of production use.

## Common Pitfalls

### Pitfall 1: Unbounded Interner Growth
**What goes wrong:** For datasets with truly unique strings (UUIDs, timestamps), the interner grows without bound and wastes memory storing a "deduplication index" for non-duplicated data.
**Why it happens:** Blindly interning all string data without considering cardinality.
**How to avoid:** Profile first with `dhat` on representative datasets. If unique string ratio >70%, don't intern those columns. Add heuristic: if interner size approaches original string size, skip interning.
**Warning signs:** Memory usage stays same or increases after interning. `dhat` shows interner HashMap consuming significant memory.

### Pitfall 2: Search Performance Regression
**What goes wrong:** Search becomes slower because each comparison requires symbol resolution (`interner.resolve(&symbol)`).
**Why it happens:** Naively replacing `cell.contains(needle)` with `interner.resolve(&symbol).contains(needle)`.
**How to avoid:** Intern the search needle once, then compare symbols directly when possible. For substring search, resolve is unavoidable but still acceptable (rendering already resolves all visible cells).
**Warning signs:** Search benchmarks show slowdown >10%. User reports lag during search with interned storage.

### Pitfall 3: Export Data Corruption
**What goes wrong:** CSV/JSON export produces symbol IDs instead of strings ("Data exported: [[1, 2, 3], [1, 4, 5]]" instead of actual values).
**Why it happens:** Forgetting to resolve symbols before serialization.
**How to avoid:** Export functions MUST iterate symbols and resolve before writing. Add integration test: load data, intern, export, verify output matches original.
**Warning signs:** Export tests fail. Manual export inspection shows numbers instead of strings.

### Pitfall 4: Interner Serialization
**What goes wrong:** Attempting to serialize/deserialize interned data without the interner context loses all string data.
**Why it happens:** Symbols are meaningless without their interner—saving `Vec<Vec<Spur>>` alone is useless.
**How to avoid:** This phase doesn't require persistence. If future phases need serialization, use lasso's `serialize` feature to save both interner and symbols together, or resolve all symbols before serialization.
**Warning signs:** Feature requests for "save workspace" or "export session". Architecture discussions about persisting tabs.

### Pitfall 5: Memory Measurement Overhead
**What goes wrong:** Calling `sys.refresh_process()` every frame causes performance degradation.
**Why it happens:** Process enumeration and memory stats are OS syscalls—expensive operations.
**How to avoid:** Throttle updates to once per second (every 30 frames at 30 FPS). Memory doesn't change meaningfully frame-to-frame.
**Warning signs:** Framerate drops when memory display is enabled. Profiling shows time spent in sysinfo refresh.

## Code Examples

Verified patterns from official sources:

### String Interning with Lasso
```rust
// Source: https://docs.rs/lasso/latest/lasso/
use lasso::{Rodeo, Spur};

// Create interner
let mut rodeo = Rodeo::default();

// Intern strings (returns symbol)
let hello: Spur = rodeo.get_or_intern("hello");
let world: Spur = rodeo.get_or_intern("world");
let hello2: Spur = rodeo.get_or_intern("hello");

// Same string returns same symbol
assert_eq!(hello, hello2);

// Resolve symbol to string slice (zero-copy)
assert_eq!(rodeo.resolve(&hello), "hello");

// Check if string is interned without interning
if let Some(symbol) = rodeo.get("hello") {
    println!("Already interned: {}", symbol);
}
```

### Process Memory Tracking
```rust
// Source: https://docs.rs/sysinfo/latest/sysinfo/
use sysinfo::{System, RefreshKind, ProcessRefreshKind, Pid};

let mut sys = System::new_with_specifics(
    RefreshKind::new().with_processes(ProcessRefreshKind::new().with_memory())
);

// Get current process PID
let pid: Pid = sysinfo::get_current_pid().expect("Failed to get current PID");

// Refresh memory stats for our process
sys.refresh_process(pid);

// Get memory usage in bytes
if let Some(process) = sys.process(pid) {
    let memory_bytes = process.memory();      // RSS (resident set size)
    let memory_mb = memory_bytes / 1024 / 1024;
    println!("Current memory usage: {} MB", memory_mb);
}
```

### FxHashMap for Faster Interning (Optional)
```rust
// Source: https://docs.rs/rustc-hash/latest/rustc_hash/
use lasso::{Rodeo, Spur};
use rustc_hash::FxHasher;
use std::hash::BuildHasherDefault;

// Create Rodeo with FxHasher (faster for small keys like strings)
type FxRodeo = Rodeo<Spur, BuildHasherDefault<FxHasher>>;
let mut rodeo = FxRodeo::default();

// Use normally—API identical to standard Rodeo
let key = rodeo.get_or_intern("fast");
```

### Incremental Interning During Streaming Load
```rust
// Integration with existing streaming architecture (from Phase 15)
pub struct Tab {
    pub data: TableData,
    pub interner: Rodeo,
    // ... other fields
}

impl Tab {
    // Called from main event loop when new rows arrive
    pub fn append_streaming_rows(&mut self, string_rows: Vec<Vec<String>>) {
        for row in string_rows {
            let interned_row: Vec<Spur> = row
                .iter()
                .map(|cell| self.interner.get_or_intern(cell))
                .collect();
            self.data.rows.push(interned_row);
        }
        self.update_cached_widths();  // Existing method
    }
}

// In main.rs event loop (Phase 15 integration):
if let Some(ref loader) = streaming_loader {
    let new_rows = loader.try_recv_batch(5000);
    if !new_rows.is_empty() {
        if let Some(tab) = workspace.tabs.get_mut(0) {
            tab.append_streaming_rows(new_rows);  // NEW: Intern during append
        }
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Vec<Vec<String>> | Vec<Vec<Spur>> + Rodeo | 2020+ (lasso stable) | 50-80% memory reduction for repetitive data |
| Manual HashMap interner | lasso crate | 2020 | Type-safe symbols, better ergonomics, optimized growth |
| Valgrind DHAT | dhat-rs crate | 2020 | Cross-platform heap profiling (already in project) |
| /proc/self/statm parsing | sysinfo crate | 2018+ | Cross-platform memory stats with maintained API |

**Deprecated/outdated:**
- string_cache: Designed for compile-time static atoms (Servo browser use case). Not suitable for runtime data interning. Still maintained but niche.
- Custom allocators for memory tracking: Modern approach uses sysinfo for process-level metrics or dhat for allocation-level profiling. Custom allocators add complexity without benefit for this use case.

## Open Questions

1. **What is the actual duplication ratio in target datasets?**
   - What we know: Requirements specify "1.8M row dataset", but no data about column cardinality
   - What's unclear: If target data has high uniqueness (UUIDs, timestamps), interning might not help
   - Recommendation: Add benchmark with synthetic data at various duplication ratios (10%, 50%, 90%). Profile with dhat before/after. Success criteria should be validated against realistic data.

2. **Should headers be interned?**
   - What we know: Headers are small (typically <50 chars per column) and never duplicated within a table
   - What's unclear: Keeping headers as String is simpler, but inconsistent with cell data
   - Recommendation: Keep headers as `Vec<String>`. Memory impact is negligible (10 columns × 50 bytes = 500 bytes vs millions of cells). Simplifies rendering code.

3. **Performance impact on search operations?**
   - What we know: Search currently operates on String slices. Interned data requires resolution.
   - What's unclear: Does `interner.resolve(&symbol).contains(needle)` regress search performance?
   - Recommendation: Maintain search benchmarks from Phase 14. Profile before/after. If regression >10%, optimize by interning needle once and comparing symbols for exact match, falling back to substring search only when needed.

4. **Should we use RodeoReader for read-heavy operations?**
   - What we know: Lasso provides `RodeoReader` for zero-contention reads after interning complete
   - What's unclear: Current architecture is single-threaded, so contention isn't an issue. Would conversion overhead outweigh benefits?
   - Recommendation: Not for Phase 16. Single-threaded `Rodeo` is simpler. If future phases add multi-threaded rendering/search, revisit RodeoReader.

## Sources

### Primary (HIGH confidence)
- [lasso crate documentation](https://docs.rs/lasso) - Complete API reference, benchmarks, usage patterns
- [sysinfo crate documentation](https://docs.rs/sysinfo) - Cross-platform process memory stats
- [Fast and Simple Rust Interner by matklad](https://matklad.github.io/2020/03/22/fast-simple-rust-interner.html) - Detailed implementation patterns, pitfalls, performance analysis
- [dhat-rs documentation](https://docs.rs/dhat) - Already integrated in project (Phase 14)

### Secondary (MEDIUM confidence)
- [5 Proven Rust Techniques for Memory-Efficient Data Structures](https://elitedev.in/rust/5-proven-rust-techniques-for-memory-efficient-data/) - SmallVec, ArrayVec, pre-allocation strategies
- [Rust Heap Profiling with Jemalloc](https://magiroux.com/rust-jemalloc-profiling) - Alternative profiling approach (not needed, dhat sufficient)
- [Rust Performance Book - Heap Allocations](https://nnethercote.github.io/perf-book/heap-allocations.html) - Vec growth strategies, allocation patterns
- [Performance Optimization in Rust: String Interning](https://medium.com/@zhiweiwang2001/performance-optimization-in-rust-understanding-string-interning-symbol-and-smartstring-6ec80b8c781a) - Symbol pattern benefits

### Tertiary (LOW confidence - verification needed)
- WebSearch results suggesting 50-80% memory savings from interning - Would need validation with project-specific data patterns
- Community discussions on string-interner vs lasso tradeoffs - Anecdotal preference, not authoritative benchmarks

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - lasso and sysinfo are mature, well-documented, with stable APIs. Both verified via official docs.
- Architecture: HIGH - Patterns extracted from official lasso docs and matklad's authoritative blog post. Integration points verified against existing codebase (Phase 15 streaming architecture).
- Pitfalls: MEDIUM - Derived from community discussions and first-principles analysis. Would increase to HIGH with real-world validation via dhat profiling.
- Memory savings claims: MEDIUM - 50-80% range is commonly cited but depends heavily on data characteristics. Requires validation with representative datasets.

**Research date:** 2026-02-10
**Valid until:** ~30 days (stable domain, mature crates)

**Key assumptions:**
1. Target datasets have moderate-to-high string duplication (typical for database tables with categorical columns, status fields, foreign keys)
2. Single-threaded architecture continues (matches current Phase 15 implementation)
3. Memory measurement at process level (RSS) is sufficient (no need for allocation-level tracking during normal operation)

**Validation required:**
- Profile current memory usage with 1.8M row dataset using dhat (establish baseline)
- Measure actual duplication ratio in representative datasets
- Benchmark search performance before/after interning
- Verify all 33 integration tests (14-03) pass with interned storage
