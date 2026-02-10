---
phase: 15-streaming-load
plan: 02
subsystem: ui
tags: [streaming, loading-indicator, viewport-rendering, event-loop, integration]
dependency_graph:
  requires:
    - phase: 15-01
      provides: StreamingParser, incremental-parsing
  provides:
    - streaming-integration
    - viewport-windowed-rendering
    - loading-progress-indicator
  affects: [16-memory-optimization]
tech_stack:
  added: []
  patterns: [viewport-windowing, streaming-event-loop, progressive-loading]
key_files:
  created: []
  modified:
    - src/main.rs
    - src/render.rs
    - src/workspace.rs
    - src/state.rs
decisions:
  - StreamingParser integrated into stdin path, replaces blocking read_to_string
  - Viewport-windowed rendering: only calculate widths for visible rows + buffer
  - Loading indicator reuses status_message infrastructure
  - Ctrl+C cancels loading but keeps app running with partial data
  - Batch size 5000 rows per poll (larger than research's 1000 to drain channel faster)
  - Window size 10,000 rows for width calculation (visible + scroll buffer)
patterns_established:
  - "Viewport windowing: Calculate column widths only for visible rows + buffer zone"
  - "Progressive loading: Data displayed immediately, rows added incrementally"
  - "Graceful cancellation: First Ctrl+C cancels load, second quits app"
metrics:
  duration_minutes: 75
  tasks_completed: 2
  files_created: 0
  files_modified: 7
  commits: 3
  completed_date: 2026-02-10
---

# Phase 15 Plan 02: Streaming Integration Summary

**StreamingParser integrated with viewport-windowed rendering, sub-second initial display for 500k+ rows, responsive navigation during load, and graceful Ctrl+C cancellation**

## Overview

Integrated StreamingParser into main.rs event loop, enabling immediate data display while background parsing continues. Added viewport-windowed rendering to maintain performance with large datasets. Users now see first rows within 1 second even for 500k+ row datasets, with full UI responsiveness during loading.

**Purpose:** Connects streaming infrastructure (15-01) to actual user experience. Without this, large datasets would block the UI during parsing. Now data appears instantly and users can navigate, search, and interact while loading continues.

## Performance

- **Duration:** 75 min (1h 15m)
- **Started:** 2026-02-10T18:28:54Z
- **Completed:** 2026-02-10T19:44:00Z
- **Tasks:** 2 (1 auto + 1 checkpoint human-verify)
- **Files modified:** 7

## Accomplishments

- StreamingParser integrated into stdin path - data appears instantly
- Viewport-windowed rendering maintains performance with 500k+ rows
- Loading indicator shows progressive row count, updates every ~250ms
- Full UI responsiveness during streaming (navigation, search, column controls)
- Ctrl+C cancellation works cleanly - first press cancels load, keeps browsing
- All 5 verification scenarios passed

## Task Commits

Each task was committed atomically:

1. **Task 1: Integrate StreamingParser into main.rs event loop** - `951c910` (feat)
   - Initial integration with streaming poll in event loop
   - Loading indicator using status_message infrastructure
   - Ctrl+C cancellation handler

**Performance & bug fixes (deviations):**

2. **Viewport-windowed rendering** - `059b9be` (perf)
   - Addresses performance degradation with large datasets
   - Calculates column widths only for visible rows + buffer
   - Window size 10,000 rows (configurable)

3. **Loading indicator completion** - `fa00010` (fix)
   - Fixes loading indicator not clearing when complete
   - Fixes row count accuracy during streaming
   - Completion message shows final count

4. **Task 2: Human verification checkpoint** - APPROVED
   - All 5 scenarios verified by user

**Plan metadata:** (pending in this commit)

## Files Created/Modified

- `src/main.rs` - StreamingParser integration, event loop polling, loading state tracking
- `src/render.rs` - Viewport-windowed width calculation, loading message rendering
- `src/workspace.rs` - Window configuration (VISIBLE_WINDOW_SIZE, BUFFER_ZONE)
- `src/state.rs` - AppMode::Loading variant for loading state tracking
- `benches/rendering.rs` - Updated window size constant references
- `benches/scrolling.rs` - Updated window size constant references
- `tests/search_tests.rs` - Updated test constants for window sizing

## Decisions Made

1. **StreamingParser stdin integration:** Replaced blocking `read_to_string()` with `StreamingParser::from_stdin()` in stdin mode. Headers available immediately, rows stream in background.

2. **Viewport-windowed rendering:** Calculate column widths only for visible rows + buffer zone (10k rows total). Prevents O(n) cost on every frame for large datasets. Critical for 500k+ row performance.

3. **Loading indicator lifecycle:** Reuses existing status_message infrastructure. Shows "Loading... N rows" during streaming, updates every frame. Clears automatically when complete or shows "Loaded N rows" briefly.

4. **Batch size 5000:** Polls channel for up to 5000 rows per event loop iteration. Larger than research's 1000 because we want to drain channel quickly and minimize buffering in channel.

5. **Graceful cancellation:** First Ctrl+C during loading cancels background parsing but keeps app running with partial data. Second Ctrl+C (or 'q') quits app. Provides "cancel but keep browsing" behavior.

6. **Window size tuning:** 10,000 row window (VISIBLE_WINDOW_SIZE=1000 + BUFFER_ZONE=9000) balances performance vs accuracy. Too small = inaccurate column widths when scrolling. Too large = slow width calculations.

## Deviations from Plan

Plan specified implementing streaming integration as described. During execution, discovered performance degradation with large datasets required additional work. All fixes aligned with deviation rules.

### Auto-fixed Issues

**1. [Rule 1 - Bug] Viewport-windowed rendering for streaming performance**
- **Found during:** Task 1 verification (500k row test)
- **Issue:** With 500k rows loaded, column width calculation runs on ALL rows every frame (O(n) cost). This caused noticeable lag during scrolling and navigation after large datasets finished loading. Width calculation was iterating through hundreds of thousands of rows unnecessarily.
- **Fix:** Implemented viewport-windowed rendering that calculates column widths only for visible rows + buffer zone. Added VISIBLE_WINDOW_SIZE (1000 rows) and BUFFER_ZONE (9000 rows) constants to workspace.rs. Modified render.rs `build_pane_render_data()` to slice data to window around current scroll position before width calculation. Now O(window_size) instead of O(total_rows).
- **Files modified:** src/render.rs, src/workspace.rs, benches/rendering.rs, benches/scrolling.rs, tests/search_tests.rs
- **Verification:** 500k row dataset remains responsive after loading completes. Scrolling is smooth. Width calculation no longer iterates full dataset.
- **Committed in:** 059b9be (perf commit after task)

**2. [Rule 1 - Bug] Loading indicator completion and row count accuracy**
- **Found during:** Task 1 verification (observing loading completion)
- **Issue:** Loading indicator didn't clear properly when streaming completed. Status message showed "Loading..." even after background thread finished. Additionally, row count displayed during loading was slightly inaccurate because `loaded_count` came from StreamingParser atomic counter but didn't account for rows already appended to workspace.
- **Fix:** Added `AppMode::Loading` enum variant to state.rs to properly track loading state. Modified main.rs to set AppMode when streaming starts and clear it when complete. Added completion message showing final row count. Fixed row count to use `workspace.active_pane().data.rows.len()` for accurate count of rows in UI.
- **Files modified:** src/main.rs, src/state.rs
- **Verification:** Loading indicator clears when complete. Completion message briefly shows "Loaded N rows". Row count accurate during streaming.
- **Committed in:** fa00010 (fix commit after perf)

---

**Total deviations:** 2 auto-fixed (Rule 1 - bugs found during verification)
**Impact on plan:** Both fixes essential for correct behavior and performance at scale. Viewport windowing prevents O(n) performance degradation with large datasets. Loading indicator fixes ensure proper UX feedback. No scope creep - both address correctness issues discovered during planned verification.

## Issues Encountered

None - plan executed smoothly with expected verification-time discoveries.

**Note:** The performance issue (viewport windowing) was found during the large dataset verification scenario (Task 2, scenario 2). This is expected behavior - verification often reveals performance characteristics that weren't obvious during initial implementation. Deviation Rule 1 (auto-fix bugs) covers this correctly.

## User Setup Required

None - no external service configuration required.

## Verification Results

All verification scenarios passed (Task 2 checkpoint approved by user):

1. ✅ **Small data (50 rows):** Table appears instantly. No loading indicator (too fast to see).

2. ✅ **Large data (500k rows):** Table appears within 1 second. Loading indicator shows "Loading... X rows" updating every ~250ms. Row count increases progressively. After loading completes, shows "Loaded 500000 rows" briefly then clears. Remains responsive after load complete.

3. ✅ **Navigation during load:** Arrow keys, j/k, /, l/h all responsive while 500k dataset loading. No frame drops or lag.

4. ✅ **Ctrl+C cancellation:** First Ctrl+C stops background parsing, shows "Loading cancelled", app remains usable with partial data. Can scroll through partial dataset. Second 'q' exits cleanly.

5. ✅ **Normal quit:** 'q' exits cleanly after any test. No hanging threads or crashes.

## Success Criteria

All success criteria met:

- ✅ LOAD-01: First rows visible within 1 second of piping data (even for 500k+ rows)
- ✅ LOAD-02: Loading indicator shows "Loading... X rows", updates every ~250ms
- ✅ LOAD-03: Navigation, search, column controls work while loading continues
- ✅ LOAD-04: Ctrl+C cancels loading without crash, partial data remains browsable
- ✅ All existing features (export, tabs, split view, search) work correctly after streaming completes
- ✅ No regressions in database connection mode (--connect flag still works identically)

## Technical Notes

**Event loop integration:**
- StreamingParser created in stdin mode initialization
- Optional `streaming_loader` field added to main loop state
- Poll section BEFORE terminal.draw() calls `try_recv_batch(5000)`
- New rows appended to workspace.tabs[0].data.rows
- Loading state tracked with AppMode::Loading

**Viewport windowing details:**
- Window centered on current scroll position (scroll_offset)
- VISIBLE_WINDOW_SIZE = 1000 rows (fits most terminals)
- BUFFER_ZONE = 9000 rows (captures off-screen content for smooth scrolling)
- Total window = 10,000 rows per width calculation
- Slicing done before width calculation to minimize cost

**Loading indicator lifecycle:**
1. Streaming starts → AppMode::Loading set
2. Each frame: check `streaming_loader.is_complete()`
3. While loading: status shows "Loading... N rows" (N from workspace row count)
4. When complete: AppMode cleared, status shows "Loaded N rows" for 3 seconds
5. Status auto-clears after timer expires

**Cancellation behavior:**
- First Ctrl+C: calls `streaming_loader.cancel()`, drops loader, shows "Loading cancelled"
- App continues running with partial data fully browsable
- Second Ctrl+C or 'q': normal quit (streaming_loader is None)

**Performance characteristics:**
- Initial display: <1 second (headers + first batch of rows)
- Frame time: O(window_size) for width calculation, not O(total_rows)
- 500k rows: Smooth scrolling after load complete
- Memory: Grows linearly with row count (addressed in Phase 16)

## Next Steps

**Phase 16: Memory Optimization**
- Replace Vec<Vec<String>> with compact storage (string interning or CompactString)
- Profile memory usage with 1.8M row dataset using dhat
- Tune batch size and window size based on memory characteristics
- Consider bounded channel if memory pressure becomes issue

**Ready for Phase 16:**
- StreamingParser provides integration point for compact storage
- Viewport windowing limits memory impact of width calculation
- Integration tests (Phase 14) protect against regressions during storage refactor

**Technical debt:**
- Batch size (5000) and window size (10,000) are hardcoded constants - could be configurable
- Unbounded channel may cause memory pressure with extremely fast generators (addressed in Phase 16)

## Impact

**Enables:**
- Immediate data display for large datasets (1.8M+ rows)
- Interactive browsing during data load
- Graceful cancellation of long-running loads
- Performance at scale via viewport windowing

**User experience improvements:**
- No more frozen UI during large data loads
- Real-time progress feedback
- Can start navigating/searching before load completes
- Cancel without losing partial data

**Architectural patterns established:**
- Streaming event loop integration
- Viewport-windowed rendering for O(1) frame cost
- Progressive loading with incremental updates
- Graceful cancellation pattern (cancel but keep app running)

## Self-Check: PASSED

**Commits verified:**
- ✅ FOUND: 951c910 (Task 1 - streaming integration)
- ✅ FOUND: 059b9be (Performance fix - viewport windowing)
- ✅ FOUND: fa00010 (Bug fix - loading indicator completion)

**Files modified verified:**
```bash
git diff 951c910^..fa00010 --stat
```
- ✅ src/main.rs modified (streaming integration)
- ✅ src/render.rs modified (viewport windowing + loading indicator)
- ✅ src/workspace.rs modified (window size constants)
- ✅ src/state.rs modified (AppMode::Loading)
- ✅ benches/rendering.rs modified (constant references)
- ✅ benches/scrolling.rs modified (constant references)
- ✅ tests/search_tests.rs modified (test constants)

**Key features present:**
- ✅ StreamingParser::from_stdin() called in stdin mode
- ✅ streaming_loader polled in event loop
- ✅ Loading indicator rendering
- ✅ Viewport-windowed width calculation
- ✅ Ctrl+C cancellation handler

All artifacts verified on disk and in git history.

---

## Commits

1. **951c910** - feat(15-02): integrate streaming parser into main event loop
   - Replaces blocking stdin read with StreamingParser::from_stdin()
   - Polls streaming loader with try_recv_batch(5000) in event loop
   - Loading indicator shows "Loading... N rows" during streaming
   - Ctrl+C cancellation stops loading but keeps app running

2. **059b9be** - perf(15-02): viewport-windowed rendering for streaming performance
   - Calculates column widths only for visible rows + buffer (10k window)
   - Prevents O(n) performance degradation with large datasets
   - Window centered on scroll position, updates as user scrolls
   - Critical for 500k+ row responsiveness

3. **fa00010** - fix(15-02): loading indicator completion and row count accuracy
   - Loading indicator clears properly when streaming completes
   - Row count accurate during streaming (uses workspace row count)
   - Completion message shows final count: "Loaded N rows"
   - AppMode::Loading tracks loading state

**Duration:** 75 minutes (1h 15m)
**Tasks:** 2 (1 auto execution + 1 human verification checkpoint)
**Verification:** All 5 scenarios passed
**Files:** 7 modified (0 created)
**Tests:** All existing tests pass, no regressions

---
*Phase: 15-streaming-load*
*Plan: 02*
*Completed: 2026-02-10*
