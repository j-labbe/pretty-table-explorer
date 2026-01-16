# Pretty Table Explorer

## What This Is

An interactive terminal-based table viewer for PostgreSQL with advanced viewing features. Provides smooth scrolling, column controls (resize, hide, reorder), multi-tab workspaces, split view comparison, and data export. Supports both piped psql output and direct database connections.

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
- ✓ Horizontal scroll indicators (◀/▶) — v1.2

### Active

(None — considering next milestone scope)

### Out of Scope

- Editing data (INSERT/UPDATE/DELETE) — read-only viewer
- Multiple database connections — single connection at a time
- Copy to clipboard — keep interaction model simple

## Background

The standard psql output breaks down with wide tables or large result sets — columns wrap awkwardly, horizontal data is hard to follow, and there's no way to scroll back through results. This tool brings the same interactive scrolling experience that modern terminal apps (like Claude Code's REPL) provide.

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

## Context

Shipped v1.2 with ~2,800 lines of Rust.
Tech stack: Rust 2021 edition, ratatui v0.29, crossterm v0.28, postgres v0.19, clap v4, ureq v2, sha2 v0.10, csv v1, serde v1, serde_json v1.
Dual-mode operation: stdin pipe for psql output, --connect for direct PostgreSQL access.
Advanced viewing: column controls (resize/hide/reorder), multi-tab workspaces with split view, CSV/JSON export, scroll indicators.
Distribution: GitHub releases for 4 platforms, install script, self-update command.

---
*Last updated: 2026-01-16 after v1.2 milestone*
