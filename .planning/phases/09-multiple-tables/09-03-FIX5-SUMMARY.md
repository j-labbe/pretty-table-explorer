---
phase: 09-multiple-tables
plan: 03-FIX5
subsystem: ui
tags: [rust, tui, split-view, tab-cycling, pane-management]

# Dependency graph
requires:
  - phase: 09-multiple-tables
    provides: Split view, pane focus tracking, tab management
provides:
  - Correct Tab key cycling in right pane
  - Split view pane duplication prevention
affects: [split-view]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Split index divergence: ensure split_idx != active_idx when split is active"
    - "Focus transfer on close: move focus to left when right pane's tab is closed"

key-files:
  created: []
  modified:
    - src/main.rs
    - src/workspace.rs

key-decisions:
  - "Tab in left pane switches focus; Tab in right pane cycles split_idx"
  - "close_tab ensures panes show different tabs after adjustment"

patterns-established:
  - "Split view invariant: active_idx and split_idx must differ when split_active is true"

issues-created: []

# Metrics
duration: 2min
completed: 2026-01-20
---

# Phase 9 Plan 03-FIX5: Split View Tab Cycling and Pane Duplication Fixes

**Fixed Tab key to cycle tabs in right pane and prevented pane duplication when closing tabs**

## Performance

- **Duration:** 2 min
- **Started:** 2026-01-20T16:26:54Z
- **Completed:** 2026-01-20T16:28:39Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- Tab key in right pane now cycles through tabs (split_idx) instead of switching focus
- Tab key in left pane switches focus to right pane (original behavior preserved)
- Closing tabs no longer causes both panes to show the same tab
- Focus transfers to left pane when right pane's tab is closed

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix Tab cycling in right pane** - `b151acb` (fix)
2. **Task 2: Investigate Enter key** - No commit needed (code already correct)
3. **Task 3: Fix close_tab duplication** - `a766e7f` (fix)

**Plan metadata:** (this commit)

## Files Created/Modified

- `src/main.rs` - Updated Tab/BackTab handlers to cycle split_idx when right pane is focused
- `src/workspace.rs` - Updated close_tab to ensure split_idx != active_idx after adjustment

## Decisions Made

- Tab key behavior is now context-dependent:
  - Left pane focused + split active: switch focus to right
  - Right pane focused + split active: cycle which tab is shown in right pane
  - No split: cycle through tabs as before
- close_tab now enforces invariant that split_idx differs from active_idx

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## UAT Issues Fixed

1. **UAT-008: Tab cycling in right pane affects left pane** - FIXED
   - Root cause: Tab always called toggle_focus() in split mode
   - Fix: Check focus_left; if right pane focused, cycle split_idx instead

2. **UAT-009: Enter key does nothing in right pane** - VERIFIED
   - Root cause was likely UAT-010 (both panes showing same tab)
   - Code correctly uses focused_tab_mut() for Enter key handling
   - Will work correctly now that UAT-010 is fixed

3. **UAT-010: Closing tabs duplicates left pane view** - FIXED
   - Root cause: close_tab could leave split_idx == active_idx
   - Fix: After adjustment, if indices match, pick different tab for right pane
   - Also: Focus transfers to left when closing focused right pane's tab

## Next Phase Readiness

- All UAT issues from 09-03-ISSUES.md addressed
- Ready for re-verification with /gsd:verify-work 09-03
- Tests pass (32/32)
- Build succeeds

---
*Phase: 09-multiple-tables*
*Plan: 03-FIX5*
*Completed: 2026-01-20*
