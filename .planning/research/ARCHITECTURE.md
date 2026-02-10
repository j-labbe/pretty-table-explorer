# Architecture Patterns: Performance Optimization for Large Datasets

**Domain:** Rust TUI Table Viewer Performance Optimization
**Researched:** 2026-02-09
**Confidence:** HIGH

## Executive Summary

Performance optimization for 1.8M+ row datasets in a ratatui-based TUI requires three architectural changes working together:

1. **Streaming Architecture**: Move stdin parsing to a background thread communicating via channels to avoid blocking the UI
2. **Memory-Efficient Storage**: Replace `Vec<Vec<String>>` with compact representations (string interning or columnar storage) to reduce memory by 50-80%
3. **Virtualized Rendering**: Leverage ratatui's built-in buffer diffing and TableState offset to render only visible rows

The existing architecture provides clean integration points. Main changes affect data flow (synchronous stdin → async channels) and storage (String-heavy Vec → compact representation), while rendering and UI modules require minimal modification.

## Current Architecture Analysis

### Existing Data Flow
```
stdin (blocking read) → Vec<Vec<String>> → App::tabs → render loop → ratatui Table widget
```

**Bottlenecks:**
- Synchronous stdin reading blocks UI initialization (1.8M rows = minutes)
- `Vec<Vec<String>>` stores full strings with high memory overhead
- Full dataset stored in memory before UI becomes interactive
- Every render cycle accesses full dataset even though only ~30 rows visible

### Integration Points

| Module | Current Responsibility | Integration Point |
|--------|----------------------|-------------------|
| `main.rs` | Stdin parsing + app loop | Split: stdin → background thread, app loop stays |
| `state.rs` | Type definitions (App, Tab) | Add new storage types, parser messages |
| `render.rs` | Table rendering | Minimal change: adapt to new Row iterator |
| `workspace.rs` | Tab/split management | No changes needed |
| `handlers.rs` | Keyboard input | No changes needed |

## Recommended Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         Main Thread                          │
│  ┌────────────┐    ┌──────────────┐    ┌────────────────┐  │
│  │  Terminal  │───→│  App State   │───→│  Render Loop   │  │
│  │  (ratatui) │    │              │    │  (ratatui)     │  │
│  └────────────┘    └──────┬───────┘    └────────────────┘  │
│                            │                                 │
│                            │ reads                           │
│                            ↓                                 │
│                    ┌───────────────┐                         │
│                    │ Compact Table │                         │
│                    │    Storage    │                         │
│                    └───────┬───────┘                         │
│                            ↑                                 │
│                            │ appends via channel             │
└────────────────────────────┼─────────────────────────────────┘
                             │
                    ┌────────┴────────┐
                    │  mpsc::channel  │
                    └────────┬────────┘
                             │
┌────────────────────────────┼─────────────────────────────────┐
│                   Background Thread                          │
│                            │                                 │
│                    ┌───────▼───────┐                         │
│                    │ Stdin Parser  │                         │
│                    │   (psql fmt)  │                         │
│                    └───────────────┘                         │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow Architecture

**Phase 1: Initialization**
```rust
// In main.rs
fn main() -> Result<()> {
    // Create channel for parser → UI communication
    let (tx, rx) = mpsc::channel();

    // Spawn background parser thread
    let parser_handle = thread::spawn(move || {
        parse_stdin(tx);
    });

    // Initialize TUI with empty/partial data
    let mut app = App::new(rx);

    // Enter event loop (non-blocking)
    run_tui(&mut app)?;

    parser_handle.join()?;
    Ok(())
}
```

**Phase 2: Background Parsing**
```rust
// Parser thread sends batches
enum ParserMessage {
    Header(Vec<String>),
    RowBatch(Vec<CompactRow>),  // Batch for efficiency
    Complete,
    Error(String),
}

fn parse_stdin(tx: mpsc::Sender<ParserMessage>) {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());

    // Parse header, send immediately
    let header = parse_header(&mut reader)?;
    tx.send(ParserMessage::Header(header))?;

    // Parse rows in batches
    let mut batch = Vec::with_capacity(1000);
    for line in reader.lines() {
        let row = parse_row(line?);
        batch.push(row);

        if batch.len() >= 1000 {
            tx.send(ParserMessage::RowBatch(batch))?;
            batch = Vec::with_capacity(1000);
        }
    }

    // Send remaining + completion signal
    if !batch.is_empty() {
        tx.send(ParserMessage::RowBatch(batch))?;
    }
    tx.send(ParserMessage::Complete)?;
}
```

