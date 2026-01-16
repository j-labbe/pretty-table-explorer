# UAT Issues: Phase 7 Column Controls

**Tested:** 2026-01-15
**Source:** .planning/phases/07-column-controls/07-01-SUMMARY.md, 07-02-SUMMARY.md, 07-03-SUMMARY.md, 07-04-PLAN.md
**Tester:** User via /gsd:verify-work

## Open Issues

None.

## Resolved Issues

### UAT-004: Column shrinks to minimum when single column fills viewport
**Resolved:** 2026-01-15 - Fixed by capping base width to 100 when auto_width exceeds max
**Root cause:** When a column's auto_width exceeded the max (100), the first minus press would jump from e.g., 150 to 100 due to clamp. Fix: when no width_override is set, cap the starting point to 100 so the first adjustment decreases by the expected delta.

### UAT-001: Column width resize (+/-) only works once
**Resolved:** 2026-01-15 - Fixed by implementing horizontal scrolling (07-04)
**Root cause:** ratatui was compressing columns to fit terminal width, so stored width changes weren't visible. Horizontal scrolling renders columns at actual widths.

### UAT-002: Column hide (H) only works once
**Resolved:** 2026-01-15 - Fixed by implementing horizontal scrolling (07-04)
**Root cause:** Same as UAT-001 - column compression masked the changes. Also fixed index mapping to use selected_visible_col.

### UAT-003: Reset control should be displayed separately in title
**Resolved:** 2026-01-15 - Title now shows "0: reset" as separate control

---

*Phase: 07-column-controls*
*Tested: 2026-01-15*
