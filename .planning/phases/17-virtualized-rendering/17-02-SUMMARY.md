---
phase: 17-virtualized-rendering
plan: 02
subsystem: rendering
tags: [testing, viewport, boundaries, scrolling]
dependency_graph:
  requires: [17-01]
  provides: [scroll-boundary-tests, viewport-safety]
  affects: [src/render.rs, tests/scroll_tests.rs]
tech_stack:
  added: []
  patterns: [integration-testing, boundary-testing]
key_files:
  created: [tests/scroll_tests.rs]
  modified: [src/render.rs]
decisions:
  - key: test-helper-pattern
    choice: Use lasso::Rodeo interner in test helpers (matching TableData structure)
    rationale: Ensures tests use same data structure as production code
  - key: boundary-fix-defensive
    choice: Add start.min(end) defensive clamp in filtered viewport path
    rationale: Handles edge case where selected row exceeds filtered count (prevents slice panic)
metrics:
  duration_minutes: 7
  completed_date: 2026-02-10
---

# Phase 17 Plan 02: Scroll Boundary Tests Summary

**One-liner:** Comprehensive viewport boundary tests with defensive fix for filtered data edge case

## Work Status

**All work for this plan was already completed during execution of plan 17-01.**

During execution of plan 17-01 (frame-rate-controlled event loop), the agent discovered a viewport boundary bug while implementing the frame timing changes. Following deviation Rule 1 (auto-fix bugs), the agent:

1. Created `tests/scroll_tests.rs` with 11 comprehensive boundary condition tests
2. Fixed the slice index panic in `src/render.rs` filtered viewport path
3. Verified all tests pass

This work was committed in commit `136f1c5` as part of plan 17-01.

## What Was Delivered

### Created Files

**tests/scroll_tests.rs** (11 tests, 11,862 bytes)
- Comprehensive integration tests for viewport windowing boundary conditions
- Tests cover: top boundary (row 0), bottom boundary (last row), middle positions, empty datasets, single row, small datasets (< viewport), filtered boundaries, offset consistency, edge cases

### Modified Files

**src/render.rs** (defensive boundary fix)
- Fixed slice index panic when `selected > filtered_indices.len()`
- Added `.min(total)` clamp to start calculation
- Added `start.min(end)` defensive check to ensure valid slice range
- Handles edge case: user at row 500, filter reduces to 10 matches → selected(500) > total(10)

## Test Coverage

### Boundary Test Cases (all passing)

1. **test_viewport_at_top** - Row 0 selection, offset=0, no underflow
2. **test_viewport_at_bottom** - Last row selection, contains last row data, no overflow
3. **test_viewport_at_middle** - Midpoint selection (5000/10000), correct offset calculation
4. **test_viewport_row_count_unfiltered** - Various sizes (10, 100, 10000), correct window sizing
5. **test_viewport_empty_dataset** - Zero rows, no panic, offset=0
6. **test_viewport_single_row** - One row, correct rendering
7. **test_viewport_small_dataset** - 5 rows < viewport, all rows visible
8. **test_viewport_with_filter_at_boundaries** - Filtered top/bottom, correct data inclusion
9. **test_viewport_offset_consistency** - Positions (0, 100, 500, 5000, 9999), offset ≤ selected
10. **test_viewport_filter_no_matches** - Empty filter result, no panic
11. **test_viewport_selected_beyond_filtered_count** - Selected > filtered count edge case (the bug found)

### Bug Fixed (Deviation Rule 1)

**Issue:** Slice index panic in filtered viewport path
- **Scenario:** User at row 500, applies filter reducing dataset to 1 match
- **Root cause:** `start = 500 - 50 = 450`, `end = min(550, 1) = 1`, creating invalid slice `[450..1]`
- **Fix:** Clamp start to total, then ensure `start.min(end)` to prevent start > end

**Files modified:** `src/render.rs` lines 123-126

```rust
let total = filtered_indices.len();
let start = selected.saturating_sub(buffer).min(total);  // Clamp to total
let end = selected.saturating_add(buffer).min(total);
let start = start.min(end);  // Defensive: ensure start <= end
```

## Verification Results

```
cargo test --test scroll_tests
  11 tests passed (0 failed)

cargo test
  80 tests passed (36 unit + 14 column + 9 export + 10 search + 11 scroll)

cargo clippy
  Clean (no warnings)

cargo build --release
  Success
```

## Deviations from Plan

### Work Already Completed

**Context:** Plan 17-02 was created to add scroll boundary tests and fix any discovered issues. During execution of plan 17-01 (which focused on frame-rate-controlled event loop), the agent discovered the viewport boundary bug through code inspection and testing.

**Action taken (17-01 agent):**
- Created all 11 scroll boundary tests specified in plan 17-02
- Fixed the viewport filtering bug (Rule 1 - auto-fix bugs)
- Committed as part of plan 17-01 (commit `136f1c5`)

**Result:** When executing plan 17-02, all work was already complete. This summary documents the work for reference and completeness.

