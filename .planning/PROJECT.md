# Pretty Table Explorer

## What This Is

An interactive terminal-based table viewer for PostgreSQL that provides smooth scrolling and clean rendering of query results. Supports both piped psql output and direct database connections, making it easy to explore data without formatting chaos.

## Core Value

Clean table rendering with proper column alignment — no wrapping, no spacing issues, just readable data.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Clean table rendering with proper column alignment
- [ ] Horizontal scrolling for wide tables
- [ ] Vertical scrolling for large result sets
- [ ] Vim-style navigation (hjkl)
- [ ] Arrow key navigation
- [ ] Accept piped input from psql
- [ ] Direct PostgreSQL connection with query interface
- [ ] Search/filter rows within table view
- [ ] Interactive column resizing

### Out of Scope

- Editing data (INSERT/UPDATE/DELETE) — read-only viewer for v1
- Multiple database connections — single connection at a time
- Export to file (CSV/JSON) — not needed for viewing
- Copy to clipboard — keep interaction model simple

## Context

The standard psql output breaks down with wide tables or large result sets — columns wrap awkwardly, horizontal data is hard to follow, and there's no way to scroll back through results. This tool brings the same interactive scrolling experience that modern terminal apps (like Claude Code's REPL) provide.

Target environment is Ubuntu Linux running under WSL2.

## Constraints

- **Language**: Rust — fast, safe, excellent TUI ecosystem (ratatui)
- **Platform**: Linux/WSL2 — primary target, cross-platform not required for v1

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust with ratatui | Best TUI library ecosystem, performance for large datasets | — Pending |
| Dual input modes (pipe + direct) | Flexibility for different workflows | — Pending |
| Read-only for v1 | Reduce complexity, focus on viewing experience | — Pending |

---
*Last updated: 2026-01-13 after initialization*
