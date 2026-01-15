# Roadmap: Pretty Table Explorer

## Overview

Build an interactive terminal table viewer in Rust, starting with core rendering, adding navigation controls, then PostgreSQL integration. The journey takes us from parsing piped psql output to a full interactive database explorer.

## Milestones

- ✅ **v1.0 MVP** — Phases 1-4 (shipped 2026-01-14)
- ✅ **v1.1 Distribution** — Phases 5-6 (shipped 2026-01-14)
- 🚧 **v1.2 Advanced Viewing** — Phases 7-9 (in progress)

## Completed Milestones

- [v1.0 MVP](milestones/v1.0-ROADMAP.md) (Phases 1-4) — SHIPPED 2026-01-14
- [v1.1 Distribution](milestones/v1.1-ROADMAP.md) (Phases 5-6) — SHIPPED 2026-01-14

<details>
<summary> v1.0 MVP (Phases 1-4) — SHIPPED 2026-01-14</summary>

- [x] Phase 1: Foundation (2/2 plans) — completed 2026-01-13
- [x] Phase 2: Table Rendering (2/2 plans) — completed 2026-01-13
- [x] Phase 3: Navigation (2/2 plans) — completed 2026-01-13
- [x] Phase 4: PostgreSQL Integration (2/2 plans) — completed 2026-01-13

</details>

<details>
<summary> v1.1 Distribution (Phases 5-6) — SHIPPED 2026-01-14</summary>

- [x] Phase 5: Release Infrastructure (2/2 plans) — completed 2026-01-14
- [x] Phase 6: Installation & Updates (2/2 plans) — completed 2026-01-14

</details>

### 🚧 v1.2 Advanced Viewing (In Progress)

**Milestone Goal:** Enhanced table interaction with column controls, data export, and multi-table workflows.

#### Phase 7: Column Controls

**Goal**: Resize, hide, and reorder columns (view-only modifications)
**Depends on**: Phase 6 (v1.1 complete)
**Research**: Unlikely (internal TUI patterns)
**Plans**: 3

Plans:
- [ ] 07-01: Column state and width resizing (+/- keys)
- [ ] 07-02: Column hide/show (H/S keys)
- [ ] 07-03: Column reordering (</> keys)

#### Phase 8: Data Export

**Goal**: Export current table view to CSV and JSON formats
**Depends on**: Phase 7
**Research**: Unlikely (standard serialization patterns)
**Plans**: TBD

Plans:
- [ ] 08-01: TBD (run /gsd:plan-phase 8 to break down)

#### Phase 9: Multiple Tables

**Goal**: Named tabs/workspaces for multiple query results + split view
**Depends on**: Phase 8
**Research**: Unlikely (extending existing architecture)
**Plans**: TBD

Plans:
- [ ] 09-01: TBD (run /gsd:plan-phase 9 to break down)

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Foundation | v1.0 | 2/2 | Complete | 2026-01-13 |
| 2. Table Rendering | v1.0 | 2/2 | Complete | 2026-01-13 |
| 3. Navigation | v1.0 | 2/2 | Complete | 2026-01-13 |
| 4. PostgreSQL Integration | v1.0 | 2/2 | Complete | 2026-01-13 |
| 5. Release Infrastructure | v1.1 | 2/2 | Complete | 2026-01-14 |
| 6. Installation & Updates | v1.1 | 2/2 | Complete | 2026-01-14 |
| 7. Column Controls | v1.2 | 0/3 | Planned | - |
| 8. Data Export | v1.2 | 0/? | Not started | - |
| 9. Multiple Tables | v1.2 | 0/? | Not started | - |

## Domain Expertise

None
