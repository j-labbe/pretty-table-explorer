# Phase 17: Virtualized Rendering - Research

**Researched:** 2026-02-10
**Domain:** Terminal UI rendering optimization, viewport windowing, scroll performance
**Confidence:** HIGH

## Summary

Phase 17 aims to ensure smooth scrolling (30+ FPS) through massive datasets (1.8M+ rows) by optimizing the existing viewport-windowed rendering system. The good news: **viewport windowing is already implemented** in `build_pane_render_data()` (Phase 15-02). The current implementation calculates a viewport window around the selected row (selected ± 2×viewport_height buffer) and only resolves/renders those rows.

The challenge is ensuring this system performs reliably across all scroll positions, handles boundary conditions correctly, and maintains stable frame rates. Research shows that ratatui's immediate-mode rendering achieves 60+ FPS when applications avoid full-dataset operations, use saturating arithmetic for bounds safety, and limit event polling appropriately.

**Primary recommendation:** Verify and tune the existing viewport windowing system rather than building new infrastructure. Focus on boundary testing (top/middle/bottom positions), frame rate measurement, and buffer size optimization.

## Standard Stack

### Core (Already in Use)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.29 | Terminal UI framework with Table widget | Industry standard for Rust TUIs, immediate-mode rendering achieves 60+ FPS |
| crossterm | 0.28 | Event polling with Duration control | Standard cross-platform terminal I/O, enables frame rate control via poll() |
| sysinfo | 0.33 | Memory tracking (already integrated) | Used in Phase 16-02 for RSS display, helps verify optimization impact |

