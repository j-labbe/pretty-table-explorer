---
phase: 09-multiple-tables
plan: 03-FIX4
type: fix
---

<objective>
Fix 1 UAT blocker issue from plan 09-03.

Source: 09-03-ISSUES.md
Priority: 1 blocker
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

**Key files:**
@src/main.rs (lines 1546-1553 - pending action processing)
@src/workspace.rs (Workspace struct and methods)
</context>

<tasks>
<task type="auto">
  <name>Fix Enter key to open tables in focused pane</name>
  <files>src/main.rs</files>
  <action>
The bug is in the pending action processing block (around line 1548-1552). When a table is opened via Enter key, the code always calls `workspace.switch_to(new_idx)` which updates `active_idx` (left pane).

In split view, if focus is on the right pane (`!workspace.focus_left`), the new tab should be opened in the right pane by updating `split_idx` instead of `active_idx`.

Fix the pending action processing:

```rust
// Process pending action (tab borrow has been dropped)
if let PendingAction::CreateTab { name, data, view_mode } = pending_action {
    let new_idx = workspace.add_tab(name, data, view_mode);
    // In split view with focus on right pane, open in right pane
    if workspace.split_active && !workspace.focus_left {
        workspace.split_idx = new_idx;
    } else {
        workspace.switch_to(new_idx);
    }
    status_message = Some(format!("Opened in tab {}", new_idx + 1));
    status_message_time = Some(Instant::now());
}
```

This ensures:
- Normal view: table opens in active tab (existing behavior)
- Split view, left pane focused: table opens in left pane (existing behavior)
- Split view, right pane focused: table opens in right pane (fixed behavior)
  </action>
  <verify>
1. `cargo build` succeeds
2. `cargo test` passes
3. Manual test:
   - Open app with multi-table data
   - Press 'v' to enter split view
   - Press Tab to focus right pane (yellow border moves right)
   - Navigate to a table and press Enter
   - Table should open in RIGHT pane (the focused one)
  </verify>
  <done>Enter key opens tables in the currently focused pane, regardless of which pane has focus in split view</done>
</task>
</tasks>

<verification>
Before declaring plan complete:
- [ ] Build succeeds without warnings
- [ ] All tests pass (32/32)
- [ ] Enter opens table in left pane when left pane has focus
- [ ] Enter opens table in right pane when right pane has focus
</verification>

<success_criteria>
- UAT-007 fixed
- Tables open in the focused pane
- Tests pass
- Ready for re-verification
</success_criteria>

<output>
After completion, create `.planning/phases/09-multiple-tables/09-03-FIX4-SUMMARY.md`
</output>
