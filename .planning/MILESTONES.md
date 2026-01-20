# Project Milestones: Pretty Table Explorer

## v1.3 Code Quality (Shipped: 2026-01-20)

**Delivered:** Refactored monolithic main.rs into clean, layered architecture for better maintainability.

**Phases completed:** 11-13 (5 plans total)

**Key accomplishments:**

- Created state.rs module with AppMode, PendingAction, PaneRenderData types
- Created render.rs module with table rendering functions (~480 lines)
- Created handlers.rs module with keyboard input handlers (~600 lines)
- Reduced main.rs from ~1520 lines to 691 lines (55% reduction)
- Achieved zero clippy warnings with -D warnings flag
- Removed dead code methods from workspace.rs

**Stats:**

- 17 files modified
- 3,092 lines of Rust (total codebase)
- 3 phases, 5 plans
- Same day (2026-01-20)

**Git range:** `refactor(11-01)` -> `docs(13)`

**What's next:** Consider new features, performance improvements, or v2.0 planning

---

## v1.2.1 Patch (Shipped: 2026-01-20)

**Delivered:** Post-release bug fixes for split view multi-tab functionality.

**Phases completed:** 9 (6 FIX plans for split view UAT issues)

**Key accomplishments:**

- Per-tab view mode: migrated view_mode from global to per-tab state
- Split view Tab key switches pane focus correctly
- Horizontal scrolling works in both split view panes
- Tab bar and controls hint visible in split view
- Enter key opens tables in the focused pane (left or right)
- Tab closing no longer duplicates pane content
- Simplified Tab key to only toggle focus in split view

**Stats:**

- 16 files modified
- 8 fix commits, 6 FIX plans
- 4 days (2026-01-16 → 2026-01-20)

**Git range:** `fix(09-03-FIX)` → `fix(09-03-FIX6)`

**UAT issues resolved:** 11 total (UAT-001 through UAT-011)

**What's next:** TBD - consider new features or v2.0 planning

---

## v1.2 Advanced Viewing (Shipped: 2026-01-16)

**Delivered:** Enhanced table interaction with column controls, data export, multi-tab workspaces, split view, and scroll indicators.

**Phases completed:** 7-10 (9 plans + 4 FIX plans)

**Key accomplishments:**

- Column controls: resize (+/-), hide/show (H/S), reorder (</>) with full horizontal scrolling
- Data export to CSV and JSON formats respecting column visibility
- Multi-tab workspaces with named tabs, tab navigation (Tab/Shift+Tab, number keys)
- Split view for side-by-side table comparison with focus-aware navigation (V toggle, Ctrl+W switch)
- Visual scroll indicators (◀/▶) on table edges showing horizontal scroll availability
- Viewport-based horizontal scrolling with proper column width handling

**Stats:**

- 36 files modified
- 2,791 lines of Rust (total codebase)
- 4 phases, 9 plans, 4 FIX plans
- 2 days (2026-01-15 → 2026-01-16)

**Git range:** `feat(07-01)` → `fix(10-01-FIX3)`

**What's next:** TBD - consider interactive column resizing, additional export formats, or new features

---

## v1.1 Distribution (Shipped: 2026-01-14)

**Delivered:** Multi-platform releases with one-line installation and self-update capability.

**Phases completed:** 5-6 (4 plans total)

**Key accomplishments:**

- Multi-platform release workflow (Linux/macOS, x86_64/aarch64)
- CI pipeline with test, clippy, fmt checks
- One-line install script with platform detection and checksum verification
- Self-update command (`pte update`)
- Version flag (--version) via clap

**Stats:**

- 2 phases, 4 plans
- Releases: v1.1.0, v1.1.1
- Timeline: 1 day (2026-01-14)

**Git tags:** `v1.1.0`, `v1.1.1`

**What's next:** Plan v1.2 or v2.0 features (column resizing, export, etc.)

---

## v1.0 MVP (Shipped: 2026-01-14)

**Delivered:** Interactive terminal table viewer for PostgreSQL with dual-mode operation (stdin pipe + direct connection), vim-style navigation, and query/search capabilities.

**Phases completed:** 1-4 (8 plans total)

**Key accomplishments:**

- Rust TUI foundation with ratatui v0.29 and crossterm for terminal handling
- psql output parsing with Unicode support and proper column alignment
- Vim-style (hjkl) and arrow key navigation with page scrolling (Ctrl+U/D)
- Direct PostgreSQL connection via --connect flag with clear error handling
- Interactive SQL query execution with ':' command mode
- Case-insensitive row filtering with '/' search mode

**Stats:**

- 25 files created/modified
- 4,235 lines of Rust
- 4 phases, 8 plans
- 2 days from project start to ship

**Git range:** `feat(01-01)` → `feat(04-02)`

---
