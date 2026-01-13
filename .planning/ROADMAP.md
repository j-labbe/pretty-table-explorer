# Roadmap: Pretty Table Explorer

## Overview

Build an interactive terminal table viewer in Rust, starting with core rendering, adding navigation controls, then PostgreSQL integration. The journey takes us from parsing piped psql output to a full interactive database explorer.

## Domain Expertise

None

## Phases

- [ ] **Phase 1: Foundation** - Project setup, Rust/ratatui scaffold, basic terminal rendering
- [ ] **Phase 2: Table Rendering** - Parse psql output, column alignment, clean table display
- [ ] **Phase 3: Navigation** - Vim/arrow key controls, horizontal/vertical scrolling
- [ ] **Phase 4: PostgreSQL Integration** - Direct database connection, query interface, search/filter

## Phase Details

### Phase 1: Foundation
**Goal**: Working Rust project with ratatui, basic terminal app that can read stdin
**Depends on**: Nothing (first phase)
**Research**: Unlikely (project setup, established patterns)
**Plans**: TBD

Plans:
- [ ] 01-01: Cargo project setup with ratatui dependencies
- [ ] 01-02: Basic terminal UI scaffold with event loop

### Phase 2: Table Rendering
**Goal**: Clean table display from piped psql output with proper column alignment
**Depends on**: Phase 1
**Research**: Likely (ratatui table widget)
**Research topics**: ratatui Table widget API, column width calculation, Unicode handling
**Plans**: TBD

Plans:
- [ ] 02-01: Parse psql pipe format into structured data
- [ ] 02-02: Render table with calculated column widths

### Phase 3: Navigation
**Goal**: Full keyboard navigation with smooth scrolling
**Depends on**: Phase 2
**Research**: Unlikely (internal patterns from Phase 2)
**Plans**: TBD

Plans:
- [ ] 03-01: Vim-style (hjkl) and arrow key navigation
- [ ] 03-02: Horizontal/vertical scrolling with viewport tracking

### Phase 4: PostgreSQL Integration
**Goal**: Direct database connection with query interface and search
**Depends on**: Phase 3
**Research**: Likely (external library)
**Research topics**: tokio-postgres or sqlx client, async runtime integration, connection management
**Plans**: TBD

Plans:
- [ ] 04-01: PostgreSQL connection and query execution
- [ ] 04-02: Interactive query input and search/filter

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 0/2 | Not started | - |
| 2. Table Rendering | 0/2 | Not started | - |
| 3. Navigation | 0/2 | Not started | - |
| 4. PostgreSQL Integration | 0/2 | Not started | - |
