# Plan 07-02 Summary: Column Hide/Show Functionality

## Objective
Add column hide/show functionality to toggle column visibility.

## Tasks Completed

### Task 1: Add visibility toggle methods to ColumnConfig
- Added `hide(col)` method to hide a specific column
- Added `show_all()` method to restore visibility to all columns
- Added `visible_count()` method to count visible columns
- Added `visible_indices()` method to get ordered list of visible column indices
- Updated `reset()` to also show all hidden columns (sets visible = true for all)

### Task 2: Update rendering to skip hidden columns
- Capture `visible_cols`, `visible_count`, and `hidden_count` before draw closure
- Update title col_info to show "(X hidden)" when columns are hidden
- Create header row using only visible columns via `visible_cols.iter()`
- Create data rows using only visible columns via `visible_cols.iter()`
- Filter widths to `visible_widths` vector containing only visible column widths

### Task 3: Add hide/show keybindings
- `H` key hides selected column (uppercase to avoid conflict with `h`/left navigation)
  - Prevents hiding last visible column (requires `visible_count() > 1`)
  - Adjusts column selection if hidden column was rightmost visible
- `S` key shows all hidden columns
- Updated title controls in TableData mode: "Esc: back, /: filter, :: query, +/-: resize, H: hide, S: show all, q: quit"
- Updated title controls in PipeData mode: "/: filter, +/-: resize, H: hide, S: show all, q: quit"

## Files Modified
- `src/column.rs` - Added hide(), show_all(), visible_count(), visible_indices() methods; updated reset()
- `src/main.rs` - Rendering updates for visible columns; H/S keybindings; updated title controls

## Verification Results
- `cargo build --release`: Success (1 warning for unused `is_visible` method - expected)
- `cargo test`: 18 tests passed
- H key hides currently selected column
- S key shows all hidden columns
- Cannot hide the last visible column
- Title shows "(X hidden)" when columns are hidden
- Navigation h/l works correctly with hidden columns
- Reset (0) also shows hidden columns

## Commits
1. `feat(07-02): add visibility toggle methods to ColumnConfig` - fa249e0
2. `feat(07-02): update rendering to skip hidden columns` - 730995c
3. `feat(07-02): add hide/show keybindings` - 7eb3c66

## Notes
- The `is_visible(col)` method from 07-01 generates a dead code warning since visibility is checked via `visible_indices()` instead of per-column checks
- Column selection index refers to the original data column index, not the visual position
- When hiding columns, width recalculation is triggered to update layout
