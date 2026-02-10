---
phase: 14-profiling-infrastructure
plan: 03
subsystem: testing-infrastructure
tags: [integration-tests, regression-coverage, search, export, column-operations]
completed: 2026-02-10

dependencies:
  requires:
    - 14-01 (library crate for external test access)
  provides:
    - integration-test-coverage
    - regression-protection
    - search-test-suite
    - export-test-suite
    - column-test-suite
  affects:
    - phase-16-storage-refactoring
    - data-integrity-verification
    - column-visibility-export-integration

tech-stack:
  added: []
  patterns:
    - "Integration tests import library crate modules for testing"
    - "Tests verify cross-module interactions (column config -> export)"
    - "Test data uses realistic psql-formatted samples"
    - "Tests cover edge cases: empty input, special characters, boundaries"

key-files:
  created:
    - tests/search_tests.rs: "10 tests for search/filter functionality and parsing"
    - tests/export_tests.rs: "9 tests for CSV and JSON export with column visibility"
    - tests/column_tests.rs: "14 tests for column hide/show/reorder/resize operations"
  modified: []

decisions:
  - what: "Remove unused TableData import from search_tests.rs"
    why: "Import was not directly used in tests, causing compiler warning"
    impact: "Clean compilation without warnings"
  - what: "Include cross-module integration test in column_tests.rs"
    why: "Phase 16 storage changes could break column visibility -> export wiring"
    impact: "test_column_visibility_with_export validates critical integration point"

metrics:
  duration: 180
  tasks_completed: 2
  files_created: 3
  files_modified: 0
  commits: 2
  deviations: 1
---

# Phase 14 Plan 03: Integration Tests for Regression Protection

**One-liner:** Comprehensive integration tests for search filtering, CSV/JSON export, and column operations to prevent regressions during Phase 16 storage refactoring.

## Overview

This plan creates a robust safety net of 33 integration tests (10 search + 9 export + 14 column) covering the three critical areas identified as highest risk for Phase 16's storage refactoring. When the core data structure changes from `Vec<Vec<String>>` to a compact format, these tests ensure search filtering, export functionality, and column operations continue working correctly.

## Execution Summary

**Status:** ✅ Complete
**Duration:** 3.0 minutes
**Commits:** 2 task commits

### Tasks Completed

| Task | Name                                              | Commit  | Files                                   |
| ---- | ------------------------------------------------- | ------- | --------------------------------------- |
| 1    | Create search and export integration tests        | ab8cadf | tests/search_tests.rs, tests/export_tests.rs |
| 2    | Create column operation integration tests         | db0e699 | tests/column_tests.rs                   |

### Deviations from Plan

**1. [Rule 3 - Blocking] Removed unused import**
- **Found during:** Task 1 compilation
- **Issue:** `TableData` imported but not directly used in search_tests.rs, causing unused import warning
- **Fix:** Removed `TableData` from import statement
- **Files modified:** tests/search_tests.rs
- **Commit:** ab8cadf (after initial test creation)

## Technical Implementation

### Test Suite Organization

**tests/search_tests.rs (10 tests):**
- Parse and filter integration: Validates parsing + filtering pipeline
- Case-insensitive matching: Ensures lowercase search matches uppercase data
- No matches: Verifies empty result set for non-matching filters
- Empty filter: Confirms no filtering when filter_text is empty
- Partial matches: Tests substring matching (e.g., "ali" matches "Alice")
- Parse edge cases: Empty input, single row, multiline psql with footer
- Cross-column filtering: Validates search across any column
- Number filtering: Tests filtering on numeric values

**tests/export_tests.rs (9 tests):**
- CSV all columns: Verifies UTF-8 BOM, headers, and data rows
- CSV column subset: Tests export with column visibility filtering
- JSON all columns: Validates JSON array structure and object keys
- JSON column subset: Ensures only visible columns in JSON output
- Special characters: Tests CSV escaping (commas, quotes, newlines)
- Empty table: Verifies header-only CSV and empty JSON array
- Column reordering: Tests export respects visible_cols order
- Single column: Edge case for minimal visibility
- JSON structure preservation: Validates object key-value integrity

**tests/column_tests.rs (14 tests):**
- Basic operations: new, hide, show_all, reset
- Width management: adjust_width with min/max bounds, auto-sizing
- Reordering: swap_display and display_position tracking
- Order preservation: Verifies hide/show maintains column order
- Cross-module integration: **Critical test** - column visibility -> export pipeline
- Compound operations: Multiple operations preserve state correctly
- Edge cases: Out-of-bounds hide, resize clamping
- Visibility + reorder: Integration of two operations

