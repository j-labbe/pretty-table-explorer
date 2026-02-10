# Feature Landscape: Performance Optimization for Large Datasets

**Domain:** Terminal table viewer / Data grid performance optimization
**Researched:** 2026-02-09
**Confidence:** MEDIUM (verified via web research, official documentation for Apache Arrow, lacks Context7 verification for TUI-specific patterns)

## Table Stakes

Features users expect in a high-performance data viewer. Missing any of these = product feels broken at scale.

| Feature | Why Expected | Complexity | Notes | Dependencies |
|---------|--------------|------------|-------|--------------|
| **Virtualized rendering** | Only visible rows rendered — standard for 100K+ rows | Medium | Render 20-50 rows (viewport + buffer), recycle DOM/rendering elements as user scrolls | Existing vertical scroll |
| **Fast initial display** | Users expect <1s to see SOMETHING, not wait for full load | Medium | Must decouple data loading from initial render — show first chunk immediately | None |
| **Smooth scrolling** | 60fps scrolling expected even at 1M+ rows | High | Terminal rendering is challenging — requires intelligent buffering, minimal ANSI output | Virtualized rendering |
| **Visible row count** | Users need to know "how much data am I looking at?" | Low | Display "Loaded 1,234 / 1,800,000 rows" during streaming, "Total: 1,800,000 rows" when complete | Progressive loading |
| **Loading feedback** | Users need to know system is working, not frozen | Low | Show indicator during load: spinner + message "Loading..." for <10s, progress % for >10s operations | Progressive loading |
| **Memory-efficient storage** | Can't hold 1.8M rows × all columns in naive String storage | High | Columnar storage (Apache Arrow pattern), string interning for repeated values, compact types | None — foundational |
| **Query cancellation** | Users must be able to abort long-running queries (Ctrl-C) | Medium | PostgreSQL: send cancel signal; Pipe mode: break read loop cleanly without corrupting state | Existing input handling |

## Differentiators

Features that set a high-performance data viewer apart. Not expected, but highly valued when present.

| Feature | Value Proposition | Complexity | Notes | Dependencies |
|---------|-------------------|------------|-------|--------------|
| **Progressive/streaming loading** | Display updates as data arrives — see first 100 rows instantly while remaining 1.8M load | High | Pipe mode: parse + render incrementally; PostgreSQL: cursor-based pagination (FETCH N) instead of SELECT * | Virtualized rendering, async data fetch |
| **Incremental search** | Search-as-you-type with immediate visual feedback | High | Index optimization for text matching at scale; must handle partial result sets during streaming | Search/filter (existing), indexed storage |
| **Lazy column loading** | Only parse/store columns visible in viewport | Medium | Parse minimal columns first, expand on demand when user scrolls horizontally | Column visibility controls (existing) |
| **Smart chunk sizing** | Automatically tune batch size based on row width, terminal size | Medium | Adaptive: 10-100MB chunks for wide tables, smaller for narrow; monitor parse time and adjust | Progressive loading |
| **Background data prefetch** | Load next N chunks in background while user views current data | Medium | Requires thread coordination — fetch ahead without blocking render loop | Progressive loading, threading |
| **Scroll position preservation** | Maintain row position during live data updates | Low | Track logical row index, not offset; important for streaming where row count changes | Virtualized rendering |
| **Estimated total row count** | Show "~1.5M rows (estimate)" before full load completes | Low | PostgreSQL: query planner estimates via EXPLAIN; Pipe: extrapolate from first N chunks | Progressive loading |
| **Sparse loading indicators** | Visual feedback for which data chunks are loaded vs pending | Low | Show "Loading rows 100,000-200,000..." in status bar or skeleton rows in viewport | Progressive loading |

## Anti-Features

Features to explicitly NOT build for performance optimization. Common in data tools but wrong for this domain.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **Infinite scroll without total count** | Users need to know dataset size for context ("am I at 10% or 90%?") | Always show total row count (loaded vs total during streaming, final count when complete) |
| **Aggressive pagination with page jumps** | Breaks terminal UX — smooth scrolling is the affordance | Virtualized rendering with smooth scroll, not discrete pages |
| **Complex loading animations** | Terminal constraints — fancy spinners add render overhead | Simple text indicators: "Loading..." or "Loaded 1.2M / 1.8M (67%)" |
| **Automatic data refresh/polling** | Unexpected data changes break user's mental model during exploration | Explicit refresh command only (user-initiated) |
| **Lazy loading with blank rows** | Breaks scrollbar accuracy and user expectations in TUI | Pre-fetch aggressively; show all rows (virtualized) once loaded |
| **Client-side sorting at scale** | Sorting 1.8M rows in-memory is slow and memory-intensive | Push sorting to PostgreSQL (ORDER BY); for pipe mode, warn or limit sort to loaded subset |
| **Multi-threaded parsing without coordination** | Rust ownership makes this complex; benefits marginal for streaming | Single-threaded incremental parsing is simpler and fast enough for terminal display rates |

