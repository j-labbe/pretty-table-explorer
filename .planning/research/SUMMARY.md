# Project Research Summary

**Project:** Pretty Table Explorer - Performance Optimization
**Domain:** Rust TUI Table Viewer Performance Optimization
**Researched:** 2026-02-09
**Confidence:** HIGH

## Executive Summary

Pretty Table Explorer needs to handle 1.8M+ row datasets efficiently while maintaining responsive UI. The research shows the optimal approach is a **streaming architecture with compact storage and virtualized rendering**. The existing ratatui-based stack is solid and requires no framework changes—focus should be on three key areas: (1) moving data loading to a background thread with channel communication, (2) replacing naive `Vec<Vec<String>>` storage with string interning for 50-80% memory savings, and (3) implementing virtualized rendering that only materializes visible rows.

The recommended approach is **measure-first optimization**: establish profiling infrastructure (criterion, flamegraph, dhat) as phase 1, then apply targeted optimizations based on measured bottlenecks. Most performance gains come from algorithmic improvements (streaming I/O, lazy evaluation, virtualization) rather than micro-optimizations like custom allocators or parallel processing. The stdlib's BufReader with tuned buffers handles 1.8M rows efficiently without adding complexity.

The primary risk is **breaking correctness while chasing performance**. Ratatui's terminal backends are not Send/Sync, so rendering must stay on the main thread. Refactoring from `Vec<Vec<String>>` to compact storage will affect search, export, and column operations—comprehensive integration tests must be in place before this migration. Off-by-one errors in virtualized scrolling are common, requiring explicit boundary testing. The recommended phase order (infrastructure → streaming → storage → virtualization) builds on validated foundations and minimizes regression risk.

## Key Findings

### Recommended Stack

The existing ratatui v0.29 and crossterm v0.28 stack is production-ready—no framework changes needed. Focus on adding **profiling infrastructure** (critical for measure-first approach) and **memory-efficient data structures** (high impact, moderate complexity).

**Core technologies to add:**
- **criterion 0.8.1**: Statistical benchmarking for detecting regressions
- **cargo-flamegraph 0.6.11**: CPU profiling to identify hot paths
- **dhat 0.3.3**: Heap profiling to measure allocation patterns
- **string-interner 0.17** or **compact_str 0.8**: Memory-efficient storage (choose based on data repetition patterns)
- **itertools 0.14**: Lazy iterator combinators for memory-efficient batching

**Defer until profiling proves need:**
- **memmap2**: Memory-mapped I/O (only if I/O bottleneck proven, adds unsafe code)
- **rayon**: Parallel processing (only if CPU-bound parsing proven, overhead often neutralizes gains)
- **bumpalo**: Arena allocator (only if allocation bottleneck proven, adds manual lifetime management)

**Key principle from STACK.md:** Start with BufReader with 256KB-1MB buffer capacity. Only add complexity (mmap, custom allocators, parallel processing) after profiling shows specific bottlenecks and baseline optimization is exhausted.

### Expected Features

Research identified 7 table stakes features users expect in high-performance data viewers, plus 8 differentiators. The critical path is: **memory-efficient storage → progressive loading → virtualized rendering → smooth scrolling**.

**Must have (table stakes):**
- **Virtualized rendering** — Only visible rows rendered, standard for 100K+ datasets
- **Fast initial display** — Users expect <1s to see SOMETHING, not wait for full load
- **Smooth scrolling** — 60fps scrolling expected even at 1M+ rows
- **Visible row count** — Display "Loaded X / Total Y" during streaming
- **Loading feedback** — Spinner/progress indicator for operations >10s
- **Memory-efficient storage** — Can't hold 1.8M rows in naive String storage
- **Query cancellation** — Ability to abort long-running operations (Ctrl-C)

**Should have (competitive):**
- **Progressive/streaming loading** — Display updates as data arrives, see first 100 rows instantly
- **Incremental search** — Search-as-you-type with immediate visual feedback (needs indexed storage)
- **Lazy column loading** — Only parse/store columns visible in viewport
- **Smart chunk sizing** — Automatically tune batch size based on row width, terminal size

**Defer (v2+):**
- **Background data prefetch** — Adds threading complexity, simple streaming sufficient for MVP
- **Estimated total row count** — Nice-to-have, "Loading..." without estimate acceptable
- **Sparse loading indicators** — Polish, basic "Loaded X / Total" is sufficient

**Anti-features (explicitly avoid):**
- Infinite scroll without total count (users need context)
- Client-side sorting at scale (push to PostgreSQL, or warn for pipe mode)
- Multi-threaded parsing without coordination (Rust ownership makes complex, benefits marginal)

