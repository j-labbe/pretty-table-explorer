# Roadmap: Pretty Table Explorer

## Overview

Build an interactive terminal table viewer in Rust, starting with core rendering, adding navigation controls, then PostgreSQL integration. The journey takes us from parsing piped psql output to a full interactive database explorer.

## Milestones

- ✅ **v1.0 MVP** — Phases 1-4 (shipped 2026-01-14)
- ✅ **v1.1 Distribution** — Phases 5-6 (shipped 2026-01-14)
- ✅ **v1.2 Advanced Viewing** — Phases 7-10 (shipped 2026-01-16)
- ✅ **v1.2.1 Patch** — Phase 9 FIX plans (shipped 2026-01-20)
- ✅ **v1.3 Code Quality** — Phases 11-13 (shipped 2026-01-20)
- 🚧 **v1.4 Performance** — Phases 14-17 (in progress)

## Completed Milestones

- [v1.0 MVP](milestones/v1.0-ROADMAP.md) (Phases 1-4) — SHIPPED 2026-01-14
- [v1.1 Distribution](milestones/v1.1-ROADMAP.md) (Phases 5-6) — SHIPPED 2026-01-14
- [v1.2 Advanced Viewing](milestones/v1.2-ROADMAP.md) (Phases 7-10) — SHIPPED 2026-01-16
- [v1.2.1 Patch](milestones/v1.2.1-ROADMAP.md) (Phase 9 FIX) — SHIPPED 2026-01-20
- [v1.3 Code Quality](milestones/v1.3-ROADMAP.md) (Phases 11-13) — SHIPPED 2026-01-20

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

<details>
<summary> v1.3 Code Quality (Phases 11-13) — SHIPPED 2026-01-20</summary>

- [x] Phase 11: Core Types Extraction (1/1 plan) — completed 2026-01-20
- [x] Phase 12: UI Layer Extraction (2/2 plans) — completed 2026-01-20
- [x] Phase 13: Handlers & Cleanup (2/2 plans) — completed 2026-01-20

</details>

## 🚧 v1.4 Performance (In Progress)

**Milestone Goal:** Optimize PTE to handle million-row datasets with fast loading and smooth scrolling.

### Phase 14: Profiling Infrastructure
**Goal**: Establish measurement-driven optimization foundation
**Depends on**: Phase 13
**Requirements**: Foundation for LOAD, MEM, REND requirements
**Success Criteria** (what must be TRUE):
  1. Criterion benchmarks exist for parsing, rendering, and scroll operations (detect regressions)
  2. Developer can generate flamegraphs to identify CPU bottlenecks
  3. Developer can run heap profiler (dhat) to measure memory allocations
  4. Integration tests exist for search, export, and column operations (prevent regressions during refactoring)
  5. Panic hooks restore terminal state on crash
**Plans:** 3 plans

Plans:
- [x] 14-01-PLAN.md — Library crate extraction, Cargo config, dhat profiling, panic hook
- [x] 14-02-PLAN.md — Criterion benchmarks (parsing, rendering, scrolling)
- [x] 14-03-PLAN.md — Integration tests (search, export, column operations)

### Phase 15: Streaming Load
**Goal**: User sees data immediately while loading continues in background
**Depends on**: Phase 14
**Requirements**: LOAD-01, LOAD-02, LOAD-03, LOAD-04
**Success Criteria** (what must be TRUE):
  1. User sees first rows on screen within 1 second of piping data (even for 1.8M+ row datasets)
  2. User sees loading indicator showing "Loaded X rows" during streaming
  3. User can navigate and scroll through partially-loaded data while loading continues
  4. User can press Ctrl+C to cancel a long-running load without application crash
**Plans**: TBD

Plans:
- [ ] 15-01: TBD

### Phase 16: Memory Optimization
**Goal**: Reduce memory footprint for large datasets via compact storage
**Depends on**: Phase 15
**Requirements**: MEM-01, MEM-02
**Success Criteria** (what must be TRUE):
  1. User can load 1.8M row dataset with less than 1GB memory usage (down from ~2GB)
  2. User sees current memory usage displayed in the status bar
  3. Search, export, and column operations work correctly with new storage (no regressions)
**Plans**: TBD

Plans:
- [ ] 16-01: TBD

### Phase 17: Virtualized Rendering
**Goal**: Smooth scrolling through massive datasets via viewport optimization
**Depends on**: Phase 16
**Requirements**: REND-01, REND-02
**Success Criteria** (what must be TRUE):
  1. User experiences smooth scrolling (no lag) through 1.8M+ row datasets at 30+ FPS
  2. Render time remains constant regardless of dataset size (only visible rows rendered)
  3. Scroll position stays accurate at top, middle, and bottom of large datasets (no off-by-one errors)
**Plans**: TBD

Plans:
- [ ] 17-01: TBD

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
| 13. Handlers & Cleanup | v1.3 | 2/2 | Complete | 2026-01-20 |
| 14. Profiling Infrastructure | v1.4 | 3/3 | Complete | 2026-02-10 |
| 15. Streaming Load | v1.4 | 0/TBD | Not started | - |
| 16. Memory Optimization | v1.4 | 0/TBD | Not started | - |
| 17. Virtualized Rendering | v1.4 | 0/TBD | Not started | - |

## Domain Expertise

None
