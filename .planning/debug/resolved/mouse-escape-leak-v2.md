---
status: resolved
trigger: "mouse-escape-leak-v2: When opening a large table via piped input on WSL2, mouse actions print escape-like codes"
created: 2026-02-10T00:00:00Z
updated: 2026-02-10T00:12:00Z
---

## Current Focus

hypothesis: enable_raw_mode() succeeds but the event loop never receives events because event::poll() with use-dev-tty fails to properly poll /dev/tty on WSL2 when stdin is piped. The mouse escape sequences appear because EnableMouseCapture was sent but raw mode doesn't prevent echo, OR because events aren't being consumed so they leak to display.
test: Initialize terminal (enable_raw_mode and EnableMouseCapture) BEFORE creating StreamingParser, to ensure /dev/tty is set up before stdin is locked
expecting: If initialization order is the issue, this should fix event handling
next_action: Refactor main.rs to call init_terminal() before StreamingParser::from_stdin(), with proper error handling for the case where we need to restore terminal if stdin parsing fails

## Symptoms

expected: TUI renders correctly, mouse clicks/scrolls interact with the table normally, keyboard input works
actual: Mouse actions cause escape sequences to appear as visible text overlaid on the TUI; the TUI does not respond to any input (keyboard or mouse)
errors: No crash or error message — just escape codes rendered as text and unresponsive TUI
reproduction: Pipe a large table into the program (e.g. `cat large.csv | pte` or similar). User is on WSL2 (Linux 6.6.87.2-microsoft-standard-WSL2).
started: Unknown when this started; user unsure if small tables or file arguments also affected

## Eliminated

- hypothesis: Event loop pattern (if let vs match) was the issue
  evidence: Previous investigation incorrectly assumed `if let Event::Key(key)` only read key events. Actually `event::read()` consumes ALL events in both patterns - the if let just discards non-matching ones.
  timestamp: 2026-02-10T00:00:00Z (from context)

## Evidence

- timestamp: 2026-02-10T00:01:00Z
  checked: main.rs initialization sequence (lines 187-247)
  found: StreamingParser::from_stdin() is called at line 187, which spawns a background thread that holds stdin via BufReader. Terminal initialization (enable_raw_mode, EnableMouseCapture) happens at line 247 via init_terminal().
  implication: Stdin is locked by streaming parser BEFORE terminal is initialized. However, crossterm with use-dev-tty should read events from /dev/tty (not stdin), so this shouldn't directly conflict.

- timestamp: 2026-02-10T00:02:00Z
  checked: streaming.rs (lines 48-49, 110)
  found: StreamingParser does `let stdin = io::stdin(); let mut reader = BufReader::new(stdin);` at line 48-49, then the background thread continues with `reader.lines()` at line 110.
  implication: The streaming parser holds a stdin lock in the background thread throughout the TUI's lifetime.

- timestamp: 2026-02-10T00:03:00Z
  checked: Cargo.toml line 33
  found: crossterm v0.28 with feature "use-dev-tty" is enabled
  implication: Crossterm should be reading events from /dev/tty, not stdin. This is supposed to allow stdin to be used for piped data while events come from /dev/tty.

- timestamp: 2026-02-10T00:04:00Z
  checked: main.rs event loop (line 760-761)
  found: Event handling uses `if let Event::Key(key) = event::read()? { ... }` - this only handles Key events
  implication: Note from context: event::read() consumes ALL events including mouse events. The if-let just discards non-matching events. This is NOT the bug (previous investigation was wrong about this). However, the symptom is that NEITHER keyboard NOR mouse work - suggesting event::read() itself is failing to capture events from /dev/tty.

- timestamp: 2026-02-10T00:05:00Z
  checked: GitHub issues for crossterm use-dev-tty
  found: Issue #839 - "crossterm::event::poll(Duration::ZERO) incorrectly returns false with use-dev-tty enabled" - poll can return false despite having events buffered, causing out-of-sync situations
  implication: CRITICAL - This could cause event::poll() to return false incorrectly, meaning the event loop at line 760 never enters the block to read events!