### Supporting (Already in Use)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| lasso | 0.7 | String interning (Phase 16-01) | Already integrated, critical for memory efficiency with large datasets |
| criterion | 0.5 | Benchmarking (Phase 14-02) | Already configured, use for regression detection |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| ratatui Table | rat-ftable (community table widget) | rat-ftable uses TableData trait for O(screen-size) rendering, but requires architectural change. Current viewport windowing achieves same goal with less disruption. |
| Manual viewport | Third-party scrollable wrapper | Wrappers force full rendering before cropping (discussed in ratatui #174), defeating the performance goal. |

**Installation:**
No new dependencies needed. All required libraries already in Cargo.toml.

## Architecture Patterns

### Recommended Project Structure (Current)
```
src/
├── render.rs           # build_pane_render_data() - viewport windowing logic
├── main.rs             # Event loop, frame timing, TableState translation
├── workspace.rs        # Tab::update_cached_widths() - incremental width calculation
└── state.rs            # PaneRenderData with viewport_row_offset
```

### Pattern 1: Viewport Windowing (Already Implemented)
**What:** Calculate visible row window around selected position, only render those rows
**When to use:** Any table with >1000 rows
**Current implementation:**
```rust
// From src/render.rs build_pane_render_data()
let selected = tab.table_state.selected().unwrap_or(0);
let buffer = viewport_height.saturating_mul(2);  // 2x viewport = smooth scroll buffer

let start = selected.saturating_sub(buffer);
let end = selected.saturating_add(buffer).min(total);
let rows: Vec<Vec<String>> = tab.data.rows[start..end]
    .iter()
    .map(|row| {
        row.iter()
            .map(|s| tab.data.resolve(s).to_string())
            .collect()
    })
    .collect();
(rows, total, start)  // start becomes viewport_row_offset
```

**Why this works:** With viewport_height=50, buffer=100 rows. Window renders 200 rows regardless of total dataset size. String resolution happens only for visible window, not full 1.8M rows.

### Pattern 2: Absolute → Relative TableState Translation
**What:** Convert absolute row positions to viewport-relative before rendering, then translate back
**When to use:** When using viewport windowing with ratatui's Table widget
**Current implementation:**
```rust
// From src/main.rs
// Before render: translate absolute → relative
if left_viewport_offset > 0 {
    if let Some(sel) = left_table_state.selected() {
        left_table_state.select(Some(sel.saturating_sub(left_viewport_offset)));
    }
    *left_table_state.offset_mut() = left_table_state
        .offset()
        .saturating_sub(left_viewport_offset);
}

// ... render ...

// After render: translate relative → absolute
if left_viewport_offset > 0 {
    if let Some(sel) = left_table_state.selected() {
        left_table_state.select(Some(sel + left_viewport_offset));
    }
    *left_table_state.offset_mut() += left_viewport_offset;
}
```

**Why this is necessary:** Ratatui's Table widget expects row indices 0..display_rows.len(). Our TableState tracks absolute positions (0..1.8M). Translation layer makes them compatible.

### Pattern 3: Incremental Width Calculation (Already Implemented)
**What:** Cache column widths, only recalculate for newly added rows
**When to use:** Streaming data or any growing dataset
**Current implementation:**
```rust
// From src/workspace.rs Tab::update_cached_widths()
let start = self.widths_cached_for_rows;
let end = self.data.rows.len();
if start < end {
    for row in &self.data.rows[start..end] {
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                let w = (self.data.resolve(cell).len() + 1) as u16;
                if w > self.cached_auto_widths[i] {
                    self.cached_auto_widths[i] = w;
                }
            }
        }
    }
    self.widths_cached_for_rows = end;
}
```

**Why this matters:** With 1.8M rows, recalculating all widths every frame = 10-100ms. Incremental caching = O(new_rows) per frame, typically <1ms.

### Pattern 4: Frame Rate Control with Event Polling
**What:** Use crossterm's poll(Duration) to limit event loop frequency
**When to use:** Any interactive TUI application
**Current implementation:**
```rust
// From src/main.rs event loop
if event::poll(Duration::from_millis(250))? {
    if let Event::Key(key) = event::read()? {
        // Handle key event
    }
}
```

**Recommended optimization:**
```rust
// 30 FPS target = 33ms frame time
// Poll with shorter timeout for responsive feel, but skip redundant renders
const FRAME_TIME_MS: u64 = 33;  // ~30 FPS
let mut last_render = Instant::now();

loop {
    let now = Instant::now();
    let time_since_render = now.duration_since(last_render);

    // Poll for events with remaining frame time
    let poll_duration = FRAME_TIME_MS.saturating_sub(time_since_render.as_millis() as u64);
    if event::poll(Duration::from_millis(poll_duration))? {
        // Handle events, may set needs_redraw flag
    }

    // Render if frame time elapsed or event occurred
    if time_since_render.as_millis() >= FRAME_TIME_MS as u128 || needs_redraw {
        terminal.draw(|frame| { /* ... */ })?;
        last_render = now;
        needs_redraw = false;
    }
}
```

**Why this helps:** Current 250ms polling = 4 FPS max. Shorter poll + frame timing = consistent 30 FPS with responsive input.

### Anti-Patterns to Avoid

**❌ Full-Dataset Operations in Render Path:**
```rust
// WRONG: Resolves ALL 1.8M rows every frame
let all_rows: Vec<Vec<String>> = tab.data.rows
    .iter()
    .map(|row| row.iter().map(|s| tab.data.resolve(s).to_string()).collect())
    .collect();
```
**Why:** String resolution is ~100ns per cell. 1.8M rows × 10 cols × 100ns = 1.8 seconds per frame. Use viewport windowing instead.

**❌ Rebuilding ColumnConfig Every Frame:**
```rust
// WRONG: Recalculates visible indices on every frame
let visible = tab.column_config.visible_indices();  // If this allocates
```
**Why:** Allocation churn causes GC pauses. Cache the result if it doesn't change between frames.

**❌ Unchecked Arithmetic on Scroll Offsets:**
```rust
// WRONG: Can underflow with small datasets
let start = selected - buffer;  // Panics if selected < buffer
```
**Why:** Use `saturating_sub()` and `saturating_add()` for viewport bounds. See Pattern 1.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Scroll offset bounds checking | Custom if/else bounds logic | `saturating_sub()`, `saturating_add()`, `.min(total)` | Saturating arithmetic prevents underflow/overflow bugs. Widely used in Rust TUI code (ratatui examples, r3bl_tui). Manual bounds checking is error-prone. |
| Frame rate limiting | Custom sleep() loops or busy-wait | `crossterm::event::poll(Duration)` | Blocks until event OR timeout, preventing CPU spin. Integrates with event handling naturally. |
| Viewport-to-absolute translation | Custom offset tracking state | Existing pattern in main.rs (see Pattern 2) | Already implemented and tested. Encapsulates tricky offset math. |
| Column width calculation | Full-dataset scan every frame | Incremental caching (Pattern 3) | Already implemented in Tab::update_cached_widths(). O(new_rows) vs O(all_rows). |

**Key insight:** The hard parts are already solved. Phase 17's work is verification, tuning, and documentation rather than new infrastructure.

## Common Pitfalls

### Pitfall 1: Off-By-One Errors at Dataset Boundaries
**What goes wrong:** Scrolling to the last row shows blank cells or panics with "index out of bounds"
**Why it happens:** Viewport window calculation uses `end = selected + buffer`, which can exceed `total_rows`
**How to avoid:**
```rust
let end = selected.saturating_add(buffer).min(total);  // ✅ Clamps to total
```
**Warning signs:**
- Blank rows appear when scrolling to bottom
- Panic: "index X out of bounds for len Y"
- Selected row indicator disappears at boundaries

### Pitfall 2: TableState Offset Mismatch After Viewport Translation
**What goes wrong:** Scroll position jumps or "sticks" after navigating
**Why it happens:** Forgetting to translate TableState back to absolute coordinates after render
**How to avoid:** Always pair absolute→relative translation before render with relative→absolute after render (see Pattern 2)
**Warning signs:**
- Cursor jumps to wrong row after scrolling
- Scrolling stops working after reaching certain positions
- Split panes show different rows for same TableState

### Pitfall 3: Viewport Buffer Too Small for Smooth Scrolling
**What goes wrong:** Visible lag or "popping" when scrolling quickly (press and hold j/k)
**Why it happens:** Buffer smaller than viewport means new rows must be calculated mid-scroll
**How to avoid:** Buffer ≥ 2× viewport_height (current implementation uses exactly 2×)
**Warning signs:**
- Frame drops when scrolling rapidly
- Brief flicker/blank at top or bottom of table during scroll
- Scrolling feels "choppy" rather than smooth

### Pitfall 4: Rendering Too Frequently (Wasted CPU)
**What goes wrong:** High CPU usage even when idle, fans spin up
**Why it happens:** Event loop renders every iteration without checking if state changed
**How to avoid:** Track `needs_redraw` flag, only set after state-changing events
**Warning signs:**
- High CPU usage when application is idle
- Battery drain on laptops
- Terminal emulator uses significant CPU even with no user input

### Pitfall 5: Filter Performance Degrades with Large Datasets
**What goes wrong:** Typing in search box ('/') becomes sluggish with 1M+ rows
**Why it happens:** Current filter implementation scans all rows on every keystroke
**How to avoid:**
```rust
// Current implementation in build_pane_render_data():
let filtered_indices: Vec<usize> = tab.data.rows
    .iter()
    .enumerate()
    .filter(|(_, row)| {
        row.iter().any(|cell| tab.data.resolve(cell).to_lowercase().contains(&filter_lower))
    })
    .map(|(i, _)| i)
    .collect();
```
**Optimization:** Apply viewport windowing to filtered results (calculate filtered indices only for viewport window, not all 1.8M rows)
**Warning signs:**
- Typing in search feels delayed with large datasets
- Cursor lags behind keystrokes
- Frame rate drops below 30 FPS during search

### Pitfall 6: String Resolution Inside Hot Loop
**What goes wrong:** Viewport windowing still slow despite small window
**Why it happens:** Resolving Spur → &str has overhead, multiplied by cells in window
**How to avoid:** Batch resolution once per viewport update, reuse resolved strings
**Current approach:** Pattern 1 already does this correctly (resolves during window extraction, stores in Vec<Vec<String>>)
**Warning signs:**
- Profiling shows `Rodeo::resolve()` as top CPU consumer
- Scroll performance varies with column count (more columns = more resolutions)

## Code Examples

Verified patterns from current codebase:

### Boundary-Safe Viewport Calculation
```rust
// Source: src/render.rs build_pane_render_data() lines 90-107
let selected = tab.table_state.selected().unwrap_or(0);
let buffer = viewport_height.saturating_mul(2);

let (display_rows, displayed_row_count, viewport_row_offset) = if tab.filter_text.is_empty() {
    let total = tab.data.rows.len();
    let start = selected.saturating_sub(buffer);  // ✅ Safe underflow
    let end = selected.saturating_add(buffer).min(total);  // ✅ Safe overflow
    // Resolve symbols to strings for PaneRenderData
    let rows: Vec<Vec<String>> = tab.data.rows[start..end]
        .iter()
        .map(|row| {
            row.iter()
                .map(|s| tab.data.resolve(s).to_string())
                .collect()
        })
        .collect();
    (rows, total, start)
} else {
    // Filtered path (similar pattern with filtered_indices)
    // ...
};
```

### TableState Translation for Viewport Rendering
```rust
// Source: src/main.rs lines 491-515
// Adjust table states for viewport windowing (translate absolute → relative)
let left_viewport_offset = left_pane_data
    .as_ref()
    .map(|p| p.viewport_row_offset)
    .unwrap_or(0);

if left_viewport_offset > 0 {
    if let Some(sel) = left_table_state.selected() {
        left_table_state.select(Some(sel.saturating_sub(left_viewport_offset)));
    }
    *left_table_state.offset_mut() = left_table_state
        .offset()
        .saturating_sub(left_viewport_offset);
}

// ... render with Table widget ...

// Translate table states back from viewport-relative to absolute (line 688-699)
if left_viewport_offset > 0 {
    if let Some(sel) = left_table_state.selected() {
        left_table_state.select(Some(sel + left_viewport_offset));
    }
    *left_table_state.offset_mut() += left_viewport_offset;
}
```

### Event Polling for Frame Rate Control
```rust
// Source: src/main.rs line 720
// Current implementation (250ms polling)
if event::poll(Duration::from_millis(250))? {
    if let Event::Key(key) = event::read()? {
        // Handle key event
    }
}

// Recommended enhancement for 30 FPS target:
use std::time::{Duration, Instant};

const TARGET_FPS: u64 = 30;
const FRAME_TIME_MS: u64 = 1000 / TARGET_FPS;  // 33ms

let mut last_render = Instant::now();

loop {
    let now = Instant::now();
    let elapsed = now.duration_since(last_render).as_millis() as u64;
    let poll_duration = FRAME_TIME_MS.saturating_sub(elapsed);

    if event::poll(Duration::from_millis(poll_duration.max(1)))? {
        // Process event, may trigger redraw
    }

    if now.duration_since(last_render).as_millis() as u64 >= FRAME_TIME_MS {
        terminal.draw(|frame| { /* render */ })?;
        last_render = now;
    }
}
```

### Incremental Width Caching
```rust
// Source: src/workspace.rs Tab::update_cached_widths() lines 65-90
pub fn update_cached_widths(&mut self) {
    let num_cols = self.data.headers.len();
    if self.cached_auto_widths.is_empty() {
        self.cached_auto_widths = vec![0u16; num_cols];
        for (i, header) in self.data.headers.iter().enumerate() {
            self.cached_auto_widths[i] = (header.len() + 1) as u16;
        }
    }
    let start = self.widths_cached_for_rows;  // Resume from last scan
    let end = self.data.rows.len();
    if start < end {
        for row in &self.data.rows[start..end] {
            for (i, cell) in row.iter().enumerate() {
                if i < num_cols {
                    let w = (self.data.resolve(cell).len() + 1) as u16;
                    if w > self.cached_auto_widths[i] {
                        self.cached_auto_widths[i] = w;
                    }
                }
            }
        }
        self.widths_cached_for_rows = end;  // Mark rows as scanned
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Full dataset rendering | Viewport windowing | Phase 15-02 (2026-02-10) | Render time O(dataset_size) → O(viewport_size). Enables 1M+ row handling. |
| Vec<Vec<String>> storage | Vec<Vec<Spur>> with string interning | Phase 16-01 (2026-02-10) | 50-80% memory savings on repetitive data. Critical for 1.8M row target. |
| Synchronous blocking stdin | Streaming parser with background thread | Phase 15-01 (2026-02-10) | First rows visible in <1 second, data streams while UI responsive. |
| 250ms event poll | (Recommended) 33ms poll for 30 FPS | Not yet implemented | Current = 4 FPS max, recommended = 30 FPS smooth scrolling. |

**Deprecated/outdated:**
- **Full-dataset operations in render path:** Replaced by viewport windowing. Old parse_psql() → TableData with Vec<Vec<String>> rendered entirely. Now only viewport window rendered.
- **Blocking stdin read:** `io::stdin().read_to_string()` blocked until EOF. Replaced by StreamingParser in Phase 15-01.

## Open Questions

1. **Optimal viewport buffer size**
   - What we know: Current implementation uses 2× viewport_height (lines 90-91 render.rs)
   - What's unclear: Is 2× sufficient for smooth rapid scrolling (hold j/k)? Would 3× or 4× be better?
   - Recommendation: Benchmark scroll FPS with different buffer sizes. Create scrolling benchmark in benches/scrolling.rs measuring frames/sec during continuous scroll through 1M rows.

2. **Filter performance with large filtered result sets**
   - What we know: Current filter scans all rows (lines 108-135 render.rs), then applies viewport windowing to filtered_indices
   - What's unclear: Performance when 500K out of 1.8M rows match filter (large filtered set still slow)
   - Recommendation: Consider lazy filtering (only calculate visible filtered rows on demand). Or incremental filtering (resume scan from current viewport position).

3. **Frame rate target: 30 FPS vs 60 FPS**
   - What we know: Success criteria says "30+ FPS", ratatui can achieve 60+ FPS
   - What's unclear: Is 30 FPS sufficient for "smooth" terminal scrolling, or should we target 60 FPS?
   - Recommendation: User testing. Terminal emulator refresh rates typically 60Hz, but terminal I/O bottleneck may make 30 FPS feel smooth. Start with 30 FPS target (simpler), measure actual smoothness, increase if needed.

## Sources

### Primary (HIGH confidence)
- Ratatui official docs: https://ratatui.rs/concepts/rendering/ (immediate-mode rendering principles)
- Ratatui Table widget API: https://docs.rs/ratatui/latest/ratatui/widgets/struct.Table.html (TableState, offset handling)
- Crossterm event polling: https://docs.rs/crossterm/latest/crossterm/event/index.html (Duration-based poll())
- Current codebase: src/render.rs, src/main.rs, src/workspace.rs (viewport windowing already implemented)

### Secondary (MEDIUM confidence)
- Ratatui GitHub #174: https://github.com/ratatui/ratatui/issues/174 (Scrollable widget design discussion, why wrappers fail for performance)
- Ratatui GitHub #579: https://github.com/ratatui/ratatui/discussions/579 (Rendering best practices: delta-based updates, time-driven animation)
- rat-ftable (community table): https://github.com/thscharler/rat-ftable (Alternative O(screen-size) table widget approach)
- Textual TUI performance: https://textual.textualize.io/blog/2024/12/12/algorithms-for-high-performance-terminal-apps/ (Virtual scrolling for 1M+ lines, 60 FPS benchmarks)

### Tertiary (LOW confidence - general patterns, not Rust-specific)
- Virtual scrolling principles: https://stevekinney.com/courses/react-performance/windowing-and-virtualization (Windowing = render visible + buffer)
- Viewport windowing general: https://blog.octoperf.com/angular-performance-optimization---virtual-scroll/ (Buffer sizing: larger = smoother, smaller = less memory)
- Scroll boundary testing: https://arxiv.org/pdf/2210.00735 (ScrollTest evaluation framework for scroll accuracy)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All libraries already in use, well-documented, proven in production
- Architecture: HIGH - Viewport windowing already implemented and working, patterns verified in codebase
- Pitfalls: HIGH - Identified from codebase analysis (saturating arithmetic, translation pattern) and ratatui discussions
- Performance targets: MEDIUM - 30 FPS is reasonable based on research, but actual smoothness needs user testing

**Research date:** 2026-02-10
**Valid until:** ~60 days (stable domain, ratatui 0.29 released late 2024, no major API changes expected soon)
