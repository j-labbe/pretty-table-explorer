---
phase: 09-multiple-tables
plan: 03-FIX5
type: fix
---

<objective>
Fix 3 UAT issues from plan 09-03.

Source: 09-03-ISSUES.md
Priority: 0 critical, 3 major, 0 minor

Issues:
- UAT-008: Tab cycling in right pane affects left pane instead
- UAT-009: Enter key does nothing in right pane
- UAT-010: Closing all tabs in right pane duplicates left pane view
</objective>

<execution_context>
@./.claude/get-shit-done/workflows/execute-plan.md
@./.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@.planning/ROADMAP.md

**Issues being fixed:**
@.planning/phases/09-multiple-tables/09-03-ISSUES.md

**Original plan for reference:**
@.planning/phases/09-multiple-tables/09-03-PLAN.md

**Current implementation:**
@src/main.rs
@src/workspace.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Fix UAT-008 - Tab cycling changes wrong pane</name>
  <files>src/main.rs</files>
  <action>
The Tab key currently toggles focus between panes (line ~1315-1320). The original plan stated Tab/Shift+Tab in right pane should cycle that pane's displayed tab.

Fix the Tab key handler to:
1. In split view with focus on RIGHT pane: cycle split_idx through available tabs
2. In split view with focus on LEFT pane: toggle focus (current behavior for switching panes)
3. Not in split view: cycle through tabs as before

Update the KeyCode::Tab handler around line 1314:
```rust
KeyCode::Tab => {
    if workspace.split_active {
        if workspace.focus_left {
            // Left pane focused: Tab switches to right pane
            workspace.toggle_focus();
        } else {
            // Right pane focused: Tab cycles which tab is shown in right pane
            workspace.split_idx = (workspace.split_idx + 1) % workspace.tab_count();
        }
    } else if workspace.tab_count() > 1 {
        workspace.next_tab();
    }
}
```

Similarly for BackTab (Shift+Tab):
```rust
KeyCode::BackTab => {
    if workspace.split_active {
        if workspace.focus_left {
            // Left pane focused: Shift+Tab switches to right pane
            workspace.toggle_focus();
        } else {
            // Right pane focused: Shift+Tab cycles backwards
            if workspace.split_idx == 0 {
                workspace.split_idx = workspace.tab_count() - 1;
            } else {
                workspace.split_idx -= 1;
            }
        }
    } else if workspace.tab_count() > 1 {
        workspace.prev_tab();
    }
}
```
  </action>
  <verify>cargo check passes</verify>
  <done>Tab/Shift+Tab in right pane cycles split_idx, in left pane switches focus</done>
</task>

<task type="auto">
  <name>Task 2: Fix UAT-009 - Enter key does nothing in right pane TableList</name>
  <files>src/main.rs</files>
  <action>
Root cause: The right pane may show a TableList view, but there's a problem with how the tab is accessed OR how the pending action routes the new tab.

Looking at line 1050: `let tab = workspace.focused_tab_mut().unwrap();`

This should correctly get the right pane's tab when focus_left is false. However, there may be an issue where the right pane doesn't have a TableList view mode tab.

The real issue is likely that:
1. The right pane displays a tab that already has TableData view mode
2. When the user closes tabs in the right pane, it doesn't create a new TableList tab

The fix should ensure that when the right pane shows a TableList and user presses Enter:
1. The query is executed
2. The new tab is opened in the focused pane (right pane)

The pending action handling at line 1548-1558 already checks `workspace.split_active && !workspace.focus_left` to set `split_idx` instead of calling `switch_to()`. This looks correct.

Debug by adding verification that the tab.view_mode check at line 1063 is actually checking the focused tab. The `tab` variable is set at line 1050 using `focused_tab_mut()` which should be correct.

Actually, looking more carefully - the issue may be that the right pane NEVER has a TableList view mode because:
- Initial tab (Tables) is in the left pane (active_idx = 0)
- When split is enabled, split_idx defaults to the next tab after active
- If there's no TableList tab in split_idx position, Enter does nothing

The fix: Ensure that when the right pane is focused and showing a TableList tab, Enter works. This should already work based on the code. The issue might be that there's no easy way to GET a TableList tab in the right pane.

Actually, re-reading UAT-010: "closing all tabs on the right pane should show the table selector". This implies the user expects the right pane to show the TableList when it has no data tabs. Currently, the system requires at least 2 tabs for split view and there's only ever ONE TableList tab.

The real fix needed: Allow the right pane to show the same TableList tab as the left pane, OR create a concept of "empty pane shows TableList".

For now, verify the Enter key works when the right pane actually contains a TableList tab (e.g., if user cycles split_idx to point to tab 0 which is the TableList).

No code changes needed for this task - the issue is that the UI doesn't provide a way to get a TableList in the right pane. This is addressed in Task 3.
  </action>
  <verify>Manual verification: with 2+ tabs including Tables tab, set split_idx to TableList tab, Enter should work</verify>
  <done>Enter key confirmed to work when right pane contains TableList tab</done>
</task>

<task type="auto">
  <name>Task 3: Fix UAT-010 - Closing tabs in right pane causes duplication</name>
  <files>src/workspace.rs, src/main.rs</files>
  <action>
Root cause: When close_tab adjusts split_idx, if the closed tab was the one in the right pane, split_idx gets clamped to an existing tab. If split_idx equals active_idx, both panes show the same tab.

Current close_tab (line 131-154 in workspace.rs):
```rust
// Adjust split_idx
if self.split_idx >= self.tabs.len() {
    self.split_idx = self.tabs.len() - 1;
}
```

This can result in split_idx == active_idx, causing both panes to show the same content.

FIX in workspace.rs close_tab():
After adjusting split_idx, ensure it differs from active_idx when split is active:
```rust
// Ensure split_idx != active_idx when split is active
if self.split_active && self.tabs.len() > 1 && self.split_idx == self.active_idx {
    // Pick a different tab for the right pane
    self.split_idx = (self.active_idx + 1) % self.tabs.len();
}
```

Also, the problem of "actions affect both panes" happens because when split_idx == active_idx, both panes reference the same tab. The fix above should resolve this by ensuring they're always different when split is active.

Additionally, if focus is on the right pane and its tab is closed, move focus back to left pane:
```rust
// If we just closed the focused (right) pane's tab, move focus to left
if !self.focus_left && idx == old_split_idx {
    self.focus_left = true;
}
```

Add this logic to close_tab() - need to capture old_split_idx before modifications.
  </action>
  <verify>cargo check passes</verify>
  <done>Closing tabs in right pane no longer causes duplication, split_idx always differs from active_idx</done>
</task>

</tasks>

<verification>
Before declaring plan complete:
- [ ] All major issues fixed
- [ ] cargo build --release succeeds
- [ ] In split view, Tab key in right pane cycles split_idx
- [ ] In split view, Tab key in left pane switches focus to right
- [ ] Closing the right pane's tab doesn't duplicate the left pane
- [ ] split_idx and active_idx are always different when split is active
- [ ] Tests pass (32/32)
</verification>

<success_criteria>
- All UAT issues from 09-03-ISSUES.md addressed
- Tests pass
- Ready for re-verification
</success_criteria>

<output>
After completion, create `.planning/phases/09-multiple-tables/09-03-FIX5-SUMMARY.md`
</output>
