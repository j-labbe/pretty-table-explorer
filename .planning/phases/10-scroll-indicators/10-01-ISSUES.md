# UAT Issues: Phase 10 Plan 1

**Tested:** 2026-01-16
**Source:** .planning/phases/10-scroll-indicators/10-01-SUMMARY.md
**Tester:** User via /gsd:verify-work

## Open Issues

### UAT-001: Left scroll indicator overlaps table data

**Discovered:** 2026-01-16
**Phase/Plan:** 10-01
**Severity:** Major
**Feature:** Left scroll indicator (◀)
**Description:** The left scroll indicator overlaps with table data instead of being in its own dedicated column
**Expected:** Left indicator should appear in a separate column that doesn't overlap any data
**Actual:** Indicator overlaps/covers table data content

### UAT-002: Right scroll indicator position wrong

**Discovered:** 2026-01-16
**Phase/Plan:** 10-01
**Severity:** Major
**Feature:** Right scroll indicator (▶)
**Description:** The right scroll indicator appears in the wrong position
**Expected:** Right indicator should appear at the rightmost edge in its own column
**Actual:** Indicator position is incorrect

### UAT-003: Column selection position offset when indicators visible

**Discovered:** 2026-01-16
**Phase/Plan:** 10-01
**Severity:** Major
**Feature:** Column selection with indicators
**Description:** Column selection position is off when scroll indicators are visible
**Expected:** Selection should correctly highlight the intended data column regardless of indicator presence
**Actual:** Selection position is offset/incorrect when indicators are displayed

## Resolved Issues

[None yet]

---

*Phase: 10-scroll-indicators*
*Plan: 01*
*Tested: 2026-01-16*
