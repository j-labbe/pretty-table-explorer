---
phase: 09-multiple-tables
plan: 03-FIX2
type: fix
---

<objective>
Fix 2 UAT issues from plan 09-03-FIX.

Source: 09-03-ISSUES.md
Priority: 0 critical, 2 major, 0 minor

Issues:
- UAT-004: Split view pane switching not working (UX clarification + keybinding improvement)
- UAT-005: Horizontal scrolling not working in split view (actual bug - missing scroll-right logic)
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

**Original plans for reference:**
@.planning/phases/09-multiple-tables/09-03-PLAN.md
@.planning/phases/09-multiple-tables/09-03-FIX-SUMMARY.md

**Key file:**
@src/main.rs (lines 695-751 - the scroll clamping logic)
</context>

<analysis>
**Root cause analysis:**

**UAT-004 (Pane switching):**
The user expected Tab key to switch focus between panes, but the actual keybindings are:
- Ctrl+W: Toggle focus between panes
- F6: Toggle focus between panes
- Tab: Cycles through tabs (in left pane when left focused, cycles right pane's tab when right focused)

The controls hint does show "Ctrl+W: switch pane" but the user may not have noticed.

**Fix approach for UAT-004:**
1. Make Tab key switch panes when in split view (in addition to Ctrl+W/F6)
   - In split view: Tab switches focus between panes
   - Tab/Shift+Tab for cycling tabs is less important when split since each pane shows a different tab
2. This provides more intuitive behavior that matches the user's expectation

**UAT-005 (Horizontal scrolling):**
The bug is in `src/main.rs` lines 695-751. The scroll-right logic that auto-scrolls the view when `selected_visible_col > last_visible_col_idx` only exists in the single-pane branch (lines 745-749), NOT in the split-view branch (lines 697-727).

When pressing `l`/Right in split view:
1. `selected_visible_col` increments correctly (line 1161)
2. But the view doesn't scroll because the scroll-right logic is missing
3. The column indicator shows Col 2/5, Col 3/5, etc. but the view stays on column 1

**Fix approach for UAT-005:**
Add the scroll-right logic to the split view branch for the focused pane.
</analysis>

<tasks>

<task type="auto">
  <name>Task 1: Fix horizontal scrolling in split view (UAT-005)</name>
  <files>src/main.rs</files>
  <action>
Add the scroll-right logic to the split view branch.

Find the split view clamping block (lines ~697-727) and add the scroll-right logic for the focused pane after clamping both panes:

```rust
if is_split {
    // Handle left pane (active tab)
    if let Some(tab) = workspace.tabs.get_mut(workspace.active_idx) {
        let visible_cols = tab.column_config.visible_indices();
        if !visible_cols.is_empty() {
            if tab.selected_visible_col >= visible_cols.len() {
                tab.selected_visible_col = visible_cols.len() - 1;
            }
            if tab.scroll_col_offset >= visible_cols.len() {
                tab.scroll_col_offset = visible_cols.len() - 1;
            }
            if tab.selected_visible_col < tab.scroll_col_offset {
                tab.scroll_col_offset = tab.selected_visible_col;
            }
            // Scroll right if selected column is beyond last visible (only if this is focused pane)
            if workspace.focus_left && tab.selected_visible_col > last_visible_col_idx.get() {
                tab.scroll_col_offset = tab.selected_visible_col.min(visible_cols.len() - 1);
            }
        }
    }
    // Handle right pane (split tab)
    if let Some(tab) = workspace.tabs.get_mut(workspace.split_idx) {
        let visible_cols = tab.column_config.visible_indices();
        if !visible_cols.is_empty() {
            if tab.selected_visible_col >= visible_cols.len() {
                tab.selected_visible_col = visible_cols.len() - 1;
            }
            if tab.scroll_col_offset >= visible_cols.len() {
                tab.scroll_col_offset = visible_cols.len() - 1;
            }
            if tab.selected_visible_col < tab.scroll_col_offset {
                tab.scroll_col_offset = tab.selected_visible_col;
            }
            // Scroll right if selected column is beyond last visible (only if this is focused pane)
            if !workspace.focus_left && tab.selected_visible_col > last_visible_col_idx.get() {
                tab.scroll_col_offset = tab.selected_visible_col.min(visible_cols.len() - 1);
            }
        }
    }
} else {
    // ... existing single pane logic ...
}
```

Key changes:
1. Add `if workspace.focus_left && ...` condition for left pane scroll-right
2. Add `if !workspace.focus_left && ...` condition for right pane scroll-right
3. Use the same scroll-right formula as single pane mode
  </action>
  <verify>cargo check passes</verify>
  <done>Split view branches have scroll-right logic for focused pane</done>
</task>

<task type="auto">
  <name>Task 2: Make Tab key switch panes in split view (UAT-004)</name>
  <files>src/main.rs</files>
  <action>
Modify the Tab key handler to switch pane focus when in split view.

Find the Tab key handler (around line 1288) and change the behavior:

Current behavior:
- Tab cycles through tabs (next_tab)
- In split mode with right focused, cycles right pane's tab

New behavior:
- In split mode: Tab toggles focus between panes (like Ctrl+W)
- In non-split mode: Tab cycles through tabs

```rust
KeyCode::Tab => {
    if workspace.split_active {
        // In split view, Tab switches focus between panes
        workspace.toggle_focus();
    } else if workspace.tab_count() > 1 {
        workspace.next_tab();
    }
}
KeyCode::BackTab => {
    if workspace.split_active {
        // Shift+Tab also switches focus in split view
        workspace.toggle_focus();
    } else if workspace.tab_count() > 1 {
        workspace.prev_tab();
    }
}
```

This makes the UX more intuitive:
- Tab/Shift+Tab to switch panes in split view
- Ctrl+W and F6 still work as alternative keybindings
- Number keys (1-9) still work for direct tab selection
  </action>
  <verify>cargo check passes</verify>
  <done>Tab key switches pane focus in split view</done>
</task>

<task type="auto">
  <name>Task 3: Update controls hint text</name>
  <files>src/main.rs</files>
  <action>
Update the controls hint to show Tab for pane switching in split view.

Find the split_controls string (around line 836-841):

```rust
let split_controls = if is_split {
    "V: unsplit, Ctrl+W: switch pane, "
} else if tab_count > 1 {
    "V: split, "
} else {
    ""
};
```

Change to:

```rust
let split_controls = if is_split {
    "Tab: switch pane, V: unsplit, "
} else if tab_count > 1 {
    "V: split, "
} else {
    ""
};
```

This makes Tab the primary keybinding shown (since it's more discoverable), and removes Ctrl+W from the hint since Tab is simpler.
  </action>
  <verify>cargo build --release succeeds</verify>
  <done>Controls hint shows Tab for pane switching</done>
</task>

</tasks>

<verification>
Before declaring plan complete:
- [ ] cargo build --release succeeds
- [ ] In split view, pressing `l`/Right scrolls the view horizontally when needed
- [ ] In split view, pressing `h`/Left scrolls the view horizontally when needed
- [ ] In split view, Tab key switches focus between panes
- [ ] In split view, Shift+Tab key also switches focus between panes
- [ ] Controls hint shows "Tab: switch pane" in split view
- [ ] Ctrl+W and F6 still work as alternative keybindings for pane switching
- [ ] In non-split mode, Tab still cycles through tabs
</verification>

<success_criteria>
- All UAT issues from 09-03-ISSUES.md addressed
- Tests pass (cargo test)
- Ready for re-verification
</success_criteria>

<output>
After completion, create `.planning/phases/09-multiple-tables/09-03-FIX2-SUMMARY.md`
</output>
