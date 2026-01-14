# UAT Issues: Phase 4 Plan 2

**Tested:** 2026-01-14
**Source:** .planning/phases/04-postgresql-integration/04-02-SUMMARY.md
**Tester:** User via /gsd:verify-work

## Open Issues

[None]

## Resolved Issues

### UAT-001: Error messages disappear too quickly

**Discovered:** 2026-01-14
**Phase/Plan:** 04-02
**Severity:** Minor
**Feature:** Query error handling
**Description:** When executing an invalid SQL query, the error message appears for only a split second before disappearing. Users may not have time to read the error message.
**Expected:** Error message should remain visible long enough to read (2-3 seconds) or persist until dismissed.
**Actual:** Error message flashes briefly and disappears after single render cycle.
**Repro:**
1. Connect to database with --connect
2. Press ':' to enter query mode
3. Type `SELECT * FROM nonexistent_table_xyz`
4. Press Enter
5. Error message appears and disappears almost immediately

**Resolved:** 2026-01-14 - Fixed in 04-02-FIX.md
**Commit:** 42d40e1
**Fix:** Added status_message_time timestamp; messages now persist for 3 seconds before auto-clearing

---

*Phase: 04-postgresql-integration*
*Plan: 02*
*Tested: 2026-01-14*
