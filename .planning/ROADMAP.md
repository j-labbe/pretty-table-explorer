# Roadmap: Pretty Table Explorer

## Overview

Build an interactive terminal table viewer in Rust, starting with core rendering, adding navigation controls, then PostgreSQL integration. The journey takes us from parsing piped psql output to a full interactive database explorer.

## Milestones

- ✅ **v1.0 MVP** — Phases 1-4 (shipped 2026-01-14)
- 🚧 **v1.1 Distribution** — Phases 5-6 (in progress)

## Completed Milestones

- ✅ [v1.0 MVP](milestones/v1.0-ROADMAP.md) (Phases 1-4) — SHIPPED 2026-01-14

<details>
<summary>✅ v1.0 MVP (Phases 1-4) — SHIPPED 2026-01-14</summary>

- [x] Phase 1: Foundation (2/2 plans) — completed 2026-01-13
- [x] Phase 2: Table Rendering (2/2 plans) — completed 2026-01-13
- [x] Phase 3: Navigation (2/2 plans) — completed 2026-01-13
- [x] Phase 4: PostgreSQL Integration (2/2 plans) — completed 2026-01-13

</details>

### 🚧 v1.1 Distribution (In Progress)

**Milestone Goal:** Make the tool easy to install and keep updated with multi-platform releases, an install script, and self-update capability.

#### Phase 5: Release Infrastructure ✅

**Goal**: GitHub Actions workflow for multi-platform builds, version embedding (--version flag), release asset naming
**Depends on**: v1.0 MVP complete
**Status**: Complete
**Completed**: 2026-01-14

Plans:
- [x] 05-01: Version embedding + --version flag (clap CLI parsing)
- [x] 05-02: GitHub Actions workflows (release + CI)

#### Phase 6: Installation & Updates

**Goal**: Install script (curl | bash), self-update command, platform detection logic
**Depends on**: Phase 5
**Research**: Unlikely (shell scripting, standard patterns)
**Plans**: TBD

Plans:
- [ ] 06-01: TBD (run /gsd:plan-phase 6 to break down)

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Foundation | v1.0 | 2/2 | Complete | 2026-01-13 |
| 2. Table Rendering | v1.0 | 2/2 | Complete | 2026-01-13 |
| 3. Navigation | v1.0 | 2/2 | Complete | 2026-01-13 |
| 4. PostgreSQL Integration | v1.0 | 2/2 | Complete | 2026-01-13 |
| 5. Release Infrastructure | v1.1 | 2/2 | Complete | 2026-01-14 |
| 6. Installation & Updates | v1.1 | 0/? | Not started | - |

## Domain Expertise

None
