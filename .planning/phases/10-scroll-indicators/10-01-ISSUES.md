# UAT Issues: Phase 10 Plan 1

**Tested:** 2026-01-16
**Source:** .planning/phases/10-scroll-indicators/10-01-SUMMARY.md
**Tester:** User via /gsd:verify-work

## Open Issues

None

## Resolved Issues

### UAT-005: Wide column navigation causes incremental "inching" scroll behavior

**Discovered:** 2026-01-16
**Resolved:** 2026-01-16 - Fixed in 10-01-FIX3.md
**Commit:** 01aa2d2
**Phase/Plan:** 10-01
**Severity:** Major
**Feature:** Column navigation with wide/truncated columns
**Description:** When navigating past a wide column (one that is partially displayed/truncated), the navigation behavior is sluggish. Instead of immediately selecting the next column, the table scrolls incrementally multiple times while keeping the wide column selected, "inching" past it before finally selecting the next column. This causes a noticeable visual delay and gives users the impression that column navigation is slow or unresponsive.
**Expected:** Pressing right arrow on a wide column should immediately select the next column to the right, scrolling the table as needed in one step.
**Actual:** Pressing right arrow triggers multiple incremental scrolls while the wide column remains selected, eventually selecting the next column after several key presses or a delay.
**Resolution:** Replaced incremental while loop with direct assignment (`scroll_col_offset = selected_visible_col`) for immediate scroll behavior.

### UAT-004: Right indicator not fixed to viewport edge with wide columns

**Discovered:** 2026-01-16
**Resolved:** 2026-01-16 - Fixed in 10-01-FIX2.md
**Commit:** e061fe7
**Phase/Plan:** 10-01
**Severity:** Major
**Feature:** Right scroll indicator (▶)
**Description:** The right scroll indicator aligns with the right side of the rightmost visible column instead of being fixed to the viewport's right border. When the next off-screen column is very wide (wider than remaining viewport space), only one column may be visible with blank space between it and the right indicator. Additionally, columns that are too wide to fit don't display at all, leaving blank areas.
**Resolution:** Used `Constraint::Fill(1)` for last data column to push indicator to edge. Added partial column content display for wide columns that don't fully fit.

### UAT-001: Left scroll indicator overlaps table data

**Discovered:** 2026-01-16
**Resolved:** 2026-01-16 - Fixed in 10-01-FIX.md
**Commit:** 3a258f0
**Phase/Plan:** 10-01
**Severity:** Major
**Feature:** Left scroll indicator (◀)
**Description:** The left scroll indicator overlaps with table data instead of being in its own dedicated column
**Resolution:** Fixed width reservation to +2 chars (1 for indicator + 1 for separator)

### UAT-002: Right scroll indicator position wrong

**Discovered:** 2026-01-16
**Resolved:** 2026-01-16 - Fixed in 10-01-FIX.md
**Commit:** 3a258f0
**Phase/Plan:** 10-01
**Severity:** Major
**Feature:** Right scroll indicator (▶)
**Description:** The right scroll indicator appears in the wrong position
**Resolution:** Fixed width reservation for right indicator

### UAT-003: Column selection position offset when indicators visible

**Discovered:** 2026-01-16
**Resolved:** 2026-01-16 - Fixed in 10-01-FIX.md
**Commit:** 82f6565
**Phase/Plan:** 10-01
**Severity:** Major
**Feature:** Column selection with indicators
**Description:** Column selection position is off when scroll indicators are visible
**Resolution:** Added safety clamp to ensure selection stays on data columns

---

*Phase: 10-scroll-indicators*
*Plan: 01*
*Tested: 2026-01-16*