### Architecture Approach

Three architectural changes work together: streaming (background thread + channels), compact storage (string interning), and virtualized rendering (only visible rows).

**Major components:**
1. **Background Parser Thread** — Reads stdin via BufReader with 512KB buffer, sends batches of 1000 rows via mpsc::channel to main thread. Avoids blocking UI initialization.
2. **Compact Table Storage** — Either InternedStorage (50-80% savings for high repetition) or CompactString-based (30-50% savings for low repetition). Implements TableStorage trait with `get_range(Range<usize>)` returning lazy iterator.
3. **Main Thread Event Loop** — Non-blocking channel polling with `try_recv()`, processes parser messages, handles input with `crossterm::poll(16ms)`, renders with frame-rate limiting (30 FPS sufficient, reduces CPU 50%).
4. **Virtualized Viewport** — Calculates visible row range from TableState offset and area.height, only materializes those rows via `storage.get_range(offset..offset+height)`.

**Key pattern from ARCHITECTURE.md:** Keep rendering on main thread only. Ratatui is thread-safe but terminal backends are not Send/Sync. Background threads communicate via channels, main thread owns Terminal and renders.

**Data flow:**
```
stdin → Background Thread (BufReader) → mpsc::channel → Main Thread (try_recv)
                                                              ↓
                                          App::storage (InternedStorage/CompactTable)
                                                              ↓
                                          Viewport calculation (offset..offset+height)
                                                              ↓
                                          Ratatui Table widget (only visible rows)
```

**Performance expectations after optimization:**
- Startup time: 2-5 min (blocking) → <100ms (immediate)
- Memory usage: ~2GB → 400MB-1GB (50-80% reduction)
- Scroll latency: 50-200ms → <16ms (60 FPS)
- Frame rate: Varies → Stable 30-60 FPS

### Critical Pitfalls

Top 5 pitfalls from PITFALLS.md that most threaten project success:

1. **Terminal Backend Not Send/Sync — Rendering from Background Threads**
   - **Risk:** Compilation errors or runtime panics if attempting `terminal.draw()` from background thread
   - **Prevention:** Design threading boundaries early—background for data loading, main thread for all rendering
   - **Detection:** Compiler errors about Send/Sync traits, terminal appearing frozen

2. **Async/Sync Impedance Mismatch — Blocking Operations in Async Code**
   - **Risk:** 40% throughput degradation from calling std::sync::Mutex or blocking I/O in async functions
   - **Prevention:** Profile before/after async conversion, use `tokio::task::spawn_blocking()` for CPU work
   - **Decision:** Consider native threads + mpsc instead of async—simpler for stdin → UI data flow

3. **Data Structure Refactoring Breaking Existing Features**
   - **Risk:** Changing from Vec<Vec<String>> breaks search, export, column operations with subtle bugs
   - **Prevention:** Comprehensive integration tests BEFORE refactoring, incremental migration with feature flags
   - **Critical phase:** Phase 2 (storage migration) is highest risk period

4. **CSV Streaming Not Actually Streaming — Memory Explosion**
   - **Risk:** "Streaming" library buffers entire file in memory before returning iterator
   - **Prevention:** Test with 1.8M row file and monitor memory with `time -v` or heaptrack
   - **Validation:** Memory usage should be constant after buffer fills, not grow with file size

5. **Virtualized Scrolling Off-By-One Errors**
   - **Risk:** Visual glitches (duplicate rows, blank rows), panic on scroll-to-bottom, wrong row highlighting
   - **Prevention:** Unit tests for boundary conditions (first row, last row, empty data), explicit Range types
   - **Detection:** Screenshot tests, property-based testing with proptest generating random scroll offsets

**Additional critical safeguard:** Install panic hooks early (use `ratatui::init()` or manual setup) to restore terminal state on crash. Without this, panics leave terminal in alternate screen + raw mode, appearing frozen to users.

## Implications for Roadmap

Based on research dependencies and pitfall severity, recommended 5-phase structure:

### Phase 1: Profiling Infrastructure & Testing Baseline
**Rationale:** Must have comprehensive tests and measurement tools BEFORE refactoring. Can't optimize without measuring, can't detect regressions without tests.

**Delivers:**
- Criterion benchmarks for parsing, rendering, search (detect regressions in CI)
- Integration tests for search, export, column operations (validate before storage migration)
- Flamegraph and dhat profiling setup (identify bottlenecks for targeted optimization)
- Panic hooks to restore terminal state (user experience safeguard)

**Addresses:** Pitfall #9 (premature optimization without measurement), foundation for all subsequent phases

