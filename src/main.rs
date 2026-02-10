use std::cell::Cell as StdCell;
use std::io;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use pretty_table_explorer::{db, export, handlers, parser, render, state, streaming, update, workspace};
use handlers::{
    handle_export_filename, handle_export_format, handle_normal_mode, handle_query_input,
    handle_search_input, KeyAction, WorkspaceOp,
};
use parser::TableData;
use render::{
    build_controls_hint, build_pane_render_data, build_pane_title, build_tab_bar,
    render_format_prompt, render_input_bar, render_table_pane,
};
use state::{AppMode, PendingAction};
use workspace::{ViewMode, Workspace};

/// Interactive terminal table viewer for PostgreSQL
#[derive(Parser, Debug)]
#[command(name = "pte", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Connect to PostgreSQL database
    #[arg(long)]
    connect: Option<String>,

    /// SQL query to execute (default: show tables)
    #[arg(long)]
    query: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Update to the latest version
    Update,
}

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Paragraph, TableState},
};

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

/// Install panic hook that restores terminal state before printing panic message.
/// Must be called BEFORE terminal initialization.
fn init_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));
}

/// Initialize the terminal for TUI rendering.
/// Enables raw mode, enters alternate screen, and creates a Terminal instance.
fn init_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

/// Restore the terminal to its original state.
/// Disables raw mode and leaves alternate screen.
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Print usage information and exit.
fn print_usage() -> ! {
    eprintln!("Usage: pte [OPTIONS]");
    eprintln!("       cat data.txt | pte");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --connect <CONN_STRING>  Connect to PostgreSQL database");
    eprintln!("  --query <SQL>            SQL query to execute (default: show tables)");
    eprintln!();
    eprintln!("Connection string formats:");
    eprintln!("  \"host=localhost user=postgres dbname=mydb\"");
    eprintln!("  \"postgresql://user:pass@host/db\"");
    std::process::exit(1);
}

/// Parse CLI arguments and return database config if --connect provided.
/// Returns (connection_string, query, has_custom_query) if in database mode.
fn parse_cli() -> (Option<Commands>, Option<(String, String, bool)>) {
    let cli = Cli::parse();

    let db_config = cli.connect.map(|conn| {
        let has_custom_query = cli.query.is_some();
        let default_query =
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name"
                .to_string();
        (conn, cli.query.unwrap_or(default_query), has_custom_query)
    });

    (cli.command, db_config)
}