**Phase 3: Main Loop Updates**
```rust
// In main.rs app loop
fn run_tui(app: &mut App) -> Result<()> {
    let mut terminal = setup_terminal()?;

    loop {
        // Non-blocking check for new data
        while let Ok(msg) = app.data_rx.try_recv() {
            app.process_parser_message(msg);
        }

        // Handle input events (crossterm::poll with timeout)
        if crossterm::event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }

        // Render current state
        terminal.draw(|f| app.render(f))?;

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
```

## Memory-Efficient Storage Options

### Option A: String Interning (Recommended for tables with repeated values)

**Use case:** Tables where column values repeat frequently (categories, status codes, IDs)

```rust
use string_interner::{StringInterner, DefaultSymbol};

struct InternedTable {
    interner: StringInterner,
    header: Vec<String>,
    rows: Vec<Vec<DefaultSymbol>>,  // u32 instead of String
}

impl InternedTable {
    fn add_row(&mut self, cells: Vec<String>) -> Vec<DefaultSymbol> {
        cells.into_iter()
            .map(|s| self.interner.get_or_intern(s))
            .collect()
    }

    fn get_row(&self, idx: usize) -> Vec<&str> {
        self.rows[idx]
            .iter()
            .map(|sym| self.interner.resolve(*sym).unwrap())
            .collect()
    }
}
```

**Memory savings:** 50-80% for tables with high repetition
**Performance:** O(1) string comparison via u32 equality
**Trade-off:** Lookup overhead when rendering (minimal, amortized by rendering only visible rows)

### Option B: Compact Row Storage (Recommended for low repetition)

```rust
use compact_str::CompactString;

struct CompactTable {
    header: Vec<String>,
    rows: Vec<Vec<CompactString>>,  // 24 bytes → inline for ≤23 chars
}
```

**Memory savings:** 30-50% for tables with many short strings (<24 chars)
**Performance:** No lookup overhead, direct access
**Trade-off:** Less savings than interning for repeated values

### Option C: Apache Arrow (Recommended for analytics/exports)

```rust
use arrow::array::{ArrayRef, StringArray};
use arrow::record_batch::RecordBatch;

struct ArrowTable {
    batches: Vec<RecordBatch>,  // Columnar storage
}
```

**Memory savings:** 60-90% via columnar compression
**Performance:** Excellent for filtering, aggregations, exports
**Trade-off:** Higher complexity, row access slower than columnar operations

**Recommendation:** Start with **Option A (String Interning)** for typical psql output tables. Measure memory usage and switch to Arrow if handling truly massive datasets or planning analytics features.

## Virtualized Rendering

Ratatui's Table widget already supports efficient rendering via TableState's offset mechanism. No custom virtualization needed.

### How Ratatui Optimizes Rendering

1. **Immediate-mode with buffer diffing**: Each frame renders to an intermediate buffer, then diffs against previous frame, sending only changes to terminal
2. **TableState offset tracking**: Automatically adjusts scroll offset to keep selected row visible
3. **Only visible rows rendered**: Table widget only iterates rows that fit in the viewport

### Integration with Storage

```rust
impl App {
    fn render_table(&self, frame: &mut Frame, area: Rect) {
        let tab = &self.tabs[self.current_tab];

        // Calculate visible row range from TableState offset
        let offset = tab.table_state.offset();
        let height = area.height.saturating_sub(2) as usize; // minus header + border
        let visible_rows = offset..offset.saturating_add(height);

        // Only materialize visible rows (key optimization)
        let rows: Vec<Row> = tab.storage
            .get_range(visible_rows)  // Iterator, not Vec
            .map(|cells| Row::new(cells))
            .collect();

        let table = Table::new(rows, tab.column_widths.clone())
            .header(Row::new(tab.header.clone()))
            .highlight_style(Style::default().bg(Color::Blue));

        frame.render_stateful_widget(table, area, &mut tab.table_state);
    }
}
```

**Key point:** `get_range()` returns an iterator that resolves strings lazily (for interned storage) or clones compactly (for CompactString). Ratatui's Table accepts any iterator convertible to Rows.

## Component Design

