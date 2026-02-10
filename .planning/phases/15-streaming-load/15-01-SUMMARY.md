---
phase: 15-streaming-load
plan: 01
subsystem: streaming
tags: [streaming, background-thread, mpsc, non-blocking, architecture]
dependency_graph:
  requires: [parser]
  provides: [streaming-parser, incremental-parsing]
  affects: []
tech_stack:
  added: [std::thread, std::sync::mpsc, std::sync::atomic]
  patterns: [background-thread, channel-communication, atomic-counters]
key_files:
  created:
    - src/streaming.rs
  modified:
    - src/parser.rs
    - src/lib.rs
decisions:
  - Use unbounded mpsc channel (bounded would block sender, memory addressed in Phase 16)
  - Headers parsed synchronously before background thread spawn (always in first 2-3 lines)
  - Batch size 1000 rows for efficient channel communication
  - AtomicBool for cancellation, AtomicUsize for row counter (Relaxed ordering sufficient)
  - Drop impl joins thread to prevent data loss
metrics:
  duration_minutes: 2
  tasks_completed: 2
  files_created: 1
  files_modified: 2
  tests_added: 5
  commits: 2
  completed_date: 2026-02-10
---

# Phase 15 Plan 01: Streaming Parser Foundation Summary

**One-liner:** Background thread + mpsc channel architecture for non-blocking stdin parsing with 1000-row batches and atomic progress tracking

## Overview

Created the streaming parser module that reads stdin in a background thread and sends parsed rows through an mpsc channel. This is the foundation for streaming load that enables non-blocking data loading for large datasets.

**Purpose:** Without this, the UI would freeze during large data loads. The background thread + channel architecture allows the main thread to remain responsive while data is being parsed.

## Tasks Completed

### Task 1: Add incremental parsing functions to parser.rs

**Status:** ✅ Complete
**Commit:** ef64351
**Duration:** ~1 minute

Added two public functions to parser.rs for line-by-line parsing:

1. `parse_psql_header(lines: &[&str]) -> Option<(Vec<String>, usize)>`
   - Parses column headers from first few lines
   - Validates separator line contains `---`
   - Returns headers and data_start_index where data rows begin
   - Returns None if headers/separator missing

2. `parse_psql_line(line: &str, column_count: usize) -> Option<Vec<String>>`
   - Parses single data row from psql output
   - Returns None for empty lines and footer lines
   - Doesn't strictly enforce column_count (matches existing parse_psql behavior)

**Tests added (5):**
- `test_parse_psql_header_valid` - Standard header + separator
- `test_parse_psql_header_no_separator` - Returns None when separator missing
- `test_parse_psql_line_data_row` - Standard pipe-delimited row
- `test_parse_psql_line_footer` - Footer line "(2 rows)" returns None
- `test_parse_psql_line_empty` - Empty/whitespace line returns None

All existing parse_psql tests still pass (13 total parser tests).

**Files modified:**
- `src/parser.rs` - Added incremental parsing functions

### Task 2: Create StreamingParser module

**Status:** ✅ Complete
**Commit:** 479d75f
**Duration:** ~1 minute

Created `src/streaming.rs` with StreamingParser struct that manages background stdin parsing.

**Architecture:**
- Headers parsed synchronously (blocking) before construction - always in first 2-3 lines
- BufReader created once and moved to background thread (stdin already partially consumed)
- Background thread reads remaining stdin line-by-line
- Rows batched (1000 rows) before sending through unbounded mpsc channel
- Atomic counters for non-blocking progress tracking

**StreamingParser fields:**
- `receiver: Receiver<Vec<Vec<String>>>` - Receives row batches
- `row_count: Arc<AtomicUsize>` - Total rows parsed (updated by background thread)
- `cancelled: Arc<AtomicBool>` - Cancellation signal
- `complete: Arc<AtomicBool>` - Set when background thread finishes
- `thread_handle: Option<JoinHandle<io::Result<()>>>` - For joining on drop
- `headers: Vec<String>` - Parsed headers (available immediately)

**Public API:**
- `from_stdin() -> io::Result<Option<Self>>` - Creates parser, returns None if no valid headers
- `try_recv_batch(max_rows: usize) -> Vec<Vec<String>>` - Non-blocking batch receive
- `total_rows_parsed() -> usize` - Atomic counter read (Ordering::Relaxed)
- `cancel()` - Sets cancelled flag (Ordering::Relaxed)
- `is_complete() -> bool` - Reads complete flag (Ordering::Acquire)
- `headers() -> &[String]` - Returns header reference

**Drop implementation:**
- Sets cancelled flag (if not already set)
- Joins thread handle to wait for thread completion

**Files created:**
- `src/streaming.rs` - StreamingParser struct (220 lines)

**Files modified:**
- `src/lib.rs` - Added `pub mod streaming;` export

**Dependencies:** Zero external dependencies, uses only std library:
- `std::thread` - Background thread
- `std::sync::mpsc` - Channel communication
- `std::sync::Arc` - Shared ownership for atomics
- `std::sync::atomic` - AtomicBool, AtomicUsize

## Verification Results