## Feature Dependencies

```
Memory-efficient storage (foundational)
    ↓
Progressive/streaming loading
    ↓
┌──────────────┬──────────────────────────┐
↓              ↓                          ↓
Virtualized    Fast initial              Loading feedback
rendering      display                   + Row count
    ↓              ↓                          ↓
Smooth         Background                Scroll position
scrolling      prefetch                  preservation
    ↓
Incremental
search
```

**Critical path:** Memory-efficient storage → Progressive loading → Virtualized rendering → Smooth scrolling

**Independent:** Query cancellation (parallel), Smart chunk sizing (optimization)

**Column features (existing):** Lazy column loading builds on existing column hide/show/reorder

## MVP Recommendation for v1.4 Performance

**Prioritize (must-have for milestone):**

1. **Memory-efficient storage** — Foundational; enables everything else
2. **Progressive/streaming loading** — Core value: "fast initial display"
3. **Virtualized rendering** — Required for smooth scrolling at scale
4. **Smooth scrolling (60fps target)** — Quality bar for "feels fast"
5. **Loading feedback + row count** — User communication during load
6. **Query cancellation** — Safety valve for mistakes

**Defer to future milestones:**

- **Incremental search** — Needs indexing strategy research; existing search works for loaded data
- **Lazy column loading** — Optimization, not required for narrow tables (5-10 cols per user context)
- **Smart chunk sizing** — Can start with fixed 100MB chunks, optimize later
- **Background prefetch** — Adds threading complexity; simple streaming sufficient for MVP
- **Estimated row count** — Nice-to-have; "Loading..." without estimate acceptable
- **Sparse loading indicators** — Polish; basic "Loaded X / Total" is sufficient

**Rationale:** Deferred features are optimizations or polish on top of the core streaming + virtualization architecture. Getting foundational pieces right (storage, streaming, virtualization, smooth render) delivers the "feels fast" experience. Additional features can layer on incrementally without architectural changes.

## Performance Expectations by User Base

### Terminal Table Viewer Users (PTE's domain)

**Expect:**
- Instant initial display (<1s to first data visible)
- Smooth keyboard-driven scrolling (vim keys, arrows, PgUp/PgDn)
- Minimal resource usage (CPU, memory)
- Simple, clear loading feedback (no fancy spinners)
- Ability to cancel/interrupt operations (Ctrl-C)

**Don't expect:**
- Mouse-driven interactions (scrollbars, drag-to-scroll)
- Real-time data updates
- Editing capabilities
- Complex filtering UIs

### Data Grid Users (web/desktop — reference only)

**Expect:**
- 60fps smooth scrolling with mouse wheel
- Virtualization for 10K+ rows
- Progressive loading with skeleton screens
- Rich filtering/sorting UIs
- Multi-column interactions

**Overlap with terminal:**
- Virtualization patterns (render visible rows only)
- Loading feedback (progress indicators)
- Performance target (60fps)
- Memory efficiency (columnar storage)

**Key differences:**
- Terminal = keyboard-first, simple indicators
- Web/desktop = mouse-first, rich animations

## Complexity vs Impact Analysis

| Feature | Complexity | Impact | Priority |
|---------|-----------|--------|----------|
| Memory-efficient storage | High | Critical | P0 — Required |
| Progressive/streaming loading | High | Critical | P0 — Required |
| Virtualized rendering | Medium | Critical | P0 — Required |
| Smooth scrolling | High | High | P0 — Quality bar |
| Loading feedback | Low | Medium | P0 — UX baseline |
| Query cancellation | Medium | High | P0 — Safety |
| Incremental search | High | Medium | P1 — Defer |
| Lazy column loading | Medium | Low | P2 — Narrow tables, minor benefit |
| Smart chunk sizing | Medium | Low | P2 — Fixed size sufficient initially |
| Background prefetch | Medium | Low | P2 — Adds complexity, marginal gain |

**P0 = MVP, P1 = v1.5 candidate, P2 = Future optimization**

## Integration with Existing Features

### Column Controls (resize/hide/reorder)
- **Impact:** Lazy column loading would optimize memory for hidden columns
- **Requirement:** Virtualized rendering must respect column visibility state
- **Complexity:** Low — existing column state already tracked per Tab

### Search/Filter
- **Impact:** Incremental search needs indexed storage for performance at scale
- **Requirement:** Search must work on partial datasets during streaming (show "searching loaded rows only" indicator)
- **Complexity:** Medium — need to handle partial result sets

### Multi-tab Workspaces
- **Impact:** Each tab loads independently — streaming in tab 1 shouldn't block tab 2 interaction
- **Requirement:** Streaming state is per-tab, not global
- **Complexity:** Low — existing Tab isolation already handles this

### Split View
- **Impact:** Two panes rendering same or different datasets — virtualization per pane
- **Requirement:** Each pane has independent viewport + scroll state
- **Complexity:** Low — existing PaneRenderData pattern supports this