### New: `ParserMessage` Enum (in state.rs)

```rust
pub enum ParserMessage {
    Header(Vec<String>),
    RowBatch(Vec<CompactRow>),
    LoadingProgress { rows_loaded: usize, elapsed: Duration },
    Complete { total_rows: usize, elapsed: Duration },
    Error(String),
}

pub type CompactRow = Vec<CompactString>;  // or Vec<DefaultSymbol> for interning
```

### New: `TableStorage` Trait (in state.rs)

```rust
pub trait TableStorage {
    fn add_batch(&mut self, rows: Vec<CompactRow>);
    fn get_range(&self, range: Range<usize>) -> Box<dyn Iterator<Item = Vec<&str>> + '_>;
    fn len(&self) -> usize;
    fn memory_usage(&self) -> usize;
}

// Implementation for interned storage
pub struct InternedStorage {
    interner: StringInterner,
    rows: Vec<Vec<DefaultSymbol>>,
}

impl TableStorage for InternedStorage {
    fn add_batch(&mut self, rows: Vec<CompactRow>) {
        for row in rows {
            let interned: Vec<_> = row.into_iter()
                .map(|s| self.interner.get_or_intern(s.as_str()))
                .collect();
            self.rows.push(interned);
        }
    }

    fn get_range(&self, range: Range<usize>) -> Box<dyn Iterator<Item = Vec<&str>> + '_> {
        Box::new(
            self.rows[range]
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|sym| self.interner.resolve(*sym).unwrap())
                        .collect()
                })
        )
    }

    fn len(&self) -> usize {
        self.rows.len()
    }

    fn memory_usage(&self) -> usize {
        std::mem::size_of_val(&self.rows) +
        self.rows.capacity() * std::mem::size_of::<Vec<DefaultSymbol>>() +
        self.interner.capacity() * std::mem::size_of::<String>()
    }
}
```

### Modified: `Tab` Struct (in state.rs)

```rust
pub struct Tab {
    pub name: String,
    pub header: Vec<String>,
    pub storage: Box<dyn TableStorage>,  // Replace Vec<Vec<String>>
    pub table_state: TableState,
    pub loading_complete: bool,
    pub total_rows: usize,
}
```

### Modified: `App` Struct (in state.rs)

```rust
pub struct App {
    pub tabs: Vec<Tab>,
    pub current_tab: usize,
    pub should_quit: bool,
    pub data_rx: mpsc::Receiver<ParserMessage>,  // NEW: receive parser messages
    pub loading_state: LoadingState,  // NEW: track background loading
}

pub struct LoadingState {
    pub is_loading: bool,
    pub rows_loaded: usize,
    pub start_time: Instant,
}

impl App {
    pub fn process_parser_message(&mut self, msg: ParserMessage) {
        match msg {
            ParserMessage::Header(header) => {
                let tab = Tab::new_with_header(header);
                self.tabs.push(tab);
            }
            ParserMessage::RowBatch(rows) => {
                let tab = &mut self.tabs[self.current_tab];
                tab.storage.add_batch(rows);
                self.loading_state.rows_loaded = tab.storage.len();
            }
            ParserMessage::LoadingProgress { rows_loaded, .. } => {
                self.loading_state.rows_loaded = rows_loaded;
            }
            ParserMessage::Complete { total_rows, elapsed } => {
                self.loading_state.is_loading = false;
                self.tabs[self.current_tab].loading_complete = true;
                self.tabs[self.current_tab].total_rows = total_rows;
            }
            ParserMessage::Error(err) => {
                // Handle error (show in status line or error modal)
            }
        }
    }
}
```

### Modified: main.rs (split into modules)

**New function: `spawn_parser_thread()`**
```rust
fn spawn_parser_thread() -> (thread::JoinHandle<Result<()>>, mpsc::Receiver<ParserMessage>) {
    let (tx, rx) = mpsc::channel();

    let handle = thread::spawn(move || {
        parse_stdin_to_channel(tx)
    });

    (handle, rx)
}

fn parse_stdin_to_channel(tx: mpsc::Sender<ParserMessage>) -> Result<()> {
    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());

    // Existing psql parsing logic, but sends via channel
    // ... (reuse existing parse logic from main.rs)

    Ok(())
}
```

