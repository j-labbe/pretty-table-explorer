# Pretty Table Explorer

## What This Is

An interactive terminal-based table viewer for PostgreSQL that provides smooth scrolling and clean rendering of query results. Supports both piped psql output and direct database connections, making it easy to explore data without formatting chaos.

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

### Active

- [ ] Interactive column resizing

### Out of Scope

- Editing data (INSERT/UPDATE/DELETE) — read-only viewer for v1
- Multiple database connections — single connection at a time
- Export to file (CSV/JSON) — not needed for viewing
- Copy to clipboard — keep interaction model simple

## Background

The standard psql output breaks down with wide tables or large result sets — columns wrap awkwardly, horizontal data is hard to follow, and there's no way to scroll back through results. This tool brings the same interactive scrolling experience that modern terminal apps (like Claude Code's REPL) provide.

Target environment is Ubuntu Linux running under WSL2.

## Constraints

- **Language**: Rust — fast, safe, excellent TUI ecosystem (ratatui)
- **Platform**: Linux/WSL2 — primary target, cross-platform not required for v1

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust with ratatui | Best TUI library ecosystem, performance for large datasets | ✓ Good — v1.0 shipped with excellent performance |
| Dual input modes (pipe + direct) | Flexibility for different workflows | ✓ Good — both modes work seamlessly |
| Read-only for v1 | Reduce complexity, focus on viewing experience | ✓ Good — focused scope enabled fast delivery |
| Sync postgres crate | Avoid async complexity in TUI event loop | ✓ Good — simple integration |
| use-dev-tty feature | Enable keyboard input when stdin is piped | ✓ Good — critical for dual-mode operation |
| AppMode enum for input | Clear state management between modes | ✓ Good — clean separation of concerns |

## Context

Shipped v1.0 with 4,235 lines of Rust.
Tech stack: Rust 2021 edition, ratatui v0.29, crossterm v0.28, postgres v0.19.
Dual-mode operation: stdin pipe for psql output, --connect for direct PostgreSQL access.
All core viewing features implemented: navigation, scrolling, query, filter.

---
*Last updated: 2026-01-14 after v1.0 milestone*