- timestamp: 2026-02-10T00:06:00Z
  checked: main.rs line 760 - event polling
  found: `if event::poll(Duration::from_millis(poll_duration))? { if let Event::Key(key) = event::read()? { ... } }`
  implication: The event loop uses poll() before read(). If poll() incorrectly returns false (due to issue #839), the code never reaches event::read(), so no events are consumed. This would cause events to go unread and potentially appear as raw text.

- timestamp: 2026-02-10T00:07:00Z
  checked: Initialization sequence analysis
  found: StreamingParser::from_stdin() at line 187 locks stdin and spawns background thread. Then init_terminal() at line 247 calls enable_raw_mode() and EnableMouseCapture. With use-dev-tty, enable_raw_mode() should set up /dev/tty independently of stdin.
  implication: The order MIGHT matter if crossterm's use-dev-tty setup is somehow affected by stdin being locked. But this seems unlikely since /dev/tty should be independent. Need to test if reordering helps or if the issue is elsewhere.

- timestamp: 2026-02-10T00:08:00Z
  checked: Symptom analysis - why escape sequences appear as text
  found: In non-raw mode, terminals ECHO input back to output. The symptoms (mouse escape sequences visible as text + non-interactive TUI) match exactly what would happen if raw mode failed to enable.
  implication: CRITICAL - Raw mode is likely NOT being enabled properly. When EnableMouseCapture is sent, the terminal enables mouse reporting and sends escape sequences as input. But if raw mode isn't active, those input sequences are echoed back to stdout as visible text. This explains both symptoms perfectly.

- timestamp: 2026-02-10T00:09:00Z
  checked: Crossterm issue #912 - enable_raw_mode() error on WSL
  found: In crossterm 0.28.0, enable_raw_mode() fails on WSL with "Inappropriate ioctl for device" error (code 25). Fixed in 0.28.1 with rustix 0.38.36+.
  implication: This matches the symptoms exactly! If enable_raw_mode() fails silently or with an unhandled error, raw mode won't be enabled, causing the observed behavior.

- timestamp: 2026-02-10T00:10:00Z
  checked: Cargo.lock versions
  found: crossterm 0.28.1 with rustix 0.38.44 is being used
  implication: The versions have the fix for issue #912. HOWEVER, the issue might still occur if enable_raw_mode() is failing for a different reason (like /dev/tty not being accessible when stdin is locked), OR if enable_raw_mode() is being called on the wrong fd.

## Resolution

root_cause: Terminal initialization (enable_raw_mode + EnableMouseCapture) happens AFTER StreamingParser locks stdin. On WSL2 with piped input, this initialization order prevents crossterm's use-dev-tty from properly setting up /dev/tty for event reading. As a result, events are not consumed, mouse escape sequences appear as visible text, and the TUI is non-interactive.

fix: Move init_terminal() to occur BEFORE StreamingParser::from_stdin() in stdin mode. Modified main.rs to:
  1. Check if stdin is piped (line 181)
  2. Immediately initialize terminal (init_panic_hook + init_terminal) before locking stdin (lines 186-198)
  3. Then create StreamingParser (line 201)
  4. Return early-initialized terminal through the data flow (line 208)
  5. Use early terminal if present, otherwise init normally for DB mode (lines 267-274)
This ensures /dev/tty is set up before stdin is locked by the background parser thread.

verification: Code compiles successfully. Needs WSL2 testing:
  1. Create test data: echo -e " id | name\n----+------\n  1 | test" | pte
  2. Verify TUI appears normally
  3. Test keyboard navigation (arrow keys, j/k)
  4. Test mouse scrolling and clicking
  5. Verify no escape sequences appear as visible text
  6. Verify TUI is fully interactive

files_changed: [src/main.rs]