**Modified: main()**
```rust
fn main() -> Result<()> {
    // Spawn parser in background
    let (parser_handle, data_rx) = spawn_parser_thread();

    // Initialize app with channel receiver
    let mut app = App::new(data_rx);

    // Run TUI (non-blocking, processes messages as they arrive)
    run_tui(&mut app)?;

    // Wait for parser to finish
    parser_handle.join().unwrap()?;

    Ok(())
}
```

## Rendering Optimizations

### Frame Rate Control

Ratatui achieves sub-millisecond rendering through buffer diffing. For large tables, frame rate control prevents unnecessary redraws.

```rust
const TARGET_FPS: u64 = 30;
const FRAME_DURATION: Duration = Duration::from_millis(1000 / TARGET_FPS);

fn run_tui(app: &mut App) -> Result<()> {
    let mut terminal = setup_terminal()?;
    let mut last_frame = Instant::now();

    loop {
        // Process data messages (non-blocking)
        while let Ok(msg) = app.data_rx.try_recv() {
            app.process_parser_message(msg);
        }

        // Poll for input with short timeout
        if crossterm::event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }

        // Render at controlled rate
        let now = Instant::now();
        if now.duration_since(last_frame) >= FRAME_DURATION {
            terminal.draw(|f| app.render(f))?;
            last_frame = now;
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
```

**Trade-off:** 30 FPS is imperceptible to users and reduces CPU usage by 50% compared to uncapped rendering.

### Loading Indicator

Show progress while data streams in:

```rust
fn render_status_line(&self, frame: &mut Frame, area: Rect) {
    let status = if self.loading_state.is_loading {
        let elapsed = self.loading_state.start_time.elapsed().as_secs();
        format!(
            "Loading... {} rows in {}s | Press q to quit",
            self.loading_state.rows_loaded,
            elapsed
        )
    } else {
        format!(
            "{} rows | Tab {}/{} | Press q to quit",
            self.tabs[self.current_tab].total_rows,
            self.current_tab + 1,
            self.tabs.len()
        )
    };

    let paragraph = Paragraph::new(status)
        .style(Style::default().fg(Color::Cyan));

    frame.render_widget(paragraph, area);
}
```

## Build Order and Dependencies

### Phase 1: Thread-Safe Data Pipeline (Foundation)

**Why first:** Unblocks UI immediately, enables progressive rendering

1. Add dependency: `string-interner = "0.17"` (or `compact_str = "0.8"`)
2. Create `ParserMessage` enum in `state.rs`
3. Add `data_rx: mpsc::Receiver<ParserMessage>` to `App`
4. Extract stdin parsing logic into `parse_stdin_to_channel()` function
5. Modify `main()` to spawn parser thread and create channel
6. Update `App::new()` to accept receiver
7. Add `App::process_parser_message()` method (initially just stores to Vec<Vec<String>>)
8. Modify event loop to call `try_recv()` each iteration

**Testing:** Verify UI becomes interactive immediately, table populates progressively

### Phase 2: Compact Storage (Memory Optimization)

**Why second:** Builds on working pipeline, testable incrementally

9. Create `TableStorage` trait in `state.rs`
10. Implement `InternedStorage` (or `CompactStorage`)
11. Replace `Tab.rows: Vec<Vec<String>>` with `Tab.storage: Box<dyn TableStorage>`
12. Update `process_parser_message()` to call `storage.add_batch()`
13. Modify render logic to use `storage.get_range()` instead of direct Vec access
14. Update export logic to iterate via `storage.get_range()`
15. Update search logic to iterate via `storage.get_range()`

**Testing:** Compare memory usage before/after with `storage.memory_usage()`, verify rendering unchanged

### Phase 3: Virtualized Rendering (Performance Polish)

**Why third:** Optimizes what's already working, safe to skip if perf acceptable

16. Add frame rate limiting with `TARGET_FPS` constant
17. Add `LoadingState` struct to track progress
18. Implement loading indicator in status line
19. Optimize `get_range()` to return iterator (not Vec) if not already
20. Profile rendering with 1.8M rows, verify <16ms frame times

**Testing:** Benchmark scroll performance, CPU usage, frame rate stability

## Performance Expectations

