# Project Milestones: Pretty Table Explorer

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

**What's next:** TBD — consider v1.1 for additional features (column resizing, export, etc.)

---