**Research flag:** Standard patterns, skip research-phase. Criterion and profiling tools are well-documented.

### Phase 2: Streaming I/O with Background Thread
**Rationale:** Unblocks UI immediately (most visible improvement), foundation for progressive loading, validates threading architecture before storage changes.

**Delivers:**
- Background parser thread reading stdin via BufReader (512KB buffer)
- mpsc::channel communication to main thread (batch size: 1000 rows)
- Non-blocking event loop with `try_recv()` and `crossterm::poll(16ms)`
- Loading indicator showing "Loaded X rows" during streaming

**Addresses:**
- FEATURES.md: Fast initial display, loading feedback, progressive loading
- PITFALLS.md: Validates threading boundaries (background data, main render)

**Avoids:** Pitfall #1 (rendering from background thread), Pitfall #8 (terminal state on panic)

**Research flag:** Needs light research for channel capacity tuning (workload-dependent, empirical testing needed). Standard mpsc patterns are well-documented, but optimal batch size needs profiling.

### Phase 3: Compact Storage Migration
**Rationale:** Highest risk phase (breaks existing features), depends on Phase 1 tests to catch regressions. Memory optimization enables larger datasets.

**Delivers:**
- TableStorage trait with `add_batch()` and `get_range()` methods
- InternedStorage implementation (string interning for 50-80% savings)
- Migration from `Vec<Vec<String>>` to `Box<dyn TableStorage>`
- Updated search, export, column operations to use new storage API

**Uses:** STACK.md: string-interner 0.17 or compact_str 0.8 (choose based on profiling)

**Implements:** ARCHITECTURE.md: Compact Table Storage component

