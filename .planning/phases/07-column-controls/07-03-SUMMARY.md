# Plan 07-03 Summary: Column Reordering Functionality

## Objective
Add column reordering functionality to move columns left/right.

## Tasks Completed

### Task 1: Add display order tracking to ColumnConfig
- Added `display_order` field to ColumnConfig struct (Vec<usize> of column indices)
- Updated `new()` to initialize display_order as 0..num_columns
- Updated `reset()` to restore display_order to original sequential order
- Modified `visible_indices()` to return columns in display_order, filtered by visibility
- Added `display_position(col_idx)` method to get position of a column in display order
- Added `swap_display(pos1, pos2)` method to swap two positions in display order

### Task 2: Add reorder keybindings
- Added `<` and `,` keys to move selected column left (swap with previous visible column)
- Added `>` and `.` keys to move selected column right (swap with next visible column)
- Selection follows the moved column (select_previous/next_column after swap)
- Reorder respects visibility - only visible columns are considered for swapping
- Widths recalculated after each reorder operation

### Task 3: Update title controls for column management
- Updated TableData controls: "+/-/0: width, H/S: hide/show, </>: move, Esc: back, q: quit"
- Updated PipeData controls: "+/-/0: width, H/S: hide/show, </>: move, q: quit"
- TableList controls remain simple: "Enter: select, /: filter, q: quit"
- Controls now clearly show all column manipulation options

## Files Modified
- `src/column.rs` - Added display_order field and reorder methods
- `src/main.rs` - Added </>/, keybindings for column reordering, updated title controls

## Verification Results
- `cargo build --release`: Success (1 warning for unused `is_visible` method - carried over from 07-02)
- `cargo test`: 18 tests passed
- `<` key moves selected column left
- `>` key moves selected column right
- Selection follows the moved column
- Cannot move leftmost column left or rightmost right (bounds checking)
- Reorder persists during navigation
- `0` key resets column order (along with widths and visibility)
- All three features work together: resize (+/-) + hide/show (H/S) + reorder (</>)

## Commits
1. `feat(07-03): add display order tracking to ColumnConfig` - 080bea2
2. `feat(07-03): add reorder keybindings` - bff7c73
3. `feat(07-03): update title controls for column management` - 97f21fc

## Notes
- The reorder implementation swaps columns in the full display_order array, but the keybindings work with visible column positions
- This allows hidden columns to maintain their relative position when shown again
- The `is_visible` method warning remains (from 07-02) - method is available for future use but currently unused
- Phase 07 (Column Controls) is now complete with all three plans:
  - 07-01: Column width resizing (+/-/0)
  - 07-02: Column hide/show (H/S)
  - 07-03: Column reordering (</>)
