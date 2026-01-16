# Plan 07-04 Summary: Horizontal Table Scrolling

## Objective
Implement horizontal scrolling for tables wider than the terminal, allowing column widths to be respected without compression.

## Tasks Completed

### Task 1: Add scroll state tracking
- Added `scroll_col_offset: usize` to track first visible column index
- Added `selected_visible_col: usize` to track global selected column position
- Added `last_visible_col_idx: StdCell<usize>` for scroll-right detection across frames
- Reset all scroll state when table data changes

### Task 2: Calculate visible column range for rendering
- Calculate available width (table_area.width - borders - highlight_symbol)
- Starting from scroll_col_offset, accumulate column widths until they exceed available space
- Always include at least one column even if it exceeds viewport
- Track `has_left_overflow` and `has_right_overflow` states

### Task 3: Update rendering to use scroll window
- Modified header_cells to only include columns in render range
- Modified data_rows to only include columns in render range
- Modified visible_widths to only include columns in render range
- Sync table_state.select_column() with render-relative position

### Task 4: Add overflow indicators
- Show "◀" in title when scroll_col_offset > 0
- Show "▶" in title when more columns exist after visible range
- Format: "◀ TableName ▶" when both overflows present

### Task 5: Auto-scroll on column navigation
- Pressing h/left: scroll left if selected_visible_col < scroll_col_offset
- Pressing l/right: scroll right if selected_visible_col > last_visible_col_idx
- Selection always stays visible during navigation

### Task 6: Update all key handlers
- Updated +/- to use selected_visible_col instead of table_state.selected_column()
- Updated H (hide) to use selected_visible_col
- Updated </> (reorder) to use selected_visible_col and handle scroll adjustment
- Title shows global column position "Col X/Y" based on selected_visible_col

## Files Modified
- `src/main.rs` - Scroll state, viewport calculation, rendering updates, key handler updates

## Verification Results
- `cargo build --release`: Success
- `cargo test`: 18 tests passed
- Horizontal scrolling works with h/l navigation
- Overflow indicators (◀ ▶) display correctly
- Column width adjustment (+/-) now visually works
- Hide/show/reorder continue to work with scroll

## Known Issues
None - UAT-004 was fixed by capping base width to 100 when auto_width exceeds max.

## Notes
- This was added during UAT to fix UAT-001/UAT-002 (blocker issues where +/-/H didn't visually work due to ratatui column compression)
- The root cause was ratatui compressing all columns to fit terminal width, masking width changes
- Solution: Render only columns that fit at their actual widths, enable horizontal scrolling for the rest