All verification criteria met:

1. ✅ `cargo build` succeeds with no errors
2. ✅ `cargo clippy` has no warnings
3. ✅ `cargo test --lib` passes all 36 tests (13 parser + 23 other modules)
4. ✅ `src/streaming.rs` exists and contains StreamingParser struct
5. ✅ `src/parser.rs` contains parse_psql_header and parse_psql_line functions
6. ✅ `src/lib.rs` exports the streaming module

## Self-Check: PASSED

**Files created:**
- ✅ FOUND: src/streaming.rs
- ✅ FOUND: StreamingParser struct
- ✅ FOUND: parse_psql_header
- ✅ FOUND: parse_psql_line
- ✅ FOUND: streaming module export

**Commits created:**
- ✅ FOUND: ef64351 (Task 1 - incremental parsing functions)
- ✅ FOUND: 479d75f (Task 2 - StreamingParser module)

All artifacts verified on disk and in git history.

## Success Criteria

All success criteria met:

- ✅ StreamingParser struct compiles and exposes documented public API
- ✅ Incremental parser functions produce identical output to existing parse_psql
- ✅ Background thread architecture uses mpsc channel (not Arc<Mutex<Vec>>)
- ✅ Cancellation uses AtomicBool, row counter uses AtomicUsize
- ✅ Drop implementation joins thread handle
- ✅ All existing tests still pass (36 tests total)

## Deviations from Plan

None - plan executed exactly as written.

No bugs found, no missing critical functionality, no blocking issues, no architectural changes needed.

## Key Decisions

1. **Unbounded channel:** Used `mpsc::channel()` instead of bounded channel. Bounded channels block the sender which we don't want. Memory pressure will be addressed in Phase 16.

2. **Synchronous header parsing:** Headers are parsed synchronously (blocking) before spawning the background thread. This is acceptable because headers are always in the first 2-3 lines and blocks for <1ms.

3. **BufReader lifecycle:** Created once from stdin and moved to background thread. Cannot create new BufReader inside thread because stdin would have been partially consumed during header parsing.

4. **Batch size 1000:** Balances channel message overhead vs memory usage per message. Can be tuned in Phase 16 if needed.

5. **Relaxed ordering:** AtomicBool (cancellation) and AtomicUsize (row counter) use Relaxed ordering since exact synchronization not critical. Complete flag uses Acquire ordering for happens-before guarantee.

6. **Drop joins thread:** Ensures background thread is properly cleaned up and prevents data loss by waiting for thread to finish processing.

## Technical Notes

**Thread safety:**
- All shared state uses Arc for safe sharing across threads
- Atomic operations for lock-free counters and flags
- Channel handles synchronization for row batches

**Error handling:**
- from_stdin() returns io::Result for I/O errors
- Returns Ok(None) if headers not found (caller can provide error message)
- Background thread returns io::Result from spawned closure
- Channel disconnect handled gracefully (stops reading)

**Performance characteristics:**
- Headers: <1ms (synchronous, first 2-3 lines)
- Background thread: Non-blocking from main thread perspective
- Batch size 1000: Reduces channel message overhead
- Atomic counters: Lock-free, O(1) read

**Integration points:**
- Used by main.rs for stdin mode (Phase 15 Plan 02)
- Requires parser.rs incremental functions (this plan)
- Provides foundation for Phase 16 memory optimization

## Next Steps

**Phase 15 Plan 02** will integrate StreamingParser into main.rs:
- Replace synchronous stdin read with StreamingParser::from_stdin()
- Add loading state to AppMode (Normal, Loading, Ready)
- Render "Loading... N rows" message during background parsing
- Call try_recv_batch() in event loop to incrementally add rows to workspace

**Phase 16** will optimize storage and memory:
- Replace Vec<Vec<String>> with compact storage (interning or CompactString)
- Tune batch size based on empirical testing with 1.8M rows
- Add memory pressure handling if needed

## Impact

**Enables:**
- Non-blocking data loading for large datasets
- Responsive UI during parsing (event loop not blocked)
- Incremental row display as data arrives
- Cancellable loading (can exit before completion)

**Foundations for:**
- Phase 16: Memory optimization (compact storage integration point)
- Phase 17: Virtualized rendering (row availability check)
- v1.4 milestone: 1.8M row dataset handling

**Architectural patterns established:**
- Background thread for long-running operations
- Channel-based communication with main thread
- Atomic counters for progress tracking without locks
- Graceful shutdown via cancellation flags

## Commits

1. **ef64351** - feat(15-01): add incremental parsing functions
   - parse_psql_header() for header-only parsing
   - parse_psql_line() for single row parsing
   - 5 unit tests covering all edge cases

2. **479d75f** - feat(15-01): create StreamingParser module
   - StreamingParser struct with background thread
   - Non-blocking API: try_recv_batch(), total_rows_parsed(), cancel(), is_complete(), headers()
   - Drop impl joins thread to prevent data loss
   - Zero external dependencies

**Duration:** 2 minutes
**Total tests:** 36 (13 parser + 23 other modules)
**Total lines added:** ~341 (121 parser.rs + 220 streaming.rs)