**Addresses:**
- FEATURES.md: Memory-efficient storage (table stakes)
- PITFALLS.md: Data structure refactoring risks (#5), lock contention with shared state (#3)

**Avoids:** Pitfall #5 (breaking existing features) via Phase 1 tests detecting regressions

**Research flag:** Needs deep research for storage strategy. Compare string interning vs CompactString vs Apache Arrow based on actual psql data patterns. Property-based testing with proptest for storage invariants.

### Phase 4: Virtualized Rendering
**Rationale:** Depends on stable storage from Phase 3. Optimizes what's already working (polish, not foundation).

**Delivers:**
- Viewport calculation from TableState offset and terminal height
- Only visible rows passed to Table widget (offset..offset+height slice)
- Frame rate limiting (30 FPS target, reduces CPU 50%)
- Scroll position preservation during live updates

**Uses:** STACK.md: itertools 0.14 for lazy iteration (chunks, batching)

**Addresses:**
- FEATURES.md: Virtualized rendering, smooth scrolling (table stakes)
- PITFALLS.md: Off-by-one errors (#6), O(n) rendering with standard widget (#10)

**Avoids:** Pitfall #6 (off-by-one) via unit tests for boundary conditions (first row, last row, empty data, data < viewport)

**Research flag:** Skip research-phase. Virtualization patterns are well-documented in ratatui examples and rat-ftable/rat-salsa. Boundary testing is standard UI programming practice.

### Phase 5: Query Cancellation & Cleanup
**Rationale:** Safety valve for user mistakes, ensures clean shutdown. Can be done in parallel with Phase 4.

**Delivers:**
- Ctrl-C signal handler sending shutdown message via channel
- Background thread checking for shutdown signal between batches
- Clean thread join on exit (no resource leaks)
- PostgreSQL cancel signal for database queries

**Addresses:**
- FEATURES.md: Query cancellation (table stakes)
- PITFALLS.md: Background thread cleanup (#12)

**Research flag:** Skip research-phase. Signal handling and thread shutdown patterns are standard.

### Phase Ordering Rationale

**Why this order:**
1. **Testing first** prevents breaking features during refactoring (Phase 3 most dangerous)
2. **Streaming before storage** validates threading architecture with minimal risk
3. **Storage before virtualization** ensures stable data access patterns
4. **Cancellation last** builds on validated threading from Phase 2

**Dependency chain:**
- Phase 1 (tests) → Phase 3 (storage migration requires tests to catch regressions)
- Phase 2 (streaming) → Phase 3 (background loading + compact storage work together)
- Phase 3 (storage) → Phase 4 (virtualization needs stable get_range() API)
- Phase 2 (threads) → Phase 5 (shutdown signals need thread coordination)

**Pitfall mitigation:**
- Phase 1 catches regressions early (prevents Pitfall #5)
- Phase 2 validates threading boundaries (prevents Pitfall #1)
- Phase 3 with tests from Phase 1 minimizes refactoring risk
- Phase 4 with boundary tests prevents off-by-one errors (Pitfall #6)

**Parallelization opportunities:**
- Phase 5 can overlap with Phase 4 (independent concerns)
- Phase 1 testing continues throughout (regression detection)

### Research Flags

**Needs deeper research during planning:**
- **Phase 2:** Channel capacity tuning (bounded vs unbounded, optimal batch size for 1.8M rows)—workload-dependent, needs empirical testing
- **Phase 3:** Storage strategy comparison (interning vs CompactString vs Arrow)—depends on actual data patterns, needs profiling with representative psql output

**Standard patterns (skip research-phase):**
- **Phase 1:** Criterion, flamegraph, dhat all have extensive documentation and examples
- **Phase 4:** Virtualization patterns well-documented in ratatui ecosystem (rat-ftable source code, official examples)
- **Phase 5:** Signal handling and thread cleanup are standard Rust concurrency patterns

**Open questions for Phase 3 (storage migration):**
1. String interning vs compact_str: Depends on value repetition in target datasets—needs benchmark with representative psql data
2. Batch size for channel: 1000 rows is estimate—profile to find optimal (balance latency vs throughput)
3. Column width sampling: Is 1000 rows sufficient for width calculation, or scan all if <10K total?

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| **Stack** | HIGH | Official ratatui docs, BurntSushi CSV benchmarks, criterion/flamegraph mature tools. Clear recommendations: BufReader first, add complexity only after profiling. |
| **Features** | MEDIUM | Verified via web research and official docs for virtualization patterns. Table stakes features align with VisiData and FastTableViewer examples. Lacks Context7 verification for TUI-specific patterns. |
| **Architecture** | HIGH | Ratatui threading patterns verified in official docs and async template. String interning proven in rustc (2000x compression in real-world case). Channel-based architecture is standard Rust pattern. |
| **Pitfalls** | HIGH | Terminal Send/Sync issue verified in ratatui FAQ and GitHub discussions. Async pitfalls confirmed with recent 2026 blog posts and Tokio docs. Data structure refactoring risks are general software engineering wisdom. |

**Overall confidence:** HIGH

The stack and architecture recommendations are strongly supported by official documentation and proven patterns. Feature prioritization is based on observed patterns in similar tools (VisiData, FastTableViewer) with some extrapolation to TUI domain. Pitfalls are well-documented in Rust ecosystem with recent real-world examples.

### Gaps to Address

**Gap 1: Optimal storage strategy depends on data characteristics**
- **Issue:** String interning excels with high repetition (50-80% savings), CompactString with many short strings (30-50% savings), Arrow with analytics use cases (60-90% savings)
- **Resolution:** Phase 3 planning should include benchmarking with representative psql output (e.g., 10K row sample from target queries). Start with string interning as default, validate with dhat profiling.

**Gap 2: Channel capacity tuning is workload-dependent**
- **Issue:** Unbounded channels cause memory spikes, bounded channels with wrong capacity cause deadlocks. No universal right answer.
- **Resolution:** Phase 2 implementation should start with `sync_channel(1000)`, add metrics for channel.len(), adjust based on profiling with 1.8M row test case.

**Gap 3: ratatui virtualization integration needs prototyping**
- **Issue:** rat-ftable/rat-salsa provide virtual table widgets, but integration with existing TableState and custom storage needs validation
- **Resolution:** Phase 4 planning should include small prototype: custom storage + rat-salsa TableData trait implementation. Fallback: calculate visible range manually and slice before passing to standard Table widget.

**Gap 4: Search indexing strategy for 1.8M rows**
- **Issue:** Linear search through 1.8M rows will be slow (O(n)). Incremental search feature needs indexed storage.
- **Resolution:** Defer to post-v1.4 milestone. MVP uses linear search on loaded data, displays "searching loaded rows only" during streaming. Index research is out of scope for performance optimization milestone.

**Gap 5: PostgreSQL vs pipe mode performance differences**
- **Issue:** Research focused on stdin parsing (pipe mode). PostgreSQL mode has different characteristics (cursor-based pagination, network I/O).
- **Resolution:** Phase 2 should validate streaming architecture works for both modes. PostgreSQL benefits from server-side cursors (FETCH N), less parsing overhead. If divergence is significant, may need mode-specific optimizations.

## Sources

### Primary (HIGH confidence)

**Ratatui Official Documentation:**
- [Ratatui Official Site](https://ratatui.rs/) — Thread safety, immediate-mode rendering, performance characteristics
- [Ratatui FAQ](https://ratatui.rs/faq/) — Terminal backend Send/Sync constraints, async patterns
- [Ratatui Rendering Concepts](https://ratatui.rs/concepts/rendering/) — Buffer diffing, optimization patterns
- [Ratatui Async Template](https://ratatui.github.io/async-template/) — Background threading architecture
- [Setup Panic Hooks | Ratatui](https://ratatui.rs/recipes/apps/panic-hooks/) — Terminal state restoration

**Rust Performance & Profiling:**
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/) — Profiling tools, heap allocations, iterators, hashing
- [Rust and CSV parsing - BurntSushi](https://burntsushi.net/csv/) — CSV crate performance benchmarks (241 MB/s raw, 146 MB/s byte records)
- [criterion crate](https://crates.io/crates/criterion) — Statistical benchmarking for regression detection
- [cargo-flamegraph GitHub](https://github.com/flamegraph-rs/flamegraph) — CPU profiling integration
- [dhat-rs GitHub](https://github.com/nnethercote/dhat-rs) — Heap profiling for allocation tracking

**Rust Concurrency & Threading:**
- [Tokio Tutorial: Select, Channels, Shared State](https://tokio.rs/tokio/tutorial/) — Async patterns, deadlock prevention
- [Shared State Concurrency - Rust Book](https://doc.rust-lang.org/book/ch16-03-shared-state.html) — Arc, Mutex, RwLock patterns
- [Rust async-book: Why Async?](https://rust-lang.github.io/async-book/01_getting_started/02_why_async.html) — When to use async vs threads

**Memory Optimization:**
- [Fast and Simple Rust Interner - matklad](https://matklad.github.io/2020/03/22/fast-simple-rust-interner.html) — Rustc string interning approach
- [The Power of Interning: 2000x Smaller](https://gendignoux.com/blog/2025/03/03/rust-interning-2000x.html) — Real-world memory savings case study
- [string-interner crate](https://docs.rs/string-interner) — API documentation, usage patterns
- [compact_str Documentation](https://docs.rs/compact_str) — Small string optimization, inline storage

### Secondary (MEDIUM confidence)

**Performance Patterns & Pitfalls:**
- [Rust Async Just Killed Your Throughput (2026)](https://medium.com/@shkmonty35/rust-async-just-killed-your-throughput-and-you-didnt-notice-c38dd119aae5) — 40% degradation from blocking operations in async
- [Rust Concurrency: Common Async Pitfalls Explained](https://leapcell.medium.com/rust-concurrency-common-async-pitfalls-explained-8f80d90b9a43) — Lock contention, deadlock patterns
- [How to deadlock Tokio application in Rust](https://turso.tech/blog/how-to-deadlock-tokio-application-in-rust-with-just-a-single-mutex) — select! with mutex ordering issues
- [Stop Writing Slow Rust: 20 Rust Tricks](https://leapcell.medium.com/stop-writing-slow-rust-20-rust-tricks-that-changed-everything-0a69317cac3e) — Optimization strategies

**TUI Performance Examples:**
- [VisiData](https://www.visidata.org/) — Reference implementation for interactive tabular data (Python, but patterns applicable)
- [FastTableViewer (tv)](https://github.com/codechenx/tv) — Terminal CSV/TSV viewer with streaming
- [Go vs. Rust for TUI Development: Bubbletea and Ratatui](https://dev-tngsh.medium.com/go-vs-rust-for-tui-development-a-deep-dive-into-bubbletea-and-ratatui-9af65c0b535b) — Ratatui performance comparison (30-40% less memory, 15% less CPU)

**UX Patterns for Loading & Virtualization:**
- [UX Patterns: Progressive Loading](https://uxpatterns.dev/glossary/progressive-loading) — User expectation for streaming interfaces
- [Nielsen Norman Group: Progress Indicators](https://www.nngroup.com/articles/progress-indicators/) — When to show percentage vs spinner
- [Patterns.dev: List Virtualization](https://www.patterns.dev/vanilla/virtual-lists/) — General virtualization patterns

### Tertiary (LOW confidence, needs validation)

**Apache Arrow (Alternative Storage):**
- [Apache Arrow Rust](https://arrow.apache.org/rust/arrow/index.html) — Columnar format documentation
- [Apache Arrow FAQ](https://arrow.apache.org/faq/) — Use cases and performance characteristics
- Note: Arrow is overkill for simple table viewer, but relevant if analytics features planned

**Property Testing:**
- [Property Testing Stateful Code in Rust](https://rtpg.co/2024/02/02/property-testing-with-imperative-rust/) — Proptest patterns for UI state invariants
- [Newtype Pattern - Rust Design Patterns](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html) — Safe refactoring strategies

---
*Research completed: 2026-02-09*
*Ready for roadmap: yes*
