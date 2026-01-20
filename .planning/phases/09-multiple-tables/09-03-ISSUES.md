# UAT Issues: Phase 9 Plan 3

**Tested:** 2026-01-16
**Source:** .planning/phases/09-multiple-tables/09-03-SUMMARY.md
**Tester:** User via manual testing

## Open Issues

### UAT-001: Tab switching not working

**Discovered:** 2026-01-16
**Phase/Plan:** 09-03
**Severity:** Major
**Feature:** Multi-tab navigation
**Description:** Cannot swap between tabs. Tab/Shift+Tab or number keys don't switch the active tab.
**Expected:** Tab/Shift+Tab should cycle through tabs, number keys (1-9) should switch directly to that tab number.
**Actual:** Tab switching keybindings don't respond or don't change the active tab.
**Repro:**
1. Open multiple tables (create multiple tabs)
2. Try pressing Tab or Shift+Tab to cycle tabs
3. Try pressing number keys (1, 2, etc.) to switch tabs
4. Observe that tabs don't switch

### UAT-002: Split view right pane cannot be changed

**Discovered:** 2026-01-16
**Phase/Plan:** 09-03
**Severity:** Major
**Feature:** Split view pane control
**Description:** When split view is open, cannot change which tab is displayed in the right (2nd) pane.
**Expected:** Should be able to cycle the right pane's tab using Tab/Shift+Tab when right pane is focused.
**Actual:** Right pane tab cannot be changed.
**Repro:**
1. Open at least 2 tabs
2. Press V to enable split view
3. Press Ctrl+W to focus the right pane
4. Try to change which tab is shown in the right pane
5. Observe that it doesn't change

### UAT-003: Table selection broken after navigating back in split view

**Discovered:** 2026-01-16
**Phase/Plan:** 09-03
**Severity:** Major
**Feature:** Table selection in split view
**Description:** When in split view and the left (1st) pane navigates back to the table selection screen (via Esc), the table selector doesn't work - cannot press Enter to select a table.
**Expected:** Enter key should select the highlighted table and load it.
**Actual:** Enter key doesn't respond on table selection screen when in split view.
**Repro:**
1. Connect to database, see table list
2. Select a table (Enter)
3. Open another tab with a query
4. Press V for split view
5. Focus left pane (Ctrl+W if needed)
6. Press Esc to go back to table list
7. Try to select a table with Enter
8. Observe that Enter doesn't work

## Resolved Issues

None

---

*Phase: 09-multiple-tables*
*Plan: 03*
*Tested: 2026-01-16*
