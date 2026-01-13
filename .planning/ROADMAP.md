# Roadmap: Pretty Table Explorer

## Overview

Build an interactive terminal table viewer in Rust, starting with core rendering, adding navigation controls, then PostgreSQL integration. The journey takes us from parsing piped psql output to a full interactive database explorer.

## Domain Expertise

None

## Phases

- [x] **Phase 1: Foundation** - Project setup, Rust/ratatui scaffold, basic terminal rendering ✓
- [x] **Phase 2: Table Rendering** - Parse psql output, column alignment, clean table display ✓
- [x] **Phase 3: Navigation** - Vim/arrow key controls, horizontal/vertical scrolling ✓
- [x] **Phase 4: PostgreSQL Integration** - Direct database connection, query interface, search/filter ✓

## Phase Details

### Phase 1: Foundation
**Goal**: Working Rust project with ratatui, basic terminal app that can read stdin
**Depends on**: Nothing (first phase)
**Research**: Unlikely (project setup, established patterns)
**Plans**: TBD

Plans:
- [x] 01-01: Cargo project setup with ratatui dependencies ✓
- [x] 01-02: Basic terminal UI scaffold with event loop ✓

### Phase 2: Table Rendering
**Goal**: Clean table display from piped psql output with proper column alignment
**Depends on**: Phase 1
**Research**: Likely (ratatui table widget)
**Research topics**: ratatui Table widget API, column width calculation, Unicode handling
**Plans**: TBD

Plans:
- [x] 02-01: Parse psql pipe format into structured data ✓
- [x] 02-02: Render table with calculated column widths ✓

### Phase 3: Navigation
**Goal**: Full keyboard navigation with smooth scrolling
**Depends on**: Phase 2
**Research**: Unlikely (internal patterns from Phase 2)
**Plans**: TBD

Plans:
- [x] 03-01: Vim-style (hjkl) and arrow key navigation ✓
- [x] 03-02: Horizontal/vertical scrolling with viewport tracking ✓

### Phase 4: PostgreSQL Integration
**Goal**: Direct database connection with query interface and search
**Depends on**: Phase 3
**Research**: Likely (external library)
**Research topics**: tokio-postgres or sqlx client, async runtime integration, connection management
**Plans**: TBD

Plans:
- [x] 04-01: PostgreSQL connection and query execution ✓
- [x] 04-02: Interactive query input and search/filter ✓

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 2/2 | Complete | 2026-01-13 |
| 2. Table Rendering | 2/2 | Complete | 2026-01-13 |
| 3. Navigation | 2/2 | Complete | 2026-01-13 |
| 4. PostgreSQL Integration | 2/2 | Complete | 2026-01-13 |