fn main() -> io::Result<()> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    // Parse CLI arguments
    let (command, db_config) = parse_cli();

    // Handle update subcommand first
    if let Some(Commands::Update) = command {
        if let Err(e) = update::do_update() {
            eprintln!("Update failed: {}", e);
            std::process::exit(1);
        }
        return Ok(());
    }

    // Get table data, database client, initial view mode, and optional streaming loader from either database or stdin
    let (table_data, mut db_client, initial_view_mode, mut streaming_loader) =
        if let Some((conn_string, query, has_custom_query)) = db_config {
            // Direct database connection mode
            match db::connect(&conn_string) {
                Ok(mut client) => match db::execute_query(&mut client, &query) {
                    Ok(data) => {
                        if data.headers.is_empty() && data.rows.is_empty() {
                            eprintln!("Query returned no results.");
                            std::process::exit(0);
                        }
                        // If user provided custom query, show as TableData; otherwise TableList
                        let mode = if has_custom_query {
                            ViewMode::TableData
                        } else {
                            ViewMode::TableList
                        };
                        (data, Some(client), mode, None)
                    }
                    Err(e) => {
                        eprintln!("Error: Query failed: {}", e);
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    // Provide helpful error messages for common connection issues
                    let err_msg = e.to_string();
                    if err_msg.contains("Connection refused") {
                        eprintln!("Error: Could not connect to PostgreSQL at specified host.");
                        eprintln!("Make sure PostgreSQL is running and accepting connections.");
                    } else if err_msg.contains("authentication") || err_msg.contains("password") {
                        eprintln!("Error: Authentication failed for PostgreSQL connection.");
                        eprintln!("Check your username and password.");
                    } else {
                        eprintln!("Error: Failed to connect to database: {}", e);
                    }
                    std::process::exit(1);
                }
            }
        } else {
            // Stdin mode - check if stdin has data
            use std::io::IsTerminal;
            if io::stdin().is_terminal() {
                // No piped input and no --connect flag
                print_usage();
            }

            // Use streaming parser for non-blocking stdin reading
            match streaming::StreamingParser::from_stdin() {
                Ok(Some(loader)) => {
                    // Create initial TableData from headers (rows will stream in)
                    let initial_data = parser::TableData {
                        headers: loader.headers().to_vec(),
                        rows: Vec::with_capacity(100_000),
                        interner: lasso::Rodeo::default(),
                    };
                    // Return data + loader as Option for event loop to poll
                    (initial_data, None, ViewMode::PipeData, Some(loader))
                }
                Ok(None) => {
                    eprintln!("Error: Invalid or empty input. Expected psql table format.");
                    eprintln!("Usage: psql -c 'SELECT ...' | pretty-table-explorer");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error reading stdin: {}", e);
                    std::process::exit(1);
                }
            }
        };

    // Store the table list for back navigation (only in DB mode without custom query)
    let table_list_cache: Option<TableData> = if initial_view_mode == ViewMode::TableList {
        Some(table_data.clone())
    } else {
        None
    };

    // Track current table name when viewing table data
    let mut current_table_name: Option<String> = None;

    // Create workspace and add initial tab with its view mode
    let mut workspace = Workspace::new();
    let tab_name = match initial_view_mode {
        ViewMode::TableList => "Tables".to_string(),
        ViewMode::TableData => current_table_name
            .clone()
            .unwrap_or_else(|| "Query".to_string()),
        ViewMode::PipeData => "Data".to_string(),
    };
    workspace.add_tab(tab_name, table_data, initial_view_mode);

    // Last visible column index from previous render (for scroll-right detection)
    // Using StdCell to allow updating from within the draw closure
    let last_visible_col_idx: StdCell<usize> = StdCell::new(0);

    // Application state for input modes
    let mut current_mode = AppMode::Normal;
    let mut input_buffer = String::new();
    let mut status_message: Option<String> = None;
    let mut status_message_time: Option<Instant> = None;

    // Export state
    let mut export_format: Option<export::ExportFormat> = None;

    // Set up panic hook to restore terminal on crash
    init_panic_hook();

    let mut terminal = init_terminal()?;

    // Track the count of displayed rows for navigation bounds
    #[allow(unused_assignments)]
    let mut displayed_row_count = 0;

    // Main event loop
    loop {
        // Poll streaming loader for new rows
        let had_streaming_loader = streaming_loader.is_some();
        if let Some(ref loader) = streaming_loader {
            // Non-blocking receive of new rows
            let new_rows = loader.try_recv_batch(5000);
            if !new_rows.is_empty() {
                // Append to the first tab's data (streaming always uses tab 0)
                if let Some(tab) = workspace.tabs.get_mut(0) {
                    // Reserve capacity in larger chunks to minimize reallocations
                    let new_total = tab.data.rows.len() + new_rows.len();
                    if new_total > tab.data.rows.capacity() {
                        let additional = (new_total - tab.data.rows.capacity()).max(50_000);
                        tab.data.rows.reserve(additional);
                    }
                    // Intern strings on main thread before appending
                    tab.intern_and_append_rows(new_rows);
                }
            }

            // Clean up loader when complete and channel is drained
            if loader.is_complete() {
                // Check if channel still has data
                let remaining = loader.try_recv_batch(5000);
                if remaining.is_empty() {
                    // All data consumed, drop the loader
                    streaming_loader = None;
                } else {
                    // Still draining, append these rows too
                    if let Some(tab) = workspace.tabs.get_mut(0) {
                        tab.intern_and_append_rows(remaining);
                    }
                }
            }
        }

        // Detect streaming completion (loader was present but just removed)
        if had_streaming_loader && streaming_loader.is_none() {
            let actual_rows = workspace.tabs.first().map(|t| t.data.rows.len()).unwrap_or(0);
            status_message = Some(format!("Loaded {} rows", actual_rows));
            status_message_time = Some(Instant::now());
        }

        // Build tab bar string BEFORE getting mutable reference to tab
        let tab_bar = build_tab_bar(&workspace);
        let tab_count = workspace.tab_count();
        let is_split = workspace.split_active && tab_count > 1;

        // Clamp selected_visible_col and scroll_col_offset for all relevant tabs
        // This needs to happen before building render data
        if is_split {
            // Handle left pane (active tab)
            if let Some(tab) = workspace.tabs.get_mut(workspace.active_idx) {
                let visible_cols = tab.column_config.visible_indices();
                if !visible_cols.is_empty() {
                    if tab.selected_visible_col >= visible_cols.len() {
                        tab.selected_visible_col = visible_cols.len() - 1;
                    }
                    if tab.scroll_col_offset >= visible_cols.len() {
                        tab.scroll_col_offset = visible_cols.len() - 1;
                    }
                    if tab.selected_visible_col < tab.scroll_col_offset {
                        tab.scroll_col_offset = tab.selected_visible_col;
                    }
                    // Scroll right if selected column is beyond last visible (only if this is focused pane)
                    if workspace.focus_left && tab.selected_visible_col > last_visible_col_idx.get()
                    {
                        tab.scroll_col_offset =
                            tab.selected_visible_col.min(visible_cols.len() - 1);
                    }
                }
            }
            // Handle right pane (split tab)
            if let Some(tab) = workspace.tabs.get_mut(workspace.split_idx) {
                let visible_cols = tab.column_config.visible_indices();
                if !visible_cols.is_empty() {
                    if tab.selected_visible_col >= visible_cols.len() {
                        tab.selected_visible_col = visible_cols.len() - 1;
                    }
                    if tab.scroll_col_offset >= visible_cols.len() {
                        tab.scroll_col_offset = visible_cols.len() - 1;
                    }
                    if tab.selected_visible_col < tab.scroll_col_offset {
                        tab.scroll_col_offset = tab.selected_visible_col;
                    }
                    // Scroll right if selected column is beyond last visible (only if this is focused pane)
                    if !workspace.focus_left
                        && tab.selected_visible_col > last_visible_col_idx.get()
                    {
                        tab.scroll_col_offset =
                            tab.selected_visible_col.min(visible_cols.len() - 1);
                    }
                }
            }
        } else {
            // Single pane mode - handle focused tab
            if let Some(tab) = workspace.focused_tab_mut() {
                let visible_cols = tab.column_config.visible_indices();
                if !visible_cols.is_empty() {
                    if tab.selected_visible_col >= visible_cols.len() {
                        tab.selected_visible_col = visible_cols.len() - 1;
                    }
                    if tab.scroll_col_offset >= visible_cols.len() {
                        tab.scroll_col_offset = visible_cols.len() - 1;
                    }
                    if tab.selected_visible_col < tab.scroll_col_offset {
                        tab.scroll_col_offset = tab.selected_visible_col;
                    }
                    // Scroll right if selected column is beyond last visible
                    // Use direct assignment to scroll in one step (not incrementally)
                    // This ensures immediate navigation even past wide columns
                    if tab.selected_visible_col > last_visible_col_idx.get() {
                        // Scroll so selected column is the first visible (leftmost)
                        // This ensures we scroll enough in one step, even for wide columns
                        tab.scroll_col_offset =
                            tab.selected_visible_col.min(visible_cols.len() - 1);
                    }
                }
            }
        }

        // Update cached widths for tabs being rendered (incremental, O(new_rows))
        if let Some(tab) = workspace.tabs.get_mut(workspace.active_idx) {
            tab.update_cached_widths();
        }
        if is_split {
            if let Some(tab) = workspace.tabs.get_mut(workspace.split_idx) {
                tab.update_cached_widths();
            }
        }

        // Build render data for panes (viewport-windowed to avoid cloning all rows)
        let viewport_height = crossterm::terminal::size()
            .map(|(_, h)| h as usize)
            .unwrap_or(50);
        let left_pane_data = workspace
            .tabs
            .get(workspace.active_idx)
            .map(|tab| build_pane_render_data(tab, viewport_height));
        let right_pane_data = if is_split {
            workspace
                .tabs
                .get(workspace.split_idx)
                .map(|tab| build_pane_render_data(tab, viewport_height))
        } else {
            None
        };

        // Get displayed row count for navigation bounds (from focused tab)
        let focused_pane_data = if is_split && !workspace.focus_left {
            right_pane_data.as_ref()
        } else {
            left_pane_data.as_ref()
        };
        displayed_row_count = focused_pane_data
            .map(|p| p.displayed_row_count)
            .unwrap_or(0);

        // Show loading indicator as status message while streaming loader is active
        if streaming_loader.is_some() {
            let actual_rows = workspace.tabs.first().map(|t| t.data.rows.len()).unwrap_or(0);
            status_message = Some(format!("Loading... {} rows", actual_rows));
            // Don't set status_message_time -- we don't want it to auto-clear during loading
            status_message_time = None;
        }

        // Capture state needed for rendering (to avoid borrow issues)
        let mode = current_mode;
        let input_buf = input_buffer.clone();
        let status = status_message.clone();

        // Capture view modes for closure (per-tab view modes)
        let left_view_mode = workspace
            .tabs
            .get(workspace.active_idx)
            .map(|t| t.view_mode)
            .unwrap_or(ViewMode::PipeData);
        let right_view_mode = if is_split {
            workspace
                .tabs
                .get(workspace.split_idx)
                .map(|t| t.view_mode)
                .unwrap_or(ViewMode::PipeData)
        } else {
            left_view_mode
        };
        // Focused tab's view mode for controls display
        let current_view = if is_split && !workspace.focus_left {
            right_view_mode
        } else {
            left_view_mode
        };
        let table_name = current_table_name.clone();

        // Capture focus state
        let focus_left = workspace.focus_left;

        // Clone table states for mutable use in render
        let mut left_table_state = workspace
            .tabs
            .get(workspace.active_idx)
            .map(|t| t.table_state.clone())
            .unwrap_or_default();
        let mut right_table_state = if is_split {
            workspace
                .tabs
                .get(workspace.split_idx)
                .map(|t| t.table_state.clone())
                .unwrap_or_default()
        } else {
            TableState::default()
        };

        // Adjust table states for viewport windowing (translate absolute → relative)
        let left_viewport_offset = left_pane_data
            .as_ref()
            .map(|p| p.viewport_row_offset)
            .unwrap_or(0);
        let right_viewport_offset = right_pane_data
            .as_ref()
            .map(|p| p.viewport_row_offset)
            .unwrap_or(0);
        if left_viewport_offset > 0 {
            if let Some(sel) = left_table_state.selected() {
                left_table_state.select(Some(sel.saturating_sub(left_viewport_offset)));
            }
            *left_table_state.offset_mut() = left_table_state
                .offset()
                .saturating_sub(left_viewport_offset);
        }
        if right_viewport_offset > 0 {
            if let Some(sel) = right_table_state.selected() {
                right_table_state.select(Some(sel.saturating_sub(right_viewport_offset)));
            }
            *right_table_state.offset_mut() = right_table_state
                .offset()
                .saturating_sub(right_viewport_offset);
        }

        terminal.draw(|frame| {
            let area = frame.area();

            // Split layout: table area + optional input bar at bottom
            let show_input_bar = mode != AppMode::Normal && mode != AppMode::ExportFormat;
            let show_format_prompt = mode == AppMode::ExportFormat;
            let chunks = if show_input_bar || show_format_prompt {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(3), Constraint::Length(3)])
                    .split(area)
            } else {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(3)])
                    .split(area)
            };

            let table_area = chunks[0];

            // Build title components
            let status_info = status
                .as_ref()
                .map(|s| format!("{} ", s))
                .unwrap_or_default();

            if is_split {
                // Split view: vertical layout with tab bar, panes, and controls
                let split_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1), // Tab bar
                        Constraint::Min(3),    // Panes
                        Constraint::Length(1), // Controls
                    ])
                    .split(table_area);

                let tab_bar_area = split_layout[0];
                let panes_area = split_layout[1];
                let controls_area = split_layout[2];

                // Render tab bar at top
                let tab_bar_widget =
                    Paragraph::new(tab_bar.clone()).style(Style::default().fg(Color::Cyan));
                frame.render_widget(tab_bar_widget, tab_bar_area);

                // Split panes horizontally
                let pane_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(panes_area);

                // Render left pane (active tab)
                if let Some(ref pane_data) = left_pane_data {
                    let pane_title =
                        build_pane_title(pane_data, &table_name, left_view_mode, focus_left);
                    render_table_pane(
                        frame,
                        pane_chunks[0],
                        pane_data,
                        pane_title,
                        focus_left,
                        &mut left_table_state,
                        &last_visible_col_idx,
                    );
                }

                // Render right pane (split tab)
                if let Some(ref pane_data) = right_pane_data {
                    let pane_title =
                        build_pane_title(pane_data, &table_name, right_view_mode, !focus_left);
                    render_table_pane(
                        frame,
                        pane_chunks[1],
                        pane_data,
                        pane_title,
                        !focus_left,
                        &mut right_table_state,
                        &last_visible_col_idx,
                    );
                }

                // Build and render controls hint at bottom
                let controls = build_controls_hint(current_view, is_split, tab_count);
                let controls_widget = Paragraph::new(format!("{}{}", status_info, controls))
                    .style(Style::default().fg(Color::DarkGray));
                frame.render_widget(controls_widget, controls_area);
            } else {
                // Single pane mode
                if let Some(ref pane_data) = left_pane_data {
                    // Build full title with tab bar, position info, filter, status, and controls
                    let row_info = pane_data
                        .selected_row
                        .map(|r| {
                            if pane_data.filter_text.is_empty() {
                                format!("Row {}/{}", r + 1, pane_data.total_rows)
                            } else {
                                format!(
                                    "Row {}/{} (filtered from {})",
                                    r + 1,
                                    pane_data.displayed_row_count,
                                    pane_data.total_rows
                                )
                            }
                        })
                        .unwrap_or_default();

                    let hidden_info = if pane_data.hidden_count > 0 {
                        format!(" ({} hidden)", pane_data.hidden_count)
                    } else {
                        String::new()
                    };
                    let col_info = if !pane_data.visible_cols.is_empty() {
                        format!(
                            " Col {}/{}{}",
                            pane_data.selected_visible_col + 1,
                            pane_data.visible_count,
                            hidden_info
                        )
                    } else {
                        String::new()
                    };

                    let position = if row_info.is_empty() {
                        String::new()
                    } else {
                        format!("[{}{}] ", row_info, col_info)
                    };

                    let filter_info = if !pane_data.filter_text.is_empty() {
                        format!("/{} ", pane_data.filter_text)
                    } else {
                        String::new()
                    };

                    let context_label: &str = match current_view {
                        ViewMode::TableList => "Tables",
                        ViewMode::TableData => table_name.as_deref().unwrap_or("Query Result"),
                        ViewMode::PipeData => "Data",
                    };
                    let controls = build_controls_hint(current_view, is_split, tab_count);

                    let title = format!(
                        "{}{} {} {}{}{}",
                        tab_bar, context_label, position, filter_info, status_info, controls
                    );

                    render_table_pane(
                        frame,
                        table_area,
                        pane_data,
                        title,
                        true, // Always focused in single pane mode
                        &mut left_table_state,
                        &last_visible_col_idx,
                    );
                }
            }

            // Render input bar when in input mode
            if show_input_bar {
                render_input_bar(frame, chunks[1], mode, &input_buf);
            }

            // Render format selection prompt
            if show_format_prompt {
                render_format_prompt(frame, chunks[1]);
            }
        })?;

        // Translate table states back from viewport-relative to absolute
        if left_viewport_offset > 0 {
            if let Some(sel) = left_table_state.selected() {
                left_table_state.select(Some(sel + left_viewport_offset));
            }
            *left_table_state.offset_mut() += left_viewport_offset;
        }
        if right_viewport_offset > 0 {
            if let Some(sel) = right_table_state.selected() {
                right_table_state.select(Some(sel + right_viewport_offset));
            }
            *right_table_state.offset_mut() += right_viewport_offset;
        }

        // Sync table states back to workspace
        if let Some(tab) = workspace.tabs.get_mut(workspace.active_idx) {
            tab.table_state = left_table_state;
        }
        if is_split {
            if let Some(tab) = workspace.tabs.get_mut(workspace.split_idx) {
                tab.table_state = right_table_state;
            }
        }

        // Clear status message after 3 seconds
        if let Some(msg_time) = status_message_time {
            if msg_time.elapsed().as_secs() >= 3 {
                status_message = None;
                status_message_time = None;
            }
        }

        // Poll with 250ms timeout for responsive feel
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                // Pending action for deferred tab creation (to avoid borrow conflicts)
                let mut pending_action = PendingAction::None;

                // Capture workspace state before borrowing tab (to avoid borrow conflicts)
                let has_split = workspace.split_active;
                let tab_count = workspace.tab_count();

                // Track workspace operation to execute after tab borrow ends
                let mut workspace_op: Option<WorkspaceOp> = None;

                // Scope for tab borrow - all handlers that need tab must be in this block
                {
                    // Get fresh mutable reference to focused tab for event handling
                    // In split mode, this respects focus_left; otherwise it's the active tab
                    let tab = workspace.focused_tab_mut().unwrap();

                    match current_mode {
                        AppMode::Normal => {
                            match handle_normal_mode(
                                &key,
                                tab,
                                &mut db_client,
                                &table_list_cache,
                                &mut current_table_name,
                                displayed_row_count,
                                has_split,
                                tab_count,
                            ) {
                                KeyAction::Quit => {
                                    if let Some(loader) = streaming_loader.take() {
                                        // Cancel loading but keep app running with partial data
                                        loader.cancel();
                                        // Drop loader (triggers join via Drop impl)
                                        drop(loader);
                                        status_message = Some("Loading cancelled".to_string());
                                        status_message_time = Some(Instant::now());
                                        // Do NOT break -- let user browse partial data
                                    } else {
                                        break; // No streaming -- normal quit
                                    }
                                }
                                KeyAction::StatusMessage(msg) => {
                                    status_message = Some(msg);
                                    status_message_time = Some(Instant::now());
                                }
                                KeyAction::CreateTab {
                                    name,
                                    data,
                                    view_mode,
                                } => {
                                    pending_action = PendingAction::CreateTab {
                                        name,
                                        data,
                                        view_mode,
                                    };
                                }
                                KeyAction::ModeChange(mode) => {
                                    current_mode = mode;
                                    input_buffer.clear();
                                }
                                KeyAction::Workspace(op) => {
                                    workspace_op = Some(op);
                                }
                                KeyAction::None => {}
                            }
                        }

                        AppMode::QueryInput => {
                            let (action, return_to_normal) =
                                handle_query_input(&key, &mut input_buffer, &mut db_client);
                            if return_to_normal {
                                current_mode = AppMode::Normal;
                            }
                            match action {
                                KeyAction::StatusMessage(msg) => {
                                    status_message = Some(msg);
                                    status_message_time = Some(Instant::now());
                                }
                                KeyAction::CreateTab {
                                    name,
                                    data,
                                    view_mode,
                                } => {
                                    pending_action = PendingAction::CreateTab {
                                        name,
                                        data,
                                        view_mode,
                                    };
                                }
                                _ => {}
                            }
                        }

                        AppMode::SearchInput => {
                            if handle_search_input(&key, &mut input_buffer, tab) {
                                current_mode = AppMode::Normal;
                            }
                        }

                        AppMode::ExportFormat => {
                            if let Some(new_mode) =
                                handle_export_format(&key, &mut export_format, &mut input_buffer)
                            {
                                current_mode = new_mode;
                            }
                        }

                        AppMode::ExportFilename => {
                            let (msg, done) =
                                handle_export_filename(&key, &mut input_buffer, export_format, tab);
                            if let Some(m) = msg {
                                status_message = Some(m);
                                status_message_time = Some(Instant::now());
                            }
                            if done {
                                current_mode = AppMode::Normal;
                                export_format = None;
                            }
                        }
                    }
                }

                // Execute workspace operations outside tab borrow scope
                if let Some(op) = workspace_op {
                    match op {
                        WorkspaceOp::ToggleSplit => workspace.toggle_split(),
                        WorkspaceOp::ToggleFocus => workspace.toggle_focus(),
                        WorkspaceOp::NextTab => workspace.next_tab(),
                        WorkspaceOp::PrevTab => workspace.prev_tab(),
                        WorkspaceOp::SwitchTo(idx) => workspace.switch_to(idx),
                        WorkspaceOp::CloseTab => {
                            let idx = workspace.focused_idx();
                            workspace.close_tab(idx);
                        }
                    }
                }

                // Process pending action (tab borrow has been dropped)
                if let PendingAction::CreateTab {
                    name,
                    data,
                    view_mode,
                } = pending_action
                {
                    let new_idx = workspace.add_tab(name, data, view_mode);
                    // In split view with focus on right pane, open in right pane
                    if workspace.split_active && !workspace.focus_left {
                        workspace.split_idx = new_idx;
                    } else {
                        workspace.switch_to(new_idx);
                    }
                    status_message = Some(format!("Opened in tab {}", new_idx + 1));
                    status_message_time = Some(Instant::now());
                }
            }
        }
    }

    // Clear terminal before exit
    terminal.clear()?;
    restore_terminal(&mut terminal)?;
    Ok(())
}
