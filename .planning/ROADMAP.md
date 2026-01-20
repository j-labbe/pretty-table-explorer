# Roadmap: Pretty Table Explorer

## Overview

Build an interactive terminal table viewer in Rust, starting with core rendering, adding navigation controls, then PostgreSQL integration. The journey takes us from parsing piped psql output to a full interactive database explorer.

## Milestones

- ✅ **v1.0 MVP** — Phases 1-4 (shipped 2026-01-14)
- ✅ **v1.1 Distribution** — Phases 5-6 (shipped 2026-01-14)
- ✅ **v1.2 Advanced Viewing** — Phases 7-10 (shipped 2026-01-16)
- ✅ **v1.2.1 Patch** — Phase 9 FIX plans (shipped 2026-01-20)
- 🚧 **v1.3 Code Quality** — Phases 11-13 (in progress)

## Completed Milestones

- [v1.0 MVP](milestones/v1.0-ROADMAP.md) (Phases 1-4) — SHIPPED 2026-01-14
- [v1.1 Distribution](milestones/v1.1-ROADMAP.md) (Phases 5-6) — SHIPPED 2026-01-14
- [v1.2 Advanced Viewing](milestones/v1.2-ROADMAP.md) (Phases 7-10) — SHIPPED 2026-01-16
- [v1.2.1 Patch](milestones/v1.2.1-ROADMAP.md) (Phase 9 FIX) — SHIPPED 2026-01-20

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

<details>
<summary> v1.2 Advanced Viewing (Phases 7-10) — SHIPPED 2026-01-16</summary>

- [x] Phase 7: Column Controls (4/4 plans) — completed 2026-01-15
- [x] Phase 8: Data Export (1/1 plan) — completed 2026-01-15
- [x] Phase 9: Multiple Tables (3/3 plans) — completed 2026-01-15
- [x] Phase 10: Scroll Indicators (1/1 plan + 3 FIX) — completed 2026-01-16

</details>

<details>
<summary> v1.2.1 Patch (Phase 9 FIX) — SHIPPED 2026-01-20</summary>

- [x] 09-03-FIX: Per-tab view mode migration — completed 2026-01-20
- [x] 09-03-FIX2: Split view pane switching and scrolling — completed 2026-01-20
- [x] 09-03-FIX3: Split view UI fixes (tab bar, controls) — completed 2026-01-20
- [x] 09-03-FIX4: Split view pane focus for Enter key — completed 2026-01-20
- [x] 09-03-FIX5: Tab cycling and pane duplication fixes — completed 2026-01-20
- [x] 09-03-FIX6: Tab key simplification — completed 2026-01-20

</details>

### 🚧 v1.3 Code Quality (In Progress)

**Milestone Goal:** Refactor monolithic main.rs into clean, layered architecture for better maintainability

#### Phase 11: Core Types Extraction — ✅ Complete

**Goal**: Extract models, state, and data structures (AppState, Tab, TableData) into dedicated modules
**Depends on**: Previous milestone complete
**Research**: Unlikely (internal patterns, refactoring)
**Plans**: 1/1 complete

Plans:
- [x] 11-01: State Module Extraction — completed 2026-01-20

#### Phase 12: UI Layer Extraction — ✅ Complete

**Goal**: Extract views, rendering, and widgets into dedicated modules
**Depends on**: Phase 11
**Research**: Unlikely (internal patterns, refactoring)
**Plans**: 2/2 complete

Plans:
- [x] 12-01: Render Module Extraction — completed 2026-01-20
- [x] 12-02: Main Loop UI Extraction — completed 2026-01-20

#### Phase 13: Handlers & Cleanup

**Goal**: Extract input handlers, commands, utils; remove dead code; fix clippy warnings; final polish
**Depends on**: Phase 12
**Research**: Unlikely (internal patterns, refactoring)
**Plans**: 2

Plans:
- [ ] 13-01: Handler Module Extraction — extract keyboard handlers from main.rs
- [ ] 13-02: Clippy Fixes and Dead Code Removal — fix warnings, remove unused code

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Foundation | v1.0 | 2/2 | Complete | 2026-01-13 |
| 2. Table Rendering | v1.0 | 2/2 | Complete | 2026-01-13 |
| 3. Navigation | v1.0 | 2/2 | Complete | 2026-01-13 |
| 4. PostgreSQL Integration | v1.0 | 2/2 | Complete | 2026-01-13 |
| 5. Release Infrastructure | v1.1 | 2/2 | Complete | 2026-01-14 |
| 6. Installation & Updates | v1.1 | 2/2 | Complete | 2026-01-14 |
| 7. Column Controls | v1.2 | 4/4 | Complete | 2026-01-15 |
| 8. Data Export | v1.2 | 1/1 | Complete | 2026-01-15 |
| 9. Multiple Tables | v1.2 | 3/3 | Complete | 2026-01-15 |
| 10. Scroll Indicators | v1.2 | 1/1 | Complete | 2026-01-15 |
| 11. Core Types Extraction | v1.3 | 1/1 | Complete | 2026-01-20 |
| 12. UI Layer Extraction | v1.3 | 2/2 | Complete | 2026-01-20 |
| 13. Handlers & Cleanup | v1.3 | 0/2 | Planned | - |

## Domain Expertise

None