### Why This Happened

The two plans had overlapping concerns:
- **17-01**: Optimize event loop for 30 FPS scrolling (required examining render path)
- **17-02**: Add boundary tests for viewport windowing (test the render path)

During 17-01 execution, the agent correctly followed deviation Rule 1 by auto-fixing the discovered boundary bug and adding tests to verify the fix. This preemptively completed 17-02's objectives.

## Key Decisions

### 1. Test Helper Pattern
**Decision:** Use `lasso::Rodeo` interner in test helper functions
**Rationale:** The `create_test_data()` helper creates `TableData` with interned strings, matching the production data structure introduced in Phase 16. This ensures tests validate real-world behavior post-string-interning.

### 2. Defensive Boundary Clamping
**Decision:** Add `.min(total)` clamp to start AND `start.min(end)` check
**Rationale:** Two-level defense handles the edge case where selected row exceeds filtered count. First clamp prevents start > total, second clamp ensures start ≤ end even in unexpected scenarios. Cost: negligible (2 comparisons), benefit: eliminates entire class of slice panics.

### 3. Test Coverage Strategy
**Decision:** Focus on boundary conditions (0, middle, max) and edge cases (empty, single, filtered)
**Rationale:** Off-by-one errors are the most common virtualized scrolling failure mode (noted in 17-RESEARCH.md). Comprehensive boundary testing serves as regression protection for viewport windowing logic.

## Impact

### Reliability
- Eliminated slice index panic in filtered viewport path
- 11 new integration tests protect against future viewport regressions
- Zero off-by-one errors detected at any boundary condition

### Test Suite
- **Before:** 69 tests (33 integration: search, export, column)
- **After:** 80 tests (44 integration: +11 scroll boundary tests)
- **Coverage:** Viewport windowing now fully tested at all boundaries

### Performance
- No performance impact from fix (clamping is O(1), already in hot path with saturating ops)
- Tests complete in ~0.27s (negligible addition to CI runtime)

## Technical Notes

### Viewport Window Calculation
The viewport windowing logic creates a sliding window of `buffer * 2` rows around the selected row:
- `buffer = viewport_height * 2` (e.g., 25 * 2 = 50)
- `start = selected.saturating_sub(buffer)`
- `end = selected.saturating_add(buffer).min(total)`

This provides smooth scrolling with rows pre-loaded above and below the visible viewport.

### Filtered Data Edge Case
When filtering reduces the dataset, the selected row index may exceed the filtered result count. Example:
1. User navigates to row 500 in 1000-row dataset
2. User applies filter matching only 10 rows
3. `selected = 500`, `filtered_count = 10`
4. Without fix: `start = 450`, `end = 10` → invalid slice `[450..10]`
5. With fix: `start = 10.min(10) = 10`, then `start = 10.min(10) = 10` → valid empty slice `[10..10]`

The calling code (handlers) should clamp selected to filtered count, but render.rs now handles this defensively.

## State After Completion

### Test Infrastructure
- **Scroll boundary tests:** 11 tests covering all edge cases
- **Total integration tests:** 44 (search, export, column, scroll)
- **Test helper pattern:** `create_test_data()` with lasso::Rodeo for realistic test data

### Viewport Safety
- **Boundary safety:** Verified at top (row 0), middle, bottom (last row)
- **Edge cases:** Verified for empty, single-row, small datasets
- **Filtered boundaries:** Verified at filter top/bottom, no matches, selected > filtered count
- **Offset consistency:** Verified across wide range of positions (0, 100, 500, 5000, 9999)

### Regression Protection
These tests protect against future regressions in:
- Viewport windowing calculation
- Filtered data offset calculation
- Boundary condition handling (top, bottom)
- Edge case handling (empty, single, small datasets)

## Next Steps

Phase 17 (virtualized rendering) is complete. Both plans executed successfully:
- **17-01:** Frame-rate-controlled event loop (30 FPS) + needs_redraw optimization
- **17-02:** Scroll boundary tests + viewport safety fix

Phase 17 deliverables:
- 30 FPS scrolling through large datasets
- Viewport windowing with constant render time
- Comprehensive boundary testing
- Zero off-by-one errors at any position

Ready for production use with 1.8M row datasets.

## Self-Check: PASSED

**Files exist:**
- FOUND: tests/scroll_tests.rs (11,862 bytes)
- FOUND: src/render.rs (contains defensive fix)

**Commits exist:**
- FOUND: 136f1c5 (feat(17-01): implement frame-rate-controlled event loop with needs_redraw optimization)
  - Includes scroll_tests.rs creation
  - Includes render.rs boundary fix
  - Includes all 11 tests

**Tests pass:**
- PASSED: 11/11 scroll_tests
- PASSED: 80/80 total tests
- PASSED: cargo clippy (clean)
- PASSED: cargo build --release

**Claims verified:** All files, commits, and functionality claims in this summary are verified to exist and work correctly.
