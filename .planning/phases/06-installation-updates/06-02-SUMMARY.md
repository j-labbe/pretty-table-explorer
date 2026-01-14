# Plan 06-02: Self-Update Command - Summary

## Objective
Add self-update command to pte so users can update to latest version with `pte update`.

## Tasks Completed

### Task 1: Add HTTP client dependency
- **Status:** Complete
- **Commit:** `0a5b90a`
- **Changes:**
  - Added `ureq = { version = "2", features = ["json"] }` for HTTP requests
  - Added `sha2 = "0.10"` for checksum verification
  - ureq chosen over reqwest for synchronous API (no async runtime), smaller binary size

### Task 2: Implement update subcommand
- **Status:** Complete
- **Commit:** `6e60cb3`
- **Changes:**
  - Added `Commands` enum with `Update` variant to CLI
  - Created `src/update.rs` module with full update logic:
    - Platform detection (linux/macos x86_64/aarch64)
    - GitHub API integration for checking latest release
    - Binary download with progress output
    - SHA256 checksum verification
    - Self-replacement with proper Unix permissions
  - Modified `parse_cli()` to return optional command
  - Added early return in `main()` for update command

## Files Modified
- `Cargo.toml` - Added ureq and sha2 dependencies
- `Cargo.lock` - Updated with new dependencies (47 new packages)
- `src/main.rs` - Added Commands enum, Subcommand derive, update handling in main()
- `src/update.rs` - New file with complete self-update implementation

## Verification Checklist
- [x] `cargo build --release` succeeds
- [x] `pte --help` shows update subcommand
- [x] `pte update` runs and attempts GitHub API call
- [x] Platform detection covers linux/macos x86_64/aarch64
- [x] Existing functionality preserved (pipe and --connect modes)
- [x] All 12 tests pass including new update module tests

## Implementation Details

### Update Flow
1. Detect current platform using `std::env::consts::{OS, ARCH}`
2. Fetch latest release from GitHub API
3. Compare versions (semver parsing)
4. Download checksums.txt and verify
5. Download platform-specific binary
6. Compute SHA256 hash and verify against expected
7. Write to temp file, set executable permissions
8. Atomic rename to replace current binary

### Configuration
The `GITHUB_REPO` constant is configurable for forks:
```rust
const GITHUB_REPO: &str = "yourusername/pretty-table-explorer";
```

### Error Handling
- Network errors display helpful messages
- Version comparison gracefully handles edge cases
- Checksum mismatch aborts with detailed error
- Unsupported platforms display clear error message

## Deviations
- None

## Issues
- None
