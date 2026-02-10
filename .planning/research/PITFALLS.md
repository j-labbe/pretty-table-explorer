# Domain Pitfalls: Performance Optimization of Rust TUI Applications

**Domain:** Performance optimization for large datasets in existing Rust TUI applications
**Researched:** 2026-02-09
**Context:** Adding streaming, threading, and memory optimization to synchronous ratatui-based table viewer
**Confidence:** MEDIUM to HIGH

## Executive Summary

Optimizing an existing synchronous Rust TUI application for large datasets (1.8M+ rows) introduces specific risks around thread safety, data structure migration, and regression of working features. The primary danger is **breaking correctness while chasing performance**. Ratatui is thread-safe by design, but terminal backends are not Send/Sync, requiring careful architecture. The most common failures occur when: (1) mixing blocking operations into async code, (2) holding locks across .await points causing deadlocks, (3) breaking existing features during data structure refactoring, (4) off-by-one errors in virtualized scrolling, and (5) failing to restore terminal state on panic.

**Critical insight:** Measure before optimizing. Most performance gains come from algorithmic improvements (O(n) → O(log n)) not micro-optimizations. Profile first, optimize hot paths only, and maintain comprehensive regression tests.

---

## Critical Pitfalls

### Pitfall 1: Terminal Backend Not Send/Sync — Rendering from Background Threads

**What goes wrong:**

Attempting to call `terminal.draw()` from a background thread causes compilation failure or runtime panics. While ratatui itself is thread-safe, the underlying terminal backends (Crossterm, Termion) are **not Send/Sync**. The Terminal type cannot be safely sent between threads.

**Why it happens:**

Developers assume that if data loading happens in a background thread, rendering should too. The pattern of "load in background, render when ready" leads to attempting to move the Terminal into a thread or across an await boundary.

**Consequences:**

- Compilation error: "Terminal cannot be sent between threads safely"
- If worked around with unsafe, terminal state corruption or panics
- Intermittent rendering bugs that are difficult to reproduce
- Terminal left in unusable state (alternate screen not restored, raw mode stuck)

**Prevention:**

1. **Keep rendering on the main thread only**. Use channels to communicate state updates from background threads to the main rendering loop.
2. **Architecture pattern**: Background thread → tokio::sync::mpsc → Main thread polls channel → Main thread renders
3. Use ratatui's async template pattern: separate event handling/data loading (can be async/threaded) from rendering (stays synchronous on main thread)
4. For heavy operations (image processing, CSV parsing), offload to background threads but send results via channels

**Detection:**

- Compiler errors about Send/Sync traits
- Terminal rendering appearing "frozen" or not updating
- Panic messages about terminal state in alternate screen (hard to see)
- Terminal not restoring properly on Ctrl+C

**Phase-specific warning:**

- **Phase 1 (Architecture)**: Design threading boundaries carefully. Decide what runs on main thread vs background
- **Phase 2 (Streaming)**: Data loading can be threaded, but state updates must marshal to main thread
- **Phase 3 (Virtualization)**: Viewport calculation stays synchronous on main thread

**Confidence:** HIGH (verified with official ratatui documentation and community patterns)

