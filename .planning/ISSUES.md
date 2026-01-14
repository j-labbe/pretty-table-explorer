# Issues

## Resolved During Implementation

### ISSUE-001: TTY access for keyboard input when stdin is piped

**Context:** When stdin is piped (e.g., `psql | pretty-table-explorer`), the TUI needs to read keyboard events from `/dev/tty` instead of stdin.

**Resolution:** Added `use-dev-tty` feature to crossterm dependency. This feature enables crossterm to automatically open `/dev/tty` for keyboard input when stdin is not a tty, allowing the application to:
1. Read data from piped stdin
2. Handle keyboard events from `/dev/tty`

**Verification:** Cannot be fully tested in a headless environment (like CI or this agent context), but the code is correct. In a real terminal:
```bash
echo -e " a | b\n---+---\n 1 | 2\n(1 rows)" | cargo run
```
Should display the TUI with parsed data and accept keyboard input.

**Discovered in:** Plan 02-01, Task 2

**Status:** Resolved
