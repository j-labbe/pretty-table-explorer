# Requirements: Pretty Table Explorer

**Defined:** 2026-02-09
**Core Value:** Clean table rendering with proper column alignment — no wrapping, no spacing issues, just readable data.

## v1.4 Requirements

Requirements for performance milestone. Each maps to roadmap phases.

### Loading

- [ ] **LOAD-01**: User sees table data on screen within 1 second of piping, even for multi-million row datasets
- [ ] **LOAD-02**: User sees loading progress indicator showing rows loaded so far during streaming
- [ ] **LOAD-03**: User can navigate and scroll through partially-loaded data while loading continues
- [ ] **LOAD-04**: User can cancel a long-running load with Ctrl+C without crashing

### Memory

- [ ] **MEM-01**: User can load 1.8M row dataset with less than 1GB memory usage (down from ~2GB)
- [ ] **MEM-02**: User sees memory usage displayed in the status bar

### Rendering

- [ ] **REND-01**: User experiences smooth scrolling (no lag) through 1.8M+ row datasets
- [ ] **REND-02**: User sees only visible rows rendered (render time constant regardless of dataset size)

## Future Requirements

Deferred to future release. Tracked but not in current roadmap.

### Search Optimization

- **SRCH-01**: User can search/filter 1.8M rows with sub-second response time
- **SRCH-02**: User sees incremental search results as they type

### Advanced Profiling

- **PROF-01**: Criterion benchmarks for parsing, rendering, and search operations
- **PROF-02**: Flamegraph and heap profiling infrastructure for ongoing optimization

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Apache Arrow columnar storage | Over-engineering for table viewer; string interning sufficient |
| Tokio async runtime | Single data flow doesn't justify async complexity; native threads sufficient |
| Database-side pagination (cursors) | Only applies to --connect mode; piped psql is primary target |
| Search indexing (tantivy) | Adds significant complexity; linear search acceptable for v1.4 |
| Frame rate limiting | Ratatui's built-in diffing handles this; add only if profiling shows need |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| LOAD-01 | Pending | Pending |
| LOAD-02 | Pending | Pending |
| LOAD-03 | Pending | Pending |
| LOAD-04 | Pending | Pending |
| MEM-01 | Pending | Pending |
| MEM-02 | Pending | Pending |
| REND-01 | Pending | Pending |
| REND-02 | Pending | Pending |

**Coverage:**
- v1.4 requirements: 8 total
- Mapped to phases: 0
- Unmapped: 8 (pending roadmap creation)

---
*Requirements defined: 2026-02-09*
*Last updated: 2026-02-09 after initial definition*