| Metric | Current (Synchronous) | After Phase 1 (Threaded) | After Phase 2 (Compact Storage) | After Phase 3 (Virtualized) |
|--------|----------------------|-------------------------|--------------------------------|----------------------------|
| **Startup time** | 2-5 minutes (blocking) | <100ms (immediate) | <100ms | <100ms |
| **Memory usage** | ~2GB (1.8M rows) | ~2GB | ~400MB-1GB | ~400MB-1GB |
| **Scroll latency** | 50-200ms | 50-200ms | 10-30ms | <16ms (60 FPS) |
| **Frame rate** | Varies | Varies | Varies | Stable 30-60 FPS |

## Patterns to Follow

### Pattern 1: Progressive Disclosure

**What:** Show data as soon as available, don't wait for completion
**When:** Any operation taking >500ms
**Example:**
```rust
// Bad: Wait for all data
let all_rows = parse_all_stdin();
app.set_rows(all_rows);
app.run();

// Good: Stream data
spawn_parser_thread(tx);
app.run();  // Already running, receives data via channel
```

### Pattern 2: Batch Processing

**What:** Send data in chunks (1000 rows) instead of individually
**When:** High-frequency updates from background thread
**Example:**
```rust
// Bad: Send each row individually (channel overhead)
for row in rows {
    tx.send(ParserMessage::Row(row))?;
}

// Good: Batch into groups
const BATCH_SIZE: usize = 1000;
for batch in rows.chunks(BATCH_SIZE) {
    tx.send(ParserMessage::RowBatch(batch.to_vec()))?;
}
```

### Pattern 3: Lazy Materialization

**What:** Convert compact representation to strings only when rendering
**When:** Using interned or compressed storage
**Example:**
```rust
// Good: Iterator resolves symbols only for visible rows
fn get_range(&self, range: Range<usize>) -> impl Iterator<Item = Vec<&str>> + '_ {
    self.rows[range].iter().map(|row| {
        row.iter()
            .map(|sym| self.interner.resolve(*sym).unwrap())
            .collect()
    })
}
```

### Pattern 4: Non-Blocking Channel Reads

**What:** Use `try_recv()` instead of `recv()` in render loop
**When:** Reading from channels in event loop
**Example:**
```rust
// Bad: Blocks render loop
if let Ok(msg) = app.data_rx.recv() {
    app.process(msg);
}

// Good: Non-blocking, processes all available messages
while let Ok(msg) = app.data_rx.try_recv() {
    app.process(msg);
}
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Shared Mutable State with Arc<Mutex<Vec>>

**What:** Sharing data between parser and renderer via Arc<Mutex<Vec<T>>>
**Why bad:** Lock contention every frame, parser blocks renderer
**Instead:** Use mpsc channels for one-way communication (parser → renderer)

```rust
// Bad: Lock contention
let rows = Arc::new(Mutex::new(Vec::new()));
let rows_clone = rows.clone();

thread::spawn(move || {
    for row in parse_stdin() {
        rows_clone.lock().unwrap().push(row);  // Blocks renderer
    }
});

// In render loop
let rows = rows.lock().unwrap();  // Blocks parser
render_table(&rows);

// Good: Channel-based communication
let (tx, rx) = mpsc::channel();

thread::spawn(move || {
    for row in parse_stdin() {
        tx.send(row).unwrap();  // Non-blocking
    }
});

// In render loop
while let Ok(row) = rx.try_recv() {  // Non-blocking
    app.add_row(row);
}
```

### Anti-Pattern 2: Pre-Allocating Full Dataset

**What:** Allocating Vec with capacity for all rows before parsing
**Why bad:** Requires knowing row count in advance, wastes memory if estimate wrong
**Instead:** Use Vec::with_capacity for batches, let Vec grow dynamically

```rust
// Bad: Guess total size
let mut rows = Vec::with_capacity(2_000_000);  // What if there are 10M rows?

// Good: Reserve capacity per batch
const BATCH_SIZE: usize = 1000;
let mut batch = Vec::with_capacity(BATCH_SIZE);
```

### Anti-Pattern 3: Cloning Full Rows for Rendering

**What:** Cloning Vec<Vec<String>> to pass to render function
**Why bad:** Expensive for large datasets, defeats purpose of compact storage
**Instead:** Use iterators and references

```rust
// Bad: Clones all data
fn render(&self) {
    let rows = self.storage.rows.clone();  // Expensive!
    Table::new(rows)
}

