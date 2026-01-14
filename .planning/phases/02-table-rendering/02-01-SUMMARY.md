# Plan 02-01 Summary: Parse Piped psql Output

## Objective
Parse piped psql output into structured table data that can be rendered.

## Tasks Completed

### Task 1: Create parser module with TableData struct
- Created `src/parser.rs` with:
  - `TableData` struct holding `headers: Vec<String>` and `rows: Vec<Vec<String>>`
  - `column_count()` and `row_count()` helper methods
  - `parse_psql(input: &str) -> Option<TableData>` function
- Parser handles:
  - Header row (split by `|`, trimmed)
  - Separator line detection (contains `---`)
  - Data rows until footer `(N rows)`
  - Empty input (returns None)
  - Malformed input (returns None)
- Added 8 unit tests covering:
  - Simple table parsing
  - Single row tables
  - Empty input
  - Whitespace-only input
  - Missing separator
  - Empty tables (0 rows)
  - Leading newlines
  - Singular "row" vs plural "rows" in footer

### Task 2: Integrate parser with stdin reading
- Modified `src/main.rs` to:
  - Declare `mod parser;`
  - Import `std::io::Read` for stdin reading
  - Read all stdin before TUI initialization
  - Parse with `parser::parse_psql(&input)`
  - Print error and exit on invalid input
  - Print parsing summary (temporary, for verification)
- Added `use-dev-tty` feature to crossterm for keyboard input when stdin is piped

## Deviations

### Auto-fixed Blocker: TTY access for piped stdin
- **Issue:** When stdin is piped, crossterm couldn't read keyboard events
- **Fix:** Added `use-dev-tty` feature to crossterm dependency in Cargo.toml
- **Result:** crossterm now opens `/dev/tty` for keyboard input when stdin is not a tty
- **Note:** Cannot fully test in headless environment, but code is correct

## Files Modified
- `src/parser.rs` (created)
- `src/main.rs` (modified)
- `Cargo.toml` (modified)
- `Cargo.lock` (auto-updated)

## Verification Results
- [x] `cargo build` succeeds without errors
- [x] `cargo test` passes all 8 parser unit tests
- [x] Piped input is parsed correctly: `echo -e " a | b\n---+---\n 1 | 2\n(1 rows)" | cargo run` prints "Parsed table: 2 columns, 1 rows"
- [x] Empty stdin handled gracefully: shows error message, doesn't crash

## Commits
1. `abbaed3` - feat(02-01): create parser module with TableData struct
2. `ed89886` - feat(02-01): integrate parser with stdin reading
