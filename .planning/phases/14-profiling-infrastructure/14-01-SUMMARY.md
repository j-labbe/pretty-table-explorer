---
phase: 14-profiling-infrastructure
plan: 01
subsystem: build-configuration
tags: [library-crate, profiling, benchmarking, flamegraph, dhat]
completed: 2026-02-10

dependencies:
  requires: []
  provides:
    - library-crate-foundation
    - dhat-heap-profiling
    - flamegraph-support
    - criterion-dev-dependency
  affects:
    - external-benchmarks
    - integration-tests
    - profiling-workflow

tech-stack:
  added:
    - criterion: "0.5 with html_reports for benchmark harness"
    - dhat: "0.3 optional dependency for heap profiling"
  patterns:
    - "Library crate re-exports all modules for external access"
    - "Profile configurations enable debug symbols for flamegraph generation"
    - "Feature flags for optional profiling capabilities"
    - "Extracted panic hook function for terminal restoration"

key-files:
  created:
    - src/lib.rs: "Library crate re-exporting all public modules"
  modified:
    - Cargo.toml: "Added [lib], dev-dependencies, features, and profile configurations"
    - src/main.rs: "Converted to use library crate imports, added dhat support, extracted panic hook"
    - src/render.rs: "Changed pub(crate) functions to pub for library access"

decisions:
  - what: "Use Criterion 0.5 instead of 0.8"
    why: "Version 0.5 is latest stable on crates.io; 0.8 doesn't exist yet"
    impact: "Benchmark harness will use stable API"
  - what: "Configure [profile.release] with debug = line-tables-only"
    why: "Enables cargo-flamegraph to resolve function names in flamegraph SVG output"
    impact: "Flamegraphs will show readable function names instead of memory addresses"
  - what: "Configure [profile.bench] with debug = true"
    why: "Provides full debug info for detailed profiling during benchmarks"
    impact: "Better profiling visibility during benchmark runs"
  - what: "Remove unused column import from main.rs"
    why: "Module imported but not referenced directly, causing compiler warning"
    impact: "Clean compilation without warnings"
  - what: "Defer benchmark declarations to future plan"
    why: "Cargo requires bench files to exist before declaring them in Cargo.toml"
    impact: "Benchmark declarations will be added in plan 14-02 when benchmarks are created"

metrics:
  duration: 173
  tasks_completed: 2
  files_created: 1
  files_modified: 3
  commits: 2
  deviations: 1
---

# Phase 14 Plan 01: Library Crate and Profiling Foundation

**One-liner:** Create library crate exposing all modules, add Criterion/dhat dependencies, extract panic hook, and configure Cargo profiles with debug symbols for flamegraph generation.

## Overview

This plan establishes the foundational infrastructure for Phase 14's profiling capabilities. By creating a library crate that re-exports all modules, we enable external benchmarks and integration tests in subsequent plans. The addition of dhat heap profiling and Cargo profile configurations (with debug symbols for flamegraph support) provides the measurement tooling needed for performance optimization work.

## Execution Summary

**Status:** ✅ Complete
**Duration:** 2.8 minutes
**Commits:** 2 task commits

### Tasks Completed

| Task | Name                                              | Commit  | Files                                   |
| ---- | ------------------------------------------------- | ------- | --------------------------------------- |
| 1    | Create library crate and configure Cargo.toml     | 3f836eb | src/lib.rs, Cargo.toml, src/main.rs, src/render.rs |
| 2    | Add dhat heap profiling support and extract panic hook | a829d44 | src/main.rs                             |

### Deviations from Plan

**1. [Rule 3 - Blocking] Removed unused column import**
- **Found during:** Task 1 verification
- **Issue:** Module `column` imported in main.rs but not directly referenced, causing unused import warning
- **Fix:** Removed `column` from library crate import statement
- **Files modified:** src/main.rs
- **Commit:** 3f836eb (included in Task 1 commit)