// Good: Iterator with references
fn render(&self) {
    let rows = self.storage.get_range(visible_range);  // Iterator of &str
    Table::new(rows)
}
```

### Anti-Pattern 4: Rendering All Rows Every Frame

**What:** Passing all rows to Table widget even if only 30 visible
**Why bad:** Wastes CPU converting compact storage to displayable format
**Instead:** Calculate visible range and only materialize those rows

```rust
// Bad: Materializes all rows
let rows: Vec<_> = (0..storage.len())
    .map(|i| storage.get_row(i))
    .collect();

// Good: Only visible rows
let offset = state.offset();
let height = area.height as usize;
let rows: Vec<_> = storage
    .get_range(offset..offset + height)
    .collect();
```

## Integration with Existing Features

### Export Functionality

Export must iterate through storage efficiently:

```rust
fn export_to_csv(&self, path: &Path) -> Result<()> {
    let mut writer = csv::Writer::from_path(path)?;

    // Write header
    writer.write_record(&self.header)?;

    // Iterate through all rows (use batching for memory efficiency)
    const EXPORT_BATCH_SIZE: usize = 10_000;
    let total_rows = self.storage.len();

    for batch_start in (0..total_rows).step_by(EXPORT_BATCH_SIZE) {
        let batch_end = (batch_start + EXPORT_BATCH_SIZE).min(total_rows);

        for row in self.storage.get_range(batch_start..batch_end) {
            writer.write_record(&row)?;
        }
    }

    Ok(())
}
```

### Search Functionality

Search must handle streaming data (search while loading):

```rust
fn search(&self, query: &str) -> Vec<usize> {
    let mut matches = Vec::new();

    // Search through currently loaded rows
    let total = self.storage.len();

    for (idx, row) in self.storage.get_range(0..total).enumerate() {
        if row.iter().any(|cell| cell.contains(query)) {
            matches.push(idx);
        }
    }

    matches
}
```

**Note:** For 1.8M rows, linear search will be slow. Consider adding an index in a later milestone.

### Column Resizing

Width calculation must handle compact storage:

```rust
fn calculate_column_widths(&self) -> Vec<u16> {
    let mut widths = vec![0; self.header.len()];

    // Check header widths
    for (i, col) in self.header.iter().enumerate() {
        widths[i] = col.len() as u16;
    }

    // Sample first 1000 rows (don't scan all 1.8M)
    const SAMPLE_SIZE: usize = 1000;
    let sample_end = SAMPLE_SIZE.min(self.storage.len());

    for row in self.storage.get_range(0..sample_end) {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.len() as u16);
        }
    }

    widths
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interned_storage_memory_efficiency() {
        let mut storage = InternedStorage::new();

        // Add 1000 rows with repeated values
        for _ in 0..1000 {
            storage.add_batch(vec![
                vec!["ACTIVE".into(), "user123".into()],
                vec!["INACTIVE".into(), "user456".into()],
            ]);
        }

        // String interning should dedupe "ACTIVE", "INACTIVE"
        // Memory usage should be << 1000 * 2 * (String size)
        let memory = storage.memory_usage();

        // Baseline: 1000 rows * 2 cols * ~50 bytes = ~100KB
        // With interning: ~50 bytes (2 unique strings) + 1000 * 2 * 4 (symbols) = ~8KB
        assert!(memory < 20_000, "Expected <20KB, got {}", memory);
    }

    #[test]
    fn test_batch_processing() {
        let (tx, rx) = mpsc::channel();

        // Send batches
        thread::spawn(move || {
            for i in 0..10 {
                let batch: Vec<_> = (0..100)
                    .map(|j| vec![format!("row_{}", i * 100 + j)])
                    .collect();
                tx.send(ParserMessage::RowBatch(batch)).unwrap();
            }
            tx.send(ParserMessage::Complete {
                total_rows: 1000,
                elapsed: Duration::from_secs(1)
            }).unwrap();
        });

        let mut app = App::new(rx);

        // Process all messages
        while let Ok(msg) = app.data_rx.recv() {
            app.process_parser_message(msg);
            if !app.loading_state.is_loading {
                break;
            }
        }

        assert_eq!(app.tabs[0].storage.len(), 1000);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_progressive_loading_ui() {
    let (tx, rx) = mpsc::channel();
    let mut app = App::new(rx);

    // Simulate slow data arrival
    thread::spawn(move || {
        tx.send(ParserMessage::Header(vec!["col1".into()])).unwrap();

        for i in 0..10 {
            thread::sleep(Duration::from_millis(100));
            let batch = vec![vec![format!("row{}", i)]];
            tx.send(ParserMessage::RowBatch(batch)).unwrap();
        }

        tx.send(ParserMessage::Complete {
            total_rows: 10,
            elapsed: Duration::from_secs(1)
        }).unwrap();
    });

    // Verify UI is interactive while loading
    assert!(app.tabs.is_empty());  // No data yet

    // Process first message (header)
    let msg = app.data_rx.recv().unwrap();
    app.process_parser_message(msg);
    assert_eq!(app.tabs.len(), 1);

    // Process first batch
    let msg = app.data_rx.recv().unwrap();
    app.process_parser_message(msg);
    assert_eq!(app.tabs[0].storage.len(), 1);
}
```

### Performance Benchmarks

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_render_visible_rows(c: &mut Criterion) {
    let mut storage = InternedStorage::new();

    // Add 1M rows
    for _ in 0..10_000 {
        let batch: Vec<_> = (0..100)
            .map(|i| vec![format!("cell_{}", i)])
            .collect();
        storage.add_batch(batch);
    }

    c.bench_function("render_30_visible_rows", |b| {
        b.iter(|| {
            let rows: Vec<_> = storage
                .get_range(black_box(0..30))
                .collect();
            black_box(rows);
        });
    });
}

criterion_group!(benches, bench_render_visible_rows);
criterion_main!(benches);
```

## Alternative Approaches Considered

### Async/Await with Tokio

**Considered:** Using tokio runtime with async stdin reading
**Pros:** More ergonomic async code, better for multiple concurrent I/O sources
**Cons:**
- Adds significant dependency weight (tokio runtime)
- Overkill for single stdin → UI data flow
- Crossterm already provides event::poll for non-blocking input
- Spawning native threads simpler for this use case

**Decision:** Use native threads with mpsc channels. Simple, no extra dependencies, proven pattern.

### Memory-Mapped Files

**Considered:** Writing stdin to temp file, then mmap for random access
**Pros:** No memory limit, OS handles paging
**Cons:**
- Requires disk I/O (slower than RAM for table viewer use case)
- Adds complexity (temp file cleanup, mmap unsafe code)
- Doesn't eliminate need to parse psql format
- Export/search still need to read full dataset

**Decision:** Use in-memory compact storage. 1.8M rows fits in RAM with interning (~500MB), faster access.

### SQLite Backend

**Considered:** Storing rows in SQLite database for querying
**Pros:** Indexed search, SQL queries, no memory limit
**Cons:**
- Massive scope increase (SQL query parsing, DB schema design)
- Slower random access than in-memory structures
- Overkill for simple table viewer
- Defeats purpose of "explore psql output quickly"

**Decision:** Defer to future milestone if advanced querying needed. Current scope: fast viewing, not analysis.

## Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
# Existing
ratatui = "0.29"
crossterm = "0.28"

# New for Phase 2
string-interner = "0.17"  # For interned storage
# OR
compact_str = "0.8"  # For compact string storage

# Optional for benchmarking
[dev-dependencies]
criterion = "0.5"
```

## Confidence Assessment

| Area | Confidence | Evidence |
|------|-----------|----------|
| Thread architecture | HIGH | Standard Rust mpsc pattern, documented in crossterm TUI examples, real-world usage in r3bl_tui |
| Storage optimization | HIGH | String interning proven in rustc, article shows 2000x compression, compact_str benchmarks published |
| Ratatui rendering | HIGH | Official docs confirm TableState offset, buffer diffing, iterator support for rows |
| Performance targets | MEDIUM | Based on reported issue (15K rows laggy), extrapolated to 1.8M; actual perf depends on terminal, data shape |

## Open Questions for Implementation

1. **Batch size tuning**: 1000 rows per batch is estimate. Profile to find optimal (balance latency vs throughput).
2. **Error handling**: How to show parser errors in UI? Modal? Status line? Log file?
3. **Interning vs compact_str**: Run benchmarks on representative psql data to choose. High repetition → interning, low repetition → compact_str.
4. **Column width sampling**: Is 1000 rows sufficient? Or scan all if <10K total rows?
5. **Search index**: Phase 3 only optimizes rendering. 1.8M row linear search will be slow. Flag for future milestone?

## Sources

**Ratatui Architecture & Performance:**
- [Ratatui Official Site](https://ratatui.rs/) - Sub-millisecond rendering, immediate-mode architecture
- [Ratatui Rendering Concepts](https://ratatui.rs/concepts/rendering/) - Buffer diffing explanation
- [Ratatui Table Widget Documentation](https://docs.rs/ratatui/latest/ratatui/widgets/struct.Table.html) - TableState offset mechanism
- [Ratatui Rendering Under the Hood](https://ratatui.rs/concepts/rendering/under-the-hood/) - Double buffer optimization
- [Table Performance Issue #1004](https://github.com/ratatui/ratatui/issues/1004) - 15K rows laggy, conversion to vec bottleneck
- [Ratatui Best Practices Discussion](https://github.com/ratatui/ratatui/discussions/579) - Frame rate control, rendering optimization

**Threading & Concurrency Patterns:**
- [Creating a TUI in Rust](https://raysuliteanu.medium.com/creating-a-tui-in-rust-e284d31983b3) - Background threads with mpsc channels
- [Improving spotify-tui: Going Async](https://keliris.dev/articles/improving-spotify-tui) - Event loop with channels, non-blocking input
- [Rust std::sync::mpsc Documentation](https://doc.rust-lang.org/std/sync/mpsc/index.html) - Multi-producer single-consumer channels
- [Tokio mpsc Channels](https://tokio.rs/tokio/tutorial/channels) - Async channel patterns
- [Crossterm Event Polling](https://docs.rs/crossterm/latest/crossterm/event/fn.poll.html) - Non-blocking event reads

**Memory Optimization:**
- [Performance Optimization in Rust: String Interning](https://medium.com/@zhiweiwang2001/performance-optimization-in-rust-understanding-string-interning-symbol-and-smartstring-6ec80b8c781a) - Symbol approach, O(1) comparison
- [Fast and Simple Rust Interner](https://matklad.github.io/2020/03/22/fast-simple-rust-interner.html) - Rustc approach, u32 symbols
- [The Power of Interning: 2000x Smaller](https://gendignoux.com/blog/2025/03/03/rust-interning-2000x.html) - Real-world memory savings
- [string-interner Crate](https://docs.rs/string-interner) - Constant time comparisons, minimal footprint
- [compact_str Documentation](https://docs.rs/compact_str/latest/compact_str/) - Small string optimization, inline storage
- [String Interners in Rust](https://dev.to/cad97/string-interners-in-rust-797) - Comparison of interner crates
- [Fun with Benchmarking Small String Optimization](https://swatinem.de/blog/smallstring-opt/) - CompactString performance

**Rust Data Structures & Performance:**
- [Rust Vec Documentation](https://doc.rust-lang.org/std/vec/struct.Vec.html) - Capacity, reserve, growth strategy
- [Rust Performance Book: Heap Allocations](https://nnethercote.github.io/perf-book/heap-allocations.html) - Reserve strategies
- [Mastering Rust Arc and Mutex](https://medium.com/@Murtza/mastering-rust-arc-and-mutex-a-comprehensive-guide-to-safe-shared-state-in-concurrent-programming-1913cd17e08d) - Shared state patterns
- [Arc Mutex Performance Optimization](https://leapcell.medium.com/even-faster-multithreading-in-rust-arc-optimization-54a5f4b0660f) - Lock contention, performance tips

**Apache Arrow (Alternative):**
- [Apache Arrow Rust](https://arrow.apache.org/rust/arrow/index.html) - Columnar format
- [Apache Arrow Performance 2025](https://arrow.apache.org/blog/2025/10/23/rust-parquet-metadata/) - 4x faster metadata parsing
- [Fast Multi-Column Sorts](https://arrow.apache.org/blog/2022/11/07/multi-column-sorts-in-arrow-rust-part-2/) - Memory efficiency

**Progressive Loading Examples:**
- [xleak: Terminal Excel Viewer](https://github.com/bgreenwell/xleak) - Lazy loading, row caching for 1000+ row files
- [r3bl_tui Async Architecture](https://docs.rs/r3bl_tui/latest/r3bl_tui/) - Tokio concurrency, async spinners
- [Ratatui FAQ](https://ratatui.rs/faq/) - Immediate-mode rendering patterns
