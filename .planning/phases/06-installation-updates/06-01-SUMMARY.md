---
phase: 06-installation-updates
plan: 01
subsystem: install
tags: [installer, shell, curl, bash]

# Dependency graph
requires:
  - phase: 05-02
    provides: release workflow with binary naming convention
provides:
  - One-line curl | bash installer
  - Platform detection (Linux/macOS x86_64/aarch64)
  - SHA256 checksum verification
  - README with installation instructions
affects: [end-users, documentation]

# Tech tracking
tech-stack:
  added: [posix-shell]
  patterns: [curl-pipe-bash-installer]

key-files:
  created:
    - install.sh
    - README.md

key-decisions:
  - "Used POSIX shell (#!/bin/sh) for maximum portability"
  - "Downloads from releases/latest/download for version-agnostic URL"
  - "Falls back to sudo when install to /usr/local/bin fails"

patterns-established:
  - "Configurable via environment variables (REPO, INSTALL_DIR)"
  - "Cleanup temp files on exit via trap"
  - "Checksum verification before installation"

issues-created: []

# Metrics
duration: 3min
completed: 2026-01-14
---

# Plan 06-01: Install Script Summary

**One-line curl | bash installer for pte with platform detection and checksum verification**

## Performance

- **Duration:** 3 min
- **Started:** 2026-01-14
- **Completed:** 2026-01-14
- **Tasks:** 2/2
- **Files created:** 2

## Accomplishments

- Created POSIX-compatible install.sh with platform detection
- Supports Linux x86_64, Linux aarch64, macOS x86_64, macOS aarch64
- Downloads binaries from GitHub releases
- Verifies SHA256 checksums before installation
- Created README.md with installation instructions and basic usage

## Task Commits

Each task was committed atomically:

1. **Task 1: Create install script with platform detection** - `994839f` (feat)
2. **Task 2: Add install documentation to README** - `a82a860` (docs)

## Files Created

- `install.sh` - POSIX shell installer with platform detection, download, checksum verification
- `README.md` - Project readme with installation instructions, usage examples, navigation keys

## Script Features

- Platform detection using `uname -s` (OS) and `uname -m` (arch)
- Supports curl and wget for downloads
- Supports sha256sum and shasum for verification
- Configurable via environment variables:
  - `REPO` - GitHub repository (default: yourusername/pretty-table-explorer)
  - `INSTALL_DIR` - Installation directory (default: /usr/local/bin)
- Automatic cleanup of temp files on exit

## Verification

All checks passed:
- [x] install.sh exists and passes bash -n syntax check
- [x] Script handles Linux x86_64, Linux aarch64, macOS x86_64, macOS aarch64
- [x] Script downloads from GitHub releases URL pattern
- [x] Script verifies checksums before installing
- [x] README has installation instructions

## Deviations from Plan

None - plan executed exactly as written

## Issues Encountered

None

---
*Phase: 06-installation-updates*
*Completed: 2026-01-14*
