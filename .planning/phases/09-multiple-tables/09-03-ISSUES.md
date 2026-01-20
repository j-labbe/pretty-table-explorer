# UAT Issues: Phase 9 Plan 3

**Tested:** 2026-01-20 (re-test after 09-03-FIX2)
**Source:** .planning/phases/09-multiple-tables/09-03-FIX2-SUMMARY.md
**Tester:** User via /gsd:verify-work

## Open Issues

None.

## Resolved Issues

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
*Plan: 03-FIX3*
*Re-tested: 2026-01-20*