**Sources:**
- [Ratatui FAQ - Thread Safety](https://ratatui.rs/faq/)
- [Ratatui Async Template](https://ratatui.github.io/async-template/)
- [Ratatui Rendering Concepts](https://ratatui.rs/concepts/rendering/)

---

### Pitfall 2: Async/Sync Impedance Mismatch — Blocking Operations in Async Code

**What goes wrong:**

Calling synchronous blocking operations (std::fs::read, std::sync::Mutex::lock(), heavy CPU work) inside async functions kills throughput. The async executor expects quick, non-blocking operations. Blocking operations monopolize worker threads, preventing other tasks from making progress.

**Why it happens:**

When refactoring synchronous code to async, developers convert function signatures to `async fn` but forget to replace blocking calls with async equivalents. The code compiles and runs but performs worse than the original synchronous version.

**Consequences:**

- **Throughput degradation**: Up to 40% performance loss in production systems
- Async runtime starvation: tasks pile up waiting for blocked worker threads
- Deadlocks when blocking on synchronous mutexes while holding async locks
- Application appears "frozen" despite async infrastructure

**Prevention:**

1. **Profile before and after async conversion** — async doesn't guarantee faster
2. **Use async-compatible primitives**:
   - `tokio::fs::read()` not `std::fs::read()`
   - `tokio::sync::Mutex` not `std::sync::Mutex` in async contexts
   - `tokio::task::spawn_blocking()` for CPU-heavy work
3. **Audit for blocking calls**: grep codebase for `std::fs::`, `std::sync::Mutex`, tight loops
4. **Consider not using async**: If only a small portion benefits from async, threaded approach may be simpler

**Detection:**

- Performance regression after async refactoring
- `tokio-console` showing tasks stuck in "running" state
- Worker thread utilization at 100% but low throughput
- Increased latency on simple operations

**Phase-specific warning:**

- **Phase 1 (Architecture)**: Decide async vs threaded based on actual needs, not assumptions
- **Phase 2 (Streaming)**: CSV parsing is I/O-bound but may not benefit from async if single-file
- **Phase 3 (Background loading)**: If using tokio, offload blocking CSV parsing to spawn_blocking()

**Confidence:** HIGH (verified with recent 2026 blog post on Rust async performance)

**Sources:**
- [Rust Async Just Killed Your Throughput](https://medium.com/@shkmonty35/rust-async-just-killed-your-throughput-and-you-didnt-notice-c38dd119aae5)
- [Rust Concurrency: Common Async Pitfalls](https://leapcell.medium.com/rust-concurrency-common-async-pitfalls-explained-8f80d90b9a43)
- [Why Async? - Asynchronous Programming in Rust](https://rust-lang.github.io/async-book/01_getting_started/02_why_async.html)

---

### Pitfall 3: Lock Contention with Arc<Mutex<T>> — Reader-Heavy Workloads

**What goes wrong:**

Using `Arc<Mutex<T>>` for shared table data causes lock contention when multiple operations (search, render, export) need read access simultaneously. Every read operation blocks all other readers, even though the data isn't being modified.

**Why it happens:**

Arc<Mutex<T>> is the "default" pattern taught for shared state in Rust. Developers apply it without considering read/write access patterns. Mutex doesn't distinguish between readers and writers — all access is exclusive.

**Consequences:**

- Search feature blocks rendering until complete
- Export blocks user interaction
- Multiple tabs/split views serialize access, defeating parallelism
- Performance degrades as thread count increases (opposite of expected)

**Prevention:**

1. **Use Arc<RwLock<T>> for read-heavy workloads**: Allows multiple concurrent readers, exclusive writer
2. **In async contexts, use tokio::sync::RwLock** to avoid blocking executor threads
3. **Minimize critical sections**: Hold locks for minimal time, clone/copy data if needed
4. **Consider message-passing over shared state**: Background thread owns data, main thread requests via channels
5. **Profile with cargo flamegraph**: Visualize time spent waiting for locks

**Detection:**

- `perf` showing high time in mutex wait
- Thread contention visible in profiling tools
- Performance degrading with more concurrent operations
- Export/search making UI unresponsive

**Phase-specific warning:**

- **Phase 1 (Architecture)**: Design data ownership — single-owner with channels vs shared with RwLock
- **Phase 2 (Search/Filter)**: Search needs read access while render continues — RwLock candidate
- **Phase 3 (Multi-tab)**: Multiple views of same data — strong RwLock use case

**Confidence:** HIGH (verified with Tokio and Rust std documentation)

**Sources:**
- [Shared State Concurrency - The Rust Programming Language](https://doc.rust-lang.org/book/ch16-03-shared-state.html)
- [Rust-101 - Mutex, Interior Mutability, RwLock, Sync](https://www.ralfj.de/projects/rust-101/part15.html)
- [Differences between tokio::sync::mpsc and crossbeam](https://users.rust-lang.org/t/differences-between-channel-in-tokio-mpsc-and-crossbeam/92676)

---

### Pitfall 4: Tokio select! Deadlock — Unbalanced Polling and Mutex Ordering

**What goes wrong:**

Using `tokio::select!` with high-frequency streams and mutexes creates two failure modes: (1) high-volume streams starve low-frequency branches (shutdown never checked), and (2) acquiring mutexes in different orders across select! branches causes deadlock.

**Why it happens:**

select! polls all branches in order. If the first branch (e.g., data stream) is constantly ready, later branches (shutdown signal) never get polled. Additionally, select! runs all branch expressions on the same thread, so blocking one blocks all.

**Consequences:**

- Shutdown/cleanup never executes (app can't be stopped gracefully)
- Deadlock when one branch holds Lock A and waits for Lock B, while another holds B and waits for A
- Worker threads blocked indefinitely, application appears frozen
- Ctrl+C doesn't respond, requires kill -9

**Prevention:**

1. **Order matters**: Place low-frequency critical branches (shutdown) FIRST in select! macro
2. **Use biased; for priority**: `select! { biased; shutdown_rx => ..., data_rx => ... }`
3. **Never hold Mutex across .await**: Lock, read/clone data, drop guard, then await
4. **For parallelism, spawn tasks**: Don't use select! to multiplex CPU work, use tokio::spawn
5. **Consistent lock ordering**: Always acquire locks in same order (alphabetically by variable name)

**Detection:**

- App won't shut down on SIGTERM or Ctrl+C
- Deadlock detector in tokio-console showing circular wait
- One branch of select! never executing (add metrics/logging)
- Worker threads stuck in "running" state indefinitely

**Phase-specific warning:**

- **Phase 2 (Streaming)**: select! on CSV chunks vs shutdown — shutdown must be first
- **Phase 3 (Background updates)**: Multiple select! with shared state — audit lock ordering
- **Phase 4 (Multi-source)**: Complex select! with many branches — consider spawn per source

**Confidence:** HIGH (verified with Tokio official documentation and GitHub issues)

**Sources:**
- [Select | Tokio Tutorial](https://tokio.rs/tokio/tutorial/select)
- [select in tokio - Rust](https://docs.rs/tokio/latest/tokio/macro.select.html)
- [How to deadlock Tokio application in Rust with just a single mutex](https://turso.tech/blog/how-to-deadlock-tokio-application-in-rust-with-just-a-single-mutex)

---

### Pitfall 5: Data Structure Refactoring Breaking Existing Features

**What goes wrong:**

Changing from `Vec<Vec<String>>` to a custom structure (for streaming, memory optimization) breaks existing features that depend on indexing, iteration, or Vec-specific APIs. Search, filter, export, column operations fail to compile or exhibit subtle bugs.

**Why it happens:**

Existing code makes assumptions about the data structure (indexing with `[]`, `.len()`, `.iter()`, direct slicing). When refactoring to a different structure (e.g., chunked storage, Arc<[Row]>, custom iterator), these assumptions break.

**Consequences:**

- Compilation errors across many files (good — caught early)
- Subtle behavior changes (worse — tests pass but behavior differs)
- Search returns wrong rows (off-by-one in custom indexing)
- Export truncates data (iterator doesn't return all rows)
- Column operations corrupt data (assumptions about contiguous memory)

**Prevention:**

1. **Newtype pattern with trait bounds**: Wrap new structure, implement Index, IntoIterator, Deref where needed
2. **Comprehensive integration tests**: Test all features (search, export, column ops) BEFORE refactoring
3. **Incremental migration**:
   - Add new structure alongside old
   - Add feature flag: `#[cfg(feature = "new-storage")]`
   - Run full test suite with both structures
   - Only remove old structure when all tests pass with new
4. **Property-based testing with proptest**: Generate random operations, verify invariants hold
5. **Git safety**: Commit working state before refactoring, use feature branches

**Detection:**

- Compiler errors (best case)
- Test failures on search, filter, export
- Differing row counts between old and new structure
- Panics on indexing operations
- Export files differ from baseline

**Phase-specific warning:**

- **Phase 2 (Data structure change)**: HIGH RISK — this is the phase where Vec<Vec<String>> changes
- **Phase 3 (Virtualization)**: Custom indexing logic — proptest for boundary cases
- **Phase 4 (Memory optimization)**: Chunked storage — verify iterators return all data

**Confidence:** MEDIUM (general software engineering wisdom, verified with Rust refactoring patterns)

**Sources:**
- [Newtype Pattern - Rust Design Patterns](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html)
- [Property Testing Stateful Code in Rust](https://rtpg.co/2024/02/02/property-testing-with-imperative-rust/)
- [Effective Rust - Item 6: Embrace the newtype pattern](https://www.lurklurk.org/effective-rust/newtype.html)

---

### Pitfall 6: Virtualized Scrolling Off-By-One Errors

**What goes wrong:**

Calculating which rows to render in a virtualized view (only render visible rows) is error-prone. Common bugs: rendering row N+1 twice, skipping row N, rendering past end of data, incorrect scrollbar position.

**Why it happens:**

Viewport calculations involve multiple coordinate systems (terminal rows, data rows, scroll offset) with inclusive/exclusive ranges. Classic off-by-one: should it be `start..end` or `start..=end`? Is scroll_offset zero-indexed?

**Consequences:**

- Visual glitches: duplicate rows, blank rows, flickering
- Panic on out-of-bounds access when scrolling to bottom
- Scrollbar shows wrong position
- Search highlighting appears on wrong row
- Last row never visible (off-by-one at end)

**Prevention:**

1. **Unit tests for boundary conditions**:
   - First row (offset = 0)
   - Last row (offset = total_rows - viewport_height)
   - Empty data (total_rows = 0)
   - Data smaller than viewport (total_rows < viewport_height)
2. **Explicit variable naming**: `visible_start_row_inclusive`, `visible_end_row_exclusive`
3. **Use Range types**: `let visible_range: Range<usize> = start..end;` — documents inclusive/exclusive
4. **Draw invariants in comments**: "// visible_rows = data[scroll_offset .. scroll_offset + viewport_height]"
5. **Property-based testing**: proptest generates random scroll offsets, verify no panic and correct count

**Detection:**

- Panic: "index out of bounds"
- Visual regression: screenshot tests show duplicate/missing rows
- Fuzz testing with random scroll positions
- Scrollbar position doesn't match actual position

**Phase-specific warning:**

- **Phase 3 (Virtualization)**: CRITICAL — this is when virtualization is implemented
- **Phase 4 (Search overlay)**: Search must account for viewport offset (row in data vs row on screen)

**Confidence:** MEDIUM (general UI programming wisdom, Rust-specific boundary checking)

**Sources:**
- [RFC 1679: Panic Safe Slicing](https://rust-lang.github.io/rfcs/1679-panic-safe-slicing.md)
- [How to avoid bounds checks in Rust (without unsafe!)](https://shnatsel.medium.com/how-to-avoid-bounds-checks-in-rust-without-unsafe-f65e618b4c1e)
- [Virtual Scrolling in React (general patterns)](https://medium.com/@swatikpl44/virtual-scrolling-in-react-6028f700da6b)

---

### Pitfall 7: CSV Streaming Not Actually Streaming — Memory Explosion

**What goes wrong:**

Using an "async" or "streaming" CSV library that advertises low memory usage, but implementation buffers entire file in memory before returning iterator. Memory usage grows to file size (e.g., 1.8M rows = 500MB+).

**Why it happens:**

Libraries like csv_async can exhibit this behavior when used incorrectly. reqwest's response.bytes() loads entire response into memory. The streaming abstraction leaks — buffering happens at HTTP or I/O layer.

**Consequences:**

- OOM (Out of Memory) kill on large files
- 500MB+ memory spike that doesn't reclaim
- Defeats purpose of streaming implementation
- Works in dev (small files) but fails in production (large files)

**Prevention:**

1. **Verify with real dataset**: Test with 1.8M row file, monitor memory with `time -v` or `heaptrack`
2. **Use csv crate with Reader::from_reader**: Wrap BufReader, read line-by-line
3. **Avoid serde for rows**: `Reader::read_byte_record()` is zero-copy, String-per-field allocates
4. **Stream from file, not memory**: Use File handle directly, not read_to_string then parse
5. **Profile memory allocation**: cargo flamegraph with `--features "flamegraph"` or heaptrack

**Detection:**

- Memory usage grows linearly with file size (should be constant after buffer fills)
- Allocator reports large single allocation
- Slower load time for large files
- OOM killer activates

**Phase-specific warning:**

- **Phase 2 (Streaming implementation)**: CRITICAL — verify streaming is actually streaming
- **Phase 3 (Large file testing)**: Test with 1.8M+ row file before declaring complete

**Confidence:** MEDIUM to HIGH (verified with real bug reports from production systems)

**Sources:**
- [Streaming CSV allocates too much memory - Rust Forum](https://users.rust-lang.org/t/streaming-csv-allocates-too-much-memory/132765)
- [Rust and CSV parsing - Andrew Gallant (BurntSushi)](https://burntsushi.net/csv/)
- [Boost your data processing with Rust! CSV Parser Guide](https://codezup.com/rust-csv-parser-high-performance-guide/)

---

### Pitfall 8: Terminal State Not Restored on Panic — Unusable Terminal

**What goes wrong:**

Application panics while in alternate screen and raw mode. Panic message is printed to alternate screen (invisible). Terminal exits, but alternate screen and raw mode are still active. User sees blank screen, keypresses don't work, terminal appears "frozen".

**Why it happens:**

Ratatui enters alternate screen and raw mode on init. If panic occurs before restore(), these settings persist. Older ratatui versions didn't auto-install panic hooks, requiring manual setup.

**Consequences:**

- User terminal unusable after crash
- User doesn't see panic message (printed to alternate screen)
- User types `reset` blindly to recover terminal
- Terrible user experience, looks like a hang not a crash

**Prevention:**

1. **Use ratatui::init()** (ratatui >= 0.28.1): Automatically installs panic hooks
2. **For older ratatui or manual Terminal::new()**:
   ```rust
   let panic_hook = std::panic::take_hook();
   std::panic::set_hook(Box::new(move |info| {
       ratatui::restore();
       panic_hook(info);
   }));
   ```
3. **Ensure restore() called on all exit paths**: Use Drop implementation or defer-like pattern
4. **Test panic paths**: `#[should_panic]` tests should still restore terminal

**Detection:**

- Manual testing: Trigger panic (e.g., unwrap on None), check terminal restores
- CI test: Run app, send signal to cause panic, verify terminal state after
- User reports of "frozen terminal after crash"

**Phase-specific warning:**

- **Phase 1 (Setup)**: Implement panic hooks early before adding complex logic
- **All phases**: Maintain panic hook when refactoring terminal initialization

**Confidence:** HIGH (verified with ratatui official documentation)

**Sources:**
- [Setup Panic Hooks | Ratatui](https://ratatui.rs/recipes/apps/panic-hooks/)
- [Counter App Error Handling | Ratatui](https://ratatui.rs/tutorials/counter-app/error-handling/)
- [ratatui::restore() documentation](https://docs.rs/ratatui/latest/ratatui/fn.restore.html)

---

## Moderate Pitfalls

### Pitfall 9: Premature Optimization — Complexity Without Measurement

**What goes wrong:**

Implementing complex optimizations (custom allocators, unsafe code, SIMD) without profiling first. Spending days optimizing cold paths that contribute <1% of runtime. Making code harder to maintain for negligible gains.

**Why it happens:**

Performance optimization is engaging/fun. Developers optimize what's interesting rather than what's slow. Assumption that "this looks slow" equals "this is slow."

**Prevention:**

1. **Profile first**: cargo flamegraph, perf, criterion benchmarks
2. **Measure impact**: Benchmark before and after optimization
3. **Focus on hot paths**: 80% of time in 20% of code
4. **Algorithmic wins beat micro-optimizations**: O(n²) → O(n log n) beats SIMD on O(n²)
5. **Avoid premature abstraction**: Start simple, optimize proven bottlenecks

**Confidence:** HIGH

**Sources:**
- [Stop Writing Slow Rust: 20 Rust Tricks](https://leapcell.medium.com/stop-writing-slow-rust-20-rust-tricks-that-changed-everything-0a69317cac3e)
- [Unofficial Guide to Rust Optimization Techniques](https://extremelysunnyyk.medium.com/unofficial-guide-to-rust-optimization-techniques-ec3bd54c5bc0)

---

### Pitfall 10: Standard Table Widget for Large Datasets — O(n) Rendering

**What goes wrong:**

Using ratatui's built-in Table widget for 1.8M rows. Widget renders ALL rows to buffer before culling to viewport. Rendering time proportional to dataset size, not screen size.

**Why it happens:**

Table widget API expects `Vec<Row>` — natural to pass all rows. Documentation doesn't emphasize virtualization requirement for large datasets.

**Prevention:**

1. **Use specialized virtual table widget**: rat-ftable (now rat-salsa) implements TableData trait for lazy rendering
2. **Only pass visible rows to Table**: Calculate viewport, slice data, pass only visible subset
3. **Measure render time**: Should be constant regardless of total rows

**Confidence:** HIGH

**Sources:**
- [rat-ftable: Fast table for large data models](https://github.com/thscharler/rat-ftable)
- [Table Widget | Ratatui](https://ratatui.rs/examples/widgets/table/)

---

### Pitfall 11: Channel Capacity Tuning — Deadlocks and Memory Spikes

**What goes wrong:**

Unbounded channels cause memory spikes when producer faster than consumer. Bounded channels with capacity too small cause deadlocks when send() blocks.

**Why it happens:**

mpsc::channel() creates unbounded channel. mpsc::sync_channel(n) requires capacity tuning. No universal right answer.

**Prevention:**

1. **Start with bounded, monitor backpressure**: sync_channel(1000)
2. **Backpressure metrics**: Track channel.len(), alert on sustained high values
3. **Match capacity to workload**: Profile producer/consumer rates
4. **Avoid blocking sends in async**: Use try_send() or tokio::sync::mpsc

**Confidence:** MEDIUM to HIGH

**Sources:**
- [Channels | Tokio Tutorial](https://tokio.rs/tokio/tutorial/channels)
- [Avoiding Over-Reliance on mpsc channels](https://blog.digital-horror.com/blog/how-to-avoid-over-reliance-on-mpsc/)

---

### Pitfall 12: Background Thread Cleanup on Exit — Resource Leaks

**What goes wrong:**

Background threads not properly joined on exit. Resources (file handles, memory) leak. Application exit hangs waiting for thread to finish infinite loop.

**Why it happens:**

Spawn thread, forget to join. Thread runs infinite loop without shutdown signal. Drop not called on thread-local storage.

**Prevention:**

1. **Shutdown signal pattern**: Arc<AtomicBool> shared with threads
2. **Join all threads before exit**: Store JoinHandle, call join() in cleanup
3. **Use scoped threads when possible**: Guarantee join before scope exit
4. **Tokio: use CancellationToken**: Structured cancellation

**Confidence:** MEDIUM

**Sources:**
- [Memory leak from ThreadLocal cleanup issues](https://github.com/open-telemetry/opentelemetry-java/issues/7083)
- [Ghostty Terminal Fixes Major Memory Leak (2026)](https://techplanet.today/post/ghostty-terminal-emulator-fixes-major-memory-leak-a-deep-dive-into-performance-debugging)

---

## Minor Pitfalls

### Pitfall 13: Missing Bounds Checks After Unsafe Optimization

**What goes wrong:**

Removing bounds checks with `.get_unchecked()` for performance, but calculation bug causes out-of-bounds access. Undefined behavior instead of panic.

**Prevention:**

1. **Avoid unsafe unless profiling proves 10%+ gain**
2. **Audit all unsafe blocks in code review**
3. **Miri testing**: cargo miri test catches UB
4. **Keep safe version in #[cfg(debug_assertions)]**

**Confidence:** HIGH

---

### Pitfall 14: Feature Flag Combinatorial Explosion — Untested Configurations

**What goes wrong:**

Adding cargo features (streaming, async, etc.) creates 2^n combinations. Some combinations untested, fail to compile.

**Prevention:**

1. **Limit features to orthogonal concerns**
2. **CI matrix testing**: cargo-all-features
3. **Feature flag hygiene**: Document incompatible combinations

**Confidence:** MEDIUM

**Sources:**
- [Cargo [features] explained with examples](https://dev.to/rimutaka/cargo-features-explained-with-examples-194g)
- [cargo-all-features: Build and test all feature flag combinations](https://github.com/frewsxcv/cargo-all-features)

---

### Pitfall 15: Export Feature Breaking with Streaming Data

**What goes wrong:**

Export to CSV/JSON assumes data is fully loaded in Vec. With streaming, iterator may consume data or fail to represent full dataset.

**Prevention:**

1. **Clone data for export**: Don't consume original iterator
2. **Test export with streaming enabled**: Verify all rows exported
3. **Progress indicator**: Export may be slower with streaming

**Confidence:** MEDIUM

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation | Priority |
|-------------|---------------|------------|----------|
| **Architecture** | Terminal rendering from background thread | Design threading boundaries: background for data, main for render | CRITICAL |
| **Architecture** | Async/sync impedance mismatch | Decide async vs threaded based on profiling, not assumptions | HIGH |
| **Streaming Implementation** | CSV library not actually streaming | Test with 1.8M row file, monitor memory | CRITICAL |
| **Streaming Implementation** | Channel deadlock from capacity | Start with bounded channel, monitor backpressure | MEDIUM |
| **Data Structure Refactor** | Breaking search/filter/export | Comprehensive integration tests before refactoring | CRITICAL |
| **Data Structure Refactor** | Lock contention with Arc<Mutex> | Use Arc<RwLock> for read-heavy workloads | HIGH |
| **Virtualization** | Off-by-one errors in viewport calculation | Unit tests for boundary conditions, property tests | HIGH |
| **Virtualization** | Standard Table widget O(n) rendering | Use rat-ftable or slice to visible rows only | HIGH |
| **Background Loading** | tokio::select! starving shutdown | Place shutdown branch first, use biased | MEDIUM |
| **Background Loading** | Background thread not cleaned up on exit | Shutdown signal pattern, join all threads | MEDIUM |
| **All Phases** | Terminal not restored on panic | Install panic hooks in ratatui::init() | HIGH |
| **All Phases** | Premature optimization without profiling | Measure first, optimize hot paths only | HIGH |
| **Testing/Release** | Performance regression undetected | Add criterion benchmarks to CI, track trends | MEDIUM |

---

## Testing Strategy to Prevent Pitfalls

### Regression Testing

1. **Integration tests for all features BEFORE refactoring**:
   - Search returns correct rows
   - Filter preserves row order
   - Export produces exact CSV/JSON
   - Column hide/show/reorder/resize works
   - Multi-tab and split view maintain state

2. **Snapshot testing for rendering output**:
   - Capture terminal output for known datasets
   - Detect unexpected rendering changes

### Performance Testing

1. **Criterion benchmarks in CI**:
   - Load time for 1M, 2M rows
   - Render time (should be constant regardless of dataset size)
   - Search time
   - Export time

2. **Memory profiling**:
   - Baseline: memory usage before optimization
   - Target: constant memory usage regardless of file size
   - Use `time -v`, heaptrack, or valgrind massif

### Property-Based Testing

1. **Proptest for virtualization logic**:
   - Generate random scroll offsets
   - Verify no panic
   - Verify row count matches viewport height (or less at end)

2. **Stateful property tests for UI state**:
   - Generate random sequence of operations (scroll, search, resize)
   - Verify invariants hold (no panics, data integrity)

### Stress Testing

1. **Large file testing**:
   - 1.8M rows (target)
   - 10M rows (stress test)
   - Empty file (0 rows)
   - Single row file

2. **Boundary conditions**:
   - First row, last row
   - Scroll to top, scroll to bottom
   - Window resize during operation

---

## Confidence Assessment

| Area | Confidence | Reasoning |
|------|------------|-----------|
| **Threading/ratatui** | HIGH | Official ratatui docs, GitHub discussions, verified patterns |
| **Async pitfalls** | HIGH | Recent 2026 blog posts, official Tokio docs, known issues |
| **Lock contention** | HIGH | Rust std docs, tokio docs, performance testing literature |
| **Data structure refactoring** | MEDIUM | General software engineering wisdom, Rust-specific patterns |
| **Virtualization off-by-one** | MEDIUM | UI programming patterns, not Rust-specific |
| **CSV streaming** | MEDIUM to HIGH | Real bug reports, csv crate docs, but library-dependent |
| **Terminal restoration** | HIGH | Official ratatui docs, well-documented pattern |
| **Channel tuning** | MEDIUM | Workload-dependent, no universal answer |

---

## Research Gaps

1. **Ratatui-specific virtualization patterns**: rat-ftable found but limited documentation. May need to study source code or prototype.

2. **Optimal channel capacity for CSV streaming**: Workload-dependent. Needs empirical testing with 1.8M row files.

3. **Benchmark regression thresholds**: What % regression is acceptable? Needs team decision and historical data.

4. **crossbeam vs tokio mpsc performance**: Conflicting information. Needs benchmarking for specific use case.

5. **Testing strategy for async/streaming code**: Less mature than sync testing. May need to develop custom test utilities.

---

## Recommended Phase Structure Based on Pitfalls

Based on pitfall severity and dependencies, recommended phase order:

1. **Phase: Architecture & Testing Baseline**
   - Addresses: Terminal restoration, testing infrastructure, profiling baseline
   - Prevents: Breaking features without detection, terminal state corruption
   - Rationale: Must have comprehensive tests BEFORE refactoring

2. **Phase: Streaming CSV Load**
   - Addresses: Memory optimization, CSV streaming validation
   - Prevents: OOM, fake streaming
   - Rationale: Foundation for large dataset support, needs validation before building on top

3. **Phase: Data Structure Migration**
   - Addresses: Breaking existing features, lock contention
   - Prevents: Search/filter/export regressions
   - Rationale: Most dangerous phase — comprehensive tests from Phase 1 are critical

4. **Phase: Virtualized Rendering**
   - Addresses: Off-by-one errors, O(n) rendering
   - Prevents: Visual glitches, performance regression
   - Rationale: Depends on stable data structure from Phase 3

5. **Phase: Background Loading & Threading**
   - Addresses: Thread safety, async pitfalls, deadlocks
   - Prevents: Rendering from background thread, blocking operations
   - Rationale: Most complex, builds on all previous phases

**Critical path**: Phase 1 (tests) → Phase 2 (streaming) → Phase 3 (data structure) → Phase 4 (virtualization) → Phase 5 (threading)

**Rationale**: Each phase validates its assumptions before the next depends on them. Testing comes first to catch regressions early.

---

## Sources

### High Confidence Sources (Official Documentation, Recent Articles)

- [Ratatui Official Documentation](https://ratatui.rs/)
- [Ratatui FAQ - Thread Safety](https://ratatui.rs/faq/)
- [Ratatui Async Template](https://github.com/ratatui/async-template)
- [Setup Panic Hooks | Ratatui](https://ratatui.rs/recipes/apps/panic-hooks/)
- [Tokio Tutorial: Select](https://tokio.rs/tokio/tutorial/select)
- [Tokio Tutorial: Channels](https://tokio.rs/tokio/tutorial/channels)
- [Tokio Tutorial: Shared State](https://tokio.rs/tokio/tutorial/shared-state)
- [Rust Async Just Killed Your Throughput (2026)](https://medium.com/@shkmonty35/rust-async-just-killed-your-throughput-and-you-didnt-notice-c38dd119aae5)
- [Rust Concurrency: Common Async Pitfalls Explained](https://leapcell.medium.com/rust-concurrency-common-async-pitfalls-explained-8f80d90b9a43)
- [How to deadlock Tokio application in Rust](https://turso.tech/blog/how-to-deadlock-tokio-application-in-rust-with-just-a-single-mutex)
- [Rust and CSV parsing - BurntSushi](https://burntsushi.net/csv/)
- [Property Testing Stateful Code in Rust](https://rtpg.co/2024/02/02/property-testing-with-imperative-rust/)

### Medium Confidence Sources (Community Patterns, General Rust Wisdom)

- [Ratatui Best Practices Discussion](https://github.com/ratatui/ratatui/discussions/220)
- [Avoiding Over-Reliance on mpsc channels](https://blog.digital-horror.com/blog/how-to-avoid-over-reliance-on-mpsc/)
- [Newtype Pattern - Rust Design Patterns](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html)
- [Stop Writing Slow Rust: 20 Rust Tricks](https://leapcell.medium.com/stop-writing-slow-rust-20-rust-tricks-that-changed-everything-0a69317cac3e)
- [Streaming CSV allocates too much memory](https://users.rust-lang.org/t/streaming-csv-allocates-too-much-memory/132765)
- [How to avoid bounds checks in Rust (without unsafe!)](https://shnatsel.medium.com/how-to-avoid-bounds-checks-in-rust-without-unsafe-f65e618b4c1e)

### Reference Documentation

- [Shared State Concurrency - Rust Book](https://doc.rust-lang.org/book/ch16-03-shared-state.html)
- [Arc in std::sync](https://doc.rust-lang.org/std/sync/struct.Arc.html)
- [RwLock in std::sync](https://doc.rust-lang.org/std/sync/struct.RwLock.html)
- [Why Async? - Async Book](https://rust-lang.github.io/async-book/01_getting_started/02_why_async.html)
- [RFC 1679: Panic Safe Slicing](https://rust-lang.github.io/rfcs/1679-panic-safe-slicing.md)
- [Features - The Cargo Book](https://doc.rust-lang.org/cargo/reference/features.html)
