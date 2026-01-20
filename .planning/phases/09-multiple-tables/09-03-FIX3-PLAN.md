---
phase: 09-multiple-tables
plan: 03-FIX3
type: fix
---

<objective>
Fix 1 UAT issue from plan 09-03: Add tab bar and controls hint to split view.

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
</context>

<tasks>
<task type="auto">
  <name>Task 1: Add tab bar and controls to split view</name>
  <files>src/main.rs</files>
  <action>
In the split view rendering branch (around line 853), modify the layout to include:
1. A header area for tab bar (1 line)
2. The two split panes (horizontal split)
3. A footer area for controls hint (1 line)

Currently the split view uses the entire `table_area` for the two panes. Instead:
1. Create a vertical layout that splits `table_area` into:
   - Tab bar row (Length 1)
   - Panes area (Min 3)
   - Controls row (Length 1)

2. Render the `tab_bar` string in the tab bar row

3. Render the `controls` string in the controls row

4. Keep the existing horizontal split for the two panes in the middle area

The code currently discards these values (line ~906):
```rust
let _ = (tab_bar.clone(), status_info.clone(), controls);
```

Instead, render them:
- Tab bar at top showing open tabs
- Controls hint at bottom showing available keybindings
  </action>
  <verify>cargo build --release succeeds</verify>
  <done>Split view displays tab bar at top and controls hint at bottom, matching single-pane view layout</done>
</task>
</tasks>

<verification>
Before declaring plan complete:
- [ ] Split view shows tab bar listing open tabs
- [ ] Split view shows controls hint at bottom
- [ ] Tests pass (cargo test)
- [ ] Build succeeds (cargo build --release)
</verification>

<success_criteria>
- UAT-006 from 09-03-ISSUES.md addressed
- Split view has visual parity with single-pane view for tab bar and controls
- Tests pass
- Ready for re-verification
</success_criteria>

<output>
After completion, create `.planning/phases/09-multiple-tables/09-03-FIX3-SUMMARY.md`
</output>