### CSV/JSON Export
- **Impact:** Export must handle partial datasets during streaming (export loaded rows, or wait for completion)
- **Requirement:** Clear UX: "Export loaded data (1.2M rows) or wait for full dataset (1.8M)?"
- **Complexity:** Low — async export with progress indicator

## Sources

### Performance Optimization & Virtualization
- [FastTableViewer - Terminal CSV/TSV viewer with streaming](https://github.com/codechenx/tv)
- [VisiData - Interactive multitool for tabular data](https://www.visidata.org/)
- [MUI X Data Grid - Virtualization](https://mui.com/x/react-data-grid/virtualization/)
- [DEV: Lazy Loading vs Virtualization](https://dev.to/richardtorres314/lazy-loading-vs-virtualization-4c60)
- [Medium: Optimizing React Performance - Virtualization](https://medium.com/@bilalazam751/optimizing-react-performance-virtualization-lazy-loading-and-memoization-9a402006c5e8)
- [Patterns.dev: List Virtualization](https://www.patterns.dev/vanilla/virtual-lists/)
- [Material React Table: Row Virtualization](https://www.material-react-table.com/docs/examples/row-virtualization)

### Progressive Loading & Streaming
- [UX Patterns: Progressive Loading](https://uxpatterns.dev/glossary/progressive-loading)
- [Milvus: Progressive Loading Techniques](https://milvus.io/ai-quick-reference/what-techniques-exist-for-progressive-loading-in-multimodal-search-interfaces)
- [Medium: Efficient Pagination with PostgreSQL Cursors](https://medium.com/@ietienam/efficient-pagination-with-postgresql-using-cursors-83e827148118)
- [Uptrace: Cursor Pagination for PostgreSQL Complete Guide](https://bun.uptrace.dev/guide/cursor-pagination.html)
- [Milan Jovanovic: Understanding Cursor Pagination](https://www.milanjovanovic.tech/blog/understanding-cursor-pagination-and-why-its-so-fast-deep-dive)

### Memory Efficiency & Columnar Storage
- [Apache Arrow: Columnar Format](https://arrow.apache.org/docs/format/Columnar.html)
- [Apache Arrow FAQ](https://arrow.apache.org/faq/)
- [Medium: Apache Arrow Complete Guide](https://medium.com/iomete/what-is-apache-arrow-a-complete-guide-to-columnar-in-memory-data-format-91e2550a8bf0)
- [MotherDuck: Columnar Storage Guide](https://motherduck.com/learn-more/columnar-storage-guide/)

### Terminal Rendering Performance
- [Ratatui: Rust TUI Framework](https://ratatui.rs/)
- [Ratatui Performance & Immediate Mode Rendering](https://www.blog.brightcoding.dev/2025/09/13/ratatui-building-rich-terminal-user-interfaces-in-rust/)
- [Kitty Terminal Performance](https://sw.kovidgoyal.net/kitty/performance/)
- [Terminal Smooth Scrolling](https://flak.tedunangst.com/post/terminal-smooth-scrolling)

### Loading Indicators & User Feedback
- [Smashing Magazine: Best Practices for Animated Progress Indicators](https://www.smashingmagazine.com/2016/12/best-practices-for-animated-progress-indicators/)
- [Nielsen Norman Group: Progress Indicators](https://www.nngroup.com/articles/progress-indicators/)
- [Pencil & Paper: UX Design Patterns for Loading](https://www.pencilandpaper.io/articles/ux-pattern-analysis-loading-feedback)
- [Pencil & Paper: Data Table Design UX Patterns](https://www.pencilandpaper.io/articles/ux-pattern-analysis-enterprise-data-tables)

### Search & Filter Optimization
- [Digma: How Indexing Enhances Query Performance](https://digma.ai/how-indexing-enhances-query-performance/)
- [GeeksforGeeks: Index Optimization](https://www.geeksforgeeks.org/sql/index-optimization/)
- [Acceldata: Database Indexing Strategies](https://www.acceldata.io/blog/mastering-database-indexing-strategies-for-peak-performance)

### Query Cancellation
- [CockroachDB: Manage Long-Running Queries](https://www.cockroachlabs.com/docs/stable/manage-long-running-queries)
- [SQLite: Interrupt Long-Running Query](https://sqlite.org/c3ref/interrupt.html)
- [Medium: Query Cancellation in SQL and ASP.NET Core](https://medium.com/@shahrukhkhan_7802/understanding-query-cancellation-in-sql-and-asp-net-core-f3dff35be305)

### Chunk Size & Batch Optimization
- [ESIP: Optimization Practices - Chunk Size](https://esipfed.github.io/cloud-computing-cluster/optimization-practices.html)
- [Couchbase: Guide to Data Chunking](https://www.couchbase.com/blog/data-chunking/)
- [GitHub: Optimal Chunk Size for Streaming](https://github.com/aio-libs/aiohttp/discussions/6285)