### Critical Integration Test

The most important test is `test_column_visibility_with_export` in column_tests.rs:

```rust
#[test]
fn test_column_visibility_with_export() {
    let data = TableData { ... };
    let mut config = ColumnConfig::new(4);
    config.hide(2); // Hide age column

    let visible_indices = config.visible_indices();

    // Export CSV and JSON with visibility filter
    let csv_result = export_table(&data, &visible_indices, ExportFormat::Csv)?;
    let json_result = export_table(&data, &visible_indices, ExportFormat::Json)?;

    // Verify hidden column excluded from both formats
    assert!(!csv_result.contains("age"));
    assert!(!parsed_json[0].contains_key("age"));
}
```

This test validates the exact wiring that Phase 16 storage changes could break:
1. ColumnConfig tracks visibility
2. visible_indices() generates column list
3. export_table() filters based on that list
4. Output excludes hidden columns

If Phase 16 breaks this chain, this test fails immediately.

### Test Data Patterns

**Realistic psql format:**
```
 id | name    | age | city
----+---------+-----+-------------
 1  | Alice   | 30  | New York
 ...
(10 rows)
```

**Varied content for search testing:**
- Mixed case names (Alice, alice) for case-insensitivity tests
- Numbers and text in different columns
- Realistic city names for cross-column filtering

**Special characters for export testing:**
- Commas in values
- Quotes in values
- Newlines in values

## Verification Results

All verification checks passed:

- ✅ `cargo test --test search_tests` passes (10 tests)
- ✅ `cargo test --test export_tests` passes (9 tests)
- ✅ `cargo test --test column_tests` passes (14 tests)
- ✅ `cargo test` passes all tests (64 total: 31 unit + 33 integration)
- ✅ No compiler warnings in test files
- ✅ Tests use library crate imports (pretty_table_explorer::*)

**Test summary:**
```
     Running unittests src/lib.rs
test result: ok. 31 passed

     Running tests/column_tests.rs
test result: ok. 14 passed

     Running tests/export_tests.rs
test result: ok. 9 passed

     Running tests/search_tests.rs
test result: ok. 10 passed
```

## Artifacts

**Key files created:**
- `tests/search_tests.rs` - 10 tests for search/filter and parsing
- `tests/export_tests.rs` - 9 tests for CSV/JSON export with column visibility
- `tests/column_tests.rs` - 14 tests for column operations and export integration

**Commits:**
- `ab8cadf` - Add search and export integration tests
- `db0e699` - Add column operation integration tests

## Downstream Impact

This plan enables:
- **Phase 16 (Memory Optimization):** Safe storage refactoring with regression coverage
  - Tests will fail immediately if compact storage breaks search, export, or columns
  - Provides confidence that optimization preserves functionality
- **Future refactoring:** Tests serve as living documentation of expected behavior
- **CI/CD integration:** Tests can be added to continuous integration pipeline

## Regression Coverage Analysis

**Phase 16 risks mitigated:**

| Risk                          | Coverage                                      | Tests                          |
| ----------------------------- | --------------------------------------------- | ------------------------------ |
| Search breaks on new storage  | Parse + filter pipeline tested end-to-end     | search_tests (10 tests)        |
| Export reads wrong data       | CSV/JSON generation verified with test data   | export_tests (9 tests)         |
| Column visibility fails       | Hide/show/reorder operations validated        | column_tests (14 tests)        |
| Column->export integration    | Critical cross-module test validates wiring   | test_column_visibility_with_export |

**Coverage statistics:**
- 33 integration tests
- 3 critical subsystems covered
- 1 explicit cross-module integration test
- Edge cases: empty input, special chars, boundaries, compound operations

## Self-Check: PASSED

Verified all claims:

```bash
# Files exist
✓ tests/search_tests.rs exists (10 tests)
✓ tests/export_tests.rs exists (9 tests)
✓ tests/column_tests.rs exists (14 tests)

# Commits exist
✓ ab8cadf: feat(14-03): add search and export integration tests
✓ db0e699: feat(14-03): add column operation integration tests

# Functionality
✓ cargo test passes (64 tests total: 31 unit + 33 integration)
✓ No compiler warnings
✓ Tests use library crate API
✓ Cross-module integration test present
```
