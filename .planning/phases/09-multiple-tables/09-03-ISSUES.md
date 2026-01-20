# UAT Issues: Phase 9 Plan 3

**Tested:** 2026-01-20 (final verification after 09-03-FIX6)
**Source:** .planning/phases/09-multiple-tables/09-03-FIX6-SUMMARY.md
**Tester:** User via /gsd:verify-work
**Result:** ALL PASS (9/9 tests)

## Open Issues

None.

## Final Verification (2026-01-20)

All 9 tests passed:
- Pre-flight, focus indicator, Tab toggle, Tab simplification (FIX6)
- Navigation, Enter in right pane, tab close, UI elements, exit split view

Feature validated and ready for release.

## Resolved Issues

### UAT-011: Tab key should only switch pane focus, not cycle tabs

**Discovered:** 2026-01-20
**Resolved:** 2026-01-20 - Fixed in 09-03-FIX6
**Commit:** 607d04f
**Description:** Tab key now only toggles focus between panes in split view.
**Verification:** Tab no longer cycles tabs when in right pane.

### UAT-009: Enter key does nothing in right pane

**Discovered:** 2026-01-20
**Resolved:** 2026-01-20 - Verified in 09-03-FIX5
**Commit:** a766e7f
**Description:** Enter key now opens tables in the focused right pane correctly.
**Verification:** User confirmed Enter works in right pane.

### UAT-010: Closing all tabs in right pane duplicates left pane view

**Discovered:** 2026-01-20
**Resolved:** 2026-01-20 - Fixed in 09-03-FIX5
**Commit:** a766e7f
**Description:** Closing tabs no longer causes pane duplication; focus transfers correctly.
**Verification:** User confirmed panes show different content after tab close.

### UAT-008: Tab cycling in right pane affects left pane instead

**Discovered:** 2026-01-20
**Resolved:** 2026-01-20 - Fixed in 09-03-FIX5, but behavior change requested (see UAT-011)
**Commit:** b151acb
**Description:** Tab now cycles tabs in right pane correctly, but user wants this behavior removed entirely.
**Note:** Superseded by UAT-011 - user wants Tab to only switch focus, not cycle tabs.

### UAT-007: Enter key opens table in wrong pane in split view

**Discovered:** 2026-01-20
**Resolved:** 2026-01-20 - Fixed in 09-03-FIX4
**Commit:** b32dde8
**Description:** Enter key now opens tables in the focused pane (left or right).
**Verification:** Pending action processing now checks focus_left before deciding which pane index to update.

### UAT-006: Tab bar and controls hint missing in split view

**Discovered:** 2026-01-20
**Resolved:** 2026-01-20 - Fixed in 09-03-FIX3
**Commit:** 01a8906
**Description:** Tab bar and controls hint now visible in split view.
**Verification:** Vertical layout wrapper adds header (tab bar) and footer (controls) around panes.

### UAT-004: Split view pane switching not working

**Discovered:** 2026-01-20
**Resolved:** 2026-01-20 - Fixed in 09-03-FIX2
**Commit:** c0e797f
**Description:** Tab key now correctly switches focus between panes in split view.
**Verification:** User confirmed Tab switches panes, yellow border moves correctly.

### UAT-005: Horizontal scrolling not working in split view

**Discovered:** 2026-01-20
**Resolved:** 2026-01-20 - Fixed in 09-03-FIX2
**Commit:** 2482348
**Description:** h/l keys now work for horizontal scrolling in split view.
**Verification:** User confirmed navigation (j/k/h/l) works in both panes.

### UAT-001: Tab switching not working

**Discovered:** 2026-01-16
**Resolved:** 2026-01-20 - Fixed in 09-03-FIX
**Commit:** f32a9dc
**Description:** Tab/Shift+Tab and number keys now correctly switch between tabs.
**Verification:** User confirmed tabs work as expected in non-split view.

### UAT-002: Split view right pane cannot be changed

**Discovered:** 2026-01-16
**Status:** Partially addressed in 09-03-FIX, but new issue UAT-004 identified
**Commit:** f32a9dc
**Note:** Per-tab view mode was implemented, but pane focus switching mechanism still has issues.

### UAT-003: Table selection broken after navigating back in split view

**Discovered:** 2026-01-16
**Status:** Cannot fully verify due to UAT-004 blocking pane focus
**Commit:** f32a9dc
**Note:** Enter key works correctly in non-split view. Split view testing blocked by UAT-004.

---

*Phase: 09-multiple-tables*
*Plan: 03-FIX5*
*Re-tested: 2026-01-20*