**2. [Planned adjustment] Deferred benchmark declarations**
- **Reason:** Cargo requires benchmark files to exist before declaring `[[bench]]` sections
- **Decision:** Remove benchmark declarations from Cargo.toml in this plan; will add them in plan 14-02 when benchmark files are created
- **Files modified:** Cargo.toml
- **Commit:** 3f836eb (included in Task 1 commit)
- **Impact:** No functional impact; benchmarks will be declared when implementations exist

## Technical Implementation

### Library Crate Structure

Created `src/lib.rs` re-exporting all modules:
- parser, column, export, state, workspace, render, handlers, db, update

Changed `pub(crate)` to `pub` in render.rs:
- `calculate_auto_widths()`
- `calculate_widths()`

These functions were crate-private but needed public visibility for external benchmarks to access them through the library interface.

### Cargo.toml Configuration

**Library target:**
```toml
[lib]
name = "pretty_table_explorer"
path = "src/lib.rs"
```

**Dependencies added:**
- `criterion = { version = "0.5", features = ["html_reports"] }` (dev-dependency)
- `dhat = { version = "0.3", optional = true }` (optional dependency)

**Feature flag:**
```toml
[features]
dhat-heap = ["dhat"]
```

**Profile configurations for flamegraph support:**
```toml
[profile.release]
debug = "line-tables-only"

[profile.bench]
debug = true
```

The `debug = "line-tables-only"` setting is critical for flamegraph generation. Without debug symbols, cargo-flamegraph produces flamegraphs showing only memory addresses. With this setting, flamegraphs display resolved function names, making performance bottlenecks immediately identifiable.

### dhat Heap Profiling

Added conditional compilation for dhat:

**Global allocator:**
```rust
#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;
```

**Profiler initialization:**
```rust
fn main() -> io::Result<()> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();
    // ...
}
```

**Usage:** `cargo run --features dhat-heap` generates `dhat-heap.json` on program exit.

### Panic Hook Extraction

Extracted inline panic hook to named function:

```rust
fn init_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));
}
```

This improves code organization and makes the terminal restoration logic more maintainable. The function is called before `init_terminal()` to ensure terminal state is restored on any panic.

## Verification Results

All verification checks passed:

- ✅ `cargo check` compiles without errors (library + binary)
- ✅ `cargo check --features dhat-heap` compiles without errors
- ✅ `cargo test` passes (31 tests in modules)
- ✅ src/lib.rs exists with module re-exports
- ✅ Cargo.toml has [lib], dev-dependencies, features sections
- ✅ Cargo.toml [profile.release] contains `debug = "line-tables-only"` for flamegraph support
- ✅ main.rs uses library crate imports
- ✅ Panic hook extracted to named function

## Artifacts

**Key files created:**
- `src/lib.rs` - Library crate exposing all modules

**Key files modified:**
- `Cargo.toml` - Added library target, dependencies, features, profile configs
- `src/main.rs` - Converted to library imports, added dhat support, extracted panic hook
- `src/render.rs` - Made calculate_*_widths functions public

**Commits:**
- `3f836eb` - Create library crate with profiling dependencies
- `a829d44` - Add dhat heap profiling and extract panic hook

## Downstream Impact

This plan enables:
- **Plan 14-02:** Criterion benchmarks can now access modules via library crate
- **Plan 14-03:** Integration tests can import and test module interactions
- **Phase 16:** Flamegraph profiling during memory optimization work
- **Phase 17:** Performance measurement during virtualized rendering implementation

The library crate architecture is now the standard pattern for all future external tests and benchmarks.

## Self-Check: PASSED

Verified all claims:

```bash
# Files exist
✓ src/lib.rs exists
✓ Cargo.toml modified

# Commits exist
✓ 3f836eb: feat(14-01): create library crate with profiling dependencies
✓ a829d44: feat(14-01): add dhat heap profiling and extract panic hook

# Functionality
✓ cargo check passes
✓ cargo check --features dhat-heap passes
✓ cargo test passes (31 tests)
✓ Profile configurations present in Cargo.toml
```
