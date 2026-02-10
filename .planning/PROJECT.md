# Pretty Table Explorer

## What This Is

An interactive terminal-based table viewer for PostgreSQL optimized for large datasets. Provides streaming data load, string-interned storage, 30 FPS smooth scrolling, column controls (resize, hide, reorder), multi-tab workspaces, split view comparison, and data export. Handles 1.8M+ row datasets with sub-second first display and low memory footprint.

## Core Value

Clean table rendering with proper column alignment — no wrapping, no spacing issues, just readable data.

## Requirements

### Validated

- ✓ Clean table rendering with proper column alignment — v1.0
- ✓ Horizontal scrolling for wide tables — v1.0
- ✓ Vertical scrolling for large result sets — v1.0
- ✓ Vim-style navigation (hjkl) — v1.0
- ✓ Arrow key navigation — v1.0
- ✓ Accept piped input from psql — v1.0
- ✓ Direct PostgreSQL connection with query interface — v1.0
- ✓ Search/filter rows within table view — v1.0
- ✓ Multi-platform binary releases — v1.1
- ✓ One-line install script — v1.1
- ✓ Self-update command — v1.1
- ✓ Version flag (--version) — v1.1
- ✓ Column width resizing (+/-) — v1.2
- ✓ Column hide/show (H/S) — v1.2
- ✓ Column reordering (</>) — v1.2
- ✓ Export to CSV and JSON — v1.2
- ✓ Multi-tab workspaces — v1.2
- ✓ Split view for table comparison — v1.2
- ✓ Horizontal scroll indicators — v1.2
- ✓ Streaming data load with sub-second first display — v1.4
- ✓ Loading progress indicator (rows loaded) — v1.4
- ✓ Navigate/scroll during background loading — v1.4
- ✓ Graceful Ctrl+C load cancellation — v1.4
- ✓ Memory-efficient string interning (50-80% savings) — v1.4
- ✓ Runtime memory usage display in status bar — v1.4
- ✓ Smooth 30 FPS scrolling through 1.8M+ rows — v1.4
- ✓ Viewport-windowed rendering (O(viewport) constant time) — v1.4

### Active

(None — all milestones shipped. Define next milestone with `/gsd:new-milestone`.)

### Out of Scope

- Editing data (INSERT/UPDATE/DELETE) — read-only viewer
- Multiple database connections — single connection at a time
- Copy to clipboard — keep interaction model simple
- Apache Arrow columnar storage — string interning sufficient
- Tokio async runtime — native threads sufficient for single data flow
- Search indexing (tantivy) — linear search acceptable for current scale

## Background

The standard psql output breaks down with wide tables or large result sets — columns wrap awkwardly, horizontal data is hard to follow, and there's no way to scroll back through results. This tool brings the same interactive scrolling experience that modern terminal apps provide.

Target environment is Ubuntu Linux running under WSL2.

## Constraints

- **Language**: Rust — fast, safe, excellent TUI ecosystem (ratatui)
- **Platform**: Linux/macOS — x86_64 and aarch64 supported

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust with ratatui | Best TUI library ecosystem, performance for large datasets | ✓ Good — v1.0 shipped with excellent performance |
| Dual input modes (pipe + direct) | Flexibility for different workflows | ✓ Good — both modes work seamlessly |
| Read-only for v1 | Reduce complexity, focus on viewing experience | ✓ Good — focused scope enabled fast delivery |
| Sync postgres crate | Avoid async complexity in TUI event loop | ✓ Good — simple integration |
| use-dev-tty feature | Enable keyboard input when stdin is piped | ✓ Good — critical for dual-mode operation |
| AppMode enum for input | Clear state management between modes | ✓ Good — clean separation of concerns |
| clap derive macros | Cleaner CLI code than manual parsing | ✓ Good — 45 lines reduced to 12 |
| ureq for HTTP | Sync API, smaller binary than reqwest | ✓ Good — self-update works without async runtime |
| POSIX shell installer | Maximum portability across Linux/macOS | ✓ Good — works on bash, zsh, sh |
| Native ARM runners | Simpler than cross-compilation for macOS | ✓ Good — reliable builds |
| Viewport-based horizontal scroll | Render only visible columns at actual widths | ✓ Good — fixes ratatui compression issues |
| PaneRenderData pattern | Pre-compute render state to avoid borrow conflicts | ✓ Good — clean split view implementation |
| Tab-isolated state | All table state in Tab struct, not main scope | ✓ Good — clean multi-tab architecture |
| Two-pass indicator width calc | Reserve space for right indicator, recalc if no overflow | ✓ Good — proper scroll indicator positioning |
| KeyAction/WorkspaceOp enums | Defer workspace operations to avoid borrow conflicts | ✓ Good — clean handler extraction without global state |
| Module extraction pattern | Extract types->render->handlers sequentially | ✓ Good — 55% main.rs reduction, zero warnings |
| Library crate pattern | All modules re-exported via src/lib.rs for external access | ✓ Good — enables Criterion benchmarks and integration tests |
| Background thread + mpsc streaming | Non-blocking data load with 1000-row batches | ✓ Good — sub-second first display for 500K+ rows |
| Viewport-windowed width calc | 10K-row sliding window for O(1) frame cost | ✓ Good — constant render time proven via benchmarks |
| lasso string interning | Vec<Vec<Spur>> with 4-byte symbols instead of String | ✓ Good — 50-80% memory savings on repetitive data |
| sysinfo memory tracking | RSS display in status bar, throttled to every 30 frames | ✓ Good — zero performance impact |
| 30 FPS frame timing | 33ms poll with needs_redraw idle gate | ✓ Good — smooth scrolling + near-zero idle CPU |
| Measure-first approach | Profiling infrastructure before optimization | ✓ Good — benchmarks caught regressions, guided decisions |

## Context

Shipped v1.4 with 5,295 lines of Rust across 15+ source files.
Tech stack: Rust 2021 edition, ratatui v0.29, crossterm v0.28, postgres v0.19, clap v4, ureq v2, lasso v0.7, sysinfo, criterion, sha2, csv, serde, serde_json.
Dual-mode operation: stdin pipe for psql output, --connect for direct PostgreSQL access.
Advanced viewing: column controls (resize/hide/reorder), multi-tab workspaces with split view, CSV/JSON export, scroll indicators.
Performance: Streaming load (background thread + mpsc), string interning (50-80% memory savings), 30 FPS frame timing, viewport-windowed rendering (O(viewport) constant time).
Distribution: GitHub releases for 4 platforms, install script, self-update command.
Architecture: Clean module separation — main.rs, handlers.rs, render.rs, workspace.rs, state.rs, streaming.rs, lib.rs.
33 integration tests + Criterion benchmark suites protect against regressions.

---
*Last updated: 2026-02-10 after v1.4 milestone*
