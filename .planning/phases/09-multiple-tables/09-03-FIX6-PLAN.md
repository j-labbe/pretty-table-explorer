---
phase: 09-multiple-tables
plan: 03-FIX6
type: fix
---

<objective>
Fix 1 UAT issue from plan 09-03.

Source: 09-03-ISSUES.md
Priority: 0 critical, 0 major, 1 minor
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

**Current Tab key handling in main.rs (lines ~1314-1326):**
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
</context>

<tasks>
<task type="auto">
  <name>Task 1: Simplify Tab key to only toggle pane focus</name>
  <files>src/main.rs</files>
  <action>
Modify the Tab key handler to:
1. In split view: ALWAYS toggle focus between panes (remove the tab cycling behavior in right pane)
2. In non-split view: Keep existing behavior (cycle through tabs)

Change from:
```rust
KeyCode::Tab => {
    if workspace.split_active {
        if workspace.focus_left {
            workspace.toggle_focus();
        } else {
            workspace.split_idx = (workspace.split_idx + 1) % workspace.tab_count();
        }
    } else if workspace.tab_count() > 1 {
        workspace.next_tab();
    }
}
```

To:
```rust
KeyCode::Tab => {
    if workspace.split_active {
        // In split view: Tab always toggles focus between panes
        workspace.toggle_focus();
    } else if workspace.tab_count() > 1 {
        workspace.next_tab();
    }
}
```

Also simplify BackTab similarly - in split view it should just toggle focus (same as Tab).
  </action>
  <verify>Build succeeds: cargo build</verify>
  <done>Tab key only toggles pane focus in split view, no longer cycles tabs in right pane</done>
</task>
</tasks>

<verification>
Before declaring plan complete:
- [ ] Tab key toggles focus between panes in split view
- [ ] Tab key cycles tabs in non-split view (unchanged)
- [ ] Build succeeds
- [ ] Tests pass
</verification>

<success_criteria>
- UAT-011 from 09-03-ISSUES.md addressed
- Tests pass
- Ready for re-verification
</success_criteria>

<output>
After completion, create `.planning/phases/09-multiple-tables/09-03-FIX6-SUMMARY.md`
</output>
