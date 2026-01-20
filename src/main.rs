mod column;
mod db;
mod export;
mod parser;
mod render;
mod state;
mod update;
mod workspace;

use std::cell::Cell as StdCell;
use std::io::{self, Read};
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use column::ColumnConfig;
use parser::TableData;
use render::{build_pane_render_data, build_pane_title, render_table_pane, calculate_auto_widths};
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
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, TableState},
};

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

    // Get table data, database client, and initial view mode from either database or stdin
    let (table_data, mut db_client, initial_view_mode) =
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
                        (data, Some(client), mode)
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

            // Read and parse stdin
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;

            match parser::parse_psql(&input) {
                Some(data) => (data, None, ViewMode::PipeData),
                None => {
                    eprintln!("Error: Invalid or empty input. Expected psql table format.");
                    eprintln!("Usage: psql -c 'SELECT ...' | pretty-table-explorer");
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
        ViewMode::TableData => current_table_name.clone().unwrap_or_else(|| "Query".to_string()),
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
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    let mut terminal = init_terminal()?;

    // Track the count of displayed rows for navigation bounds
    #[allow(unused_assignments)]
    let mut displayed_row_count = 0;

    // Main event loop
    loop {
        // Build tab bar string BEFORE getting mutable reference to tab
        // (only shown when multiple tabs exist)
        // Format: "1:name 2:name [3:active] 4:name | " with numbers matching keyboard shortcuts
        let tab_count = workspace.tab_count();
        let is_split = workspace.split_active && tab_count > 1;
        let tab_bar = if tab_count > 1 {
            let names: Vec<String> = workspace.tabs.iter().enumerate().map(|(i, t)| {
                // Truncate long tab names to prevent title overflow
                let name = if t.name.len() > 15 {
                    format!("{}...", &t.name[..12])
                } else {
                    t.name.clone()
                };
                // Show index number (1-based) with each tab name
                // Mark both active and split tabs in split mode
                if is_split && i == workspace.split_idx && i != workspace.active_idx {
                    format!("<{}:{}>", i + 1, name)
                } else if i == workspace.active_idx {
                    format!("[{}:{}]", i + 1, name)
                } else {
                    format!("{}:{}", i + 1, name)
                }
            }).collect();
            format!("{} | ", names.join(" "))
        } else {
            String::new()
        };

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
                    if workspace.focus_left && tab.selected_visible_col > last_visible_col_idx.get() {
                        tab.scroll_col_offset = tab.selected_visible_col.min(visible_cols.len() - 1);
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
                    if !workspace.focus_left && tab.selected_visible_col > last_visible_col_idx.get() {
                        tab.scroll_col_offset = tab.selected_visible_col.min(visible_cols.len() - 1);
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
                        tab.scroll_col_offset = tab.selected_visible_col.min(visible_cols.len() - 1);
                    }
                }
            }
        }

        // Build render data for panes
        let left_pane_data = workspace.tabs.get(workspace.active_idx).map(build_pane_render_data);
        let right_pane_data = if is_split {
            workspace.tabs.get(workspace.split_idx).map(build_pane_render_data)
        } else {
            None
        };

        // Get displayed row count for navigation bounds (from focused tab)
        let focused_pane_data = if is_split && !workspace.focus_left {
            right_pane_data.as_ref()
        } else {
            left_pane_data.as_ref()
        };
        displayed_row_count = focused_pane_data.map(|p| p.displayed_row_count).unwrap_or(0);

        // Capture state needed for rendering (to avoid borrow issues)
        let mode = current_mode;
        let input_buf = input_buffer.clone();
        let status = status_message.clone();

        // Capture view modes for closure (per-tab view modes)
        let left_view_mode = workspace.tabs.get(workspace.active_idx)
            .map(|t| t.view_mode)
            .unwrap_or(ViewMode::PipeData);
        let right_view_mode = if is_split {
            workspace.tabs.get(workspace.split_idx)
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
        let mut left_table_state = workspace.tabs.get(workspace.active_idx)
            .map(|t| t.table_state.clone())
            .unwrap_or_default();
        let mut right_table_state = if is_split {
            workspace.tabs.get(workspace.split_idx)
                .map(|t| t.table_state.clone())
                .unwrap_or_default()
        } else {
            TableState::default()
        };

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

            // Build context-appropriate controls hint
            let split_controls = if is_split {
                "Tab: switch pane, V: unsplit, "
            } else if tab_count > 1 {
                "V: split, "
            } else {
                ""
            };
            let tab_controls = if tab_count > 1 { "1-9: tab, W: close, " } else { "" };

            if is_split {
                // Split view: vertical layout with tab bar, panes, and controls
                let split_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),  // Tab bar
                        Constraint::Min(3),     // Panes
                        Constraint::Length(1),  // Controls
                    ])
                    .split(table_area);

                let tab_bar_area = split_layout[0];
                let panes_area = split_layout[1];
                let controls_area = split_layout[2];

                // Render tab bar at top
                let tab_bar_widget = Paragraph::new(tab_bar.clone())
                    .style(Style::default().fg(Color::Cyan));
                frame.render_widget(tab_bar_widget, tab_bar_area);

                // Split panes horizontally
                let pane_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(panes_area);

                // Render left pane (active tab)
                if let Some(ref pane_data) = left_pane_data {
                    let pane_title = build_pane_title(
                        pane_data,
                        &table_name,
                        left_view_mode,
                        focus_left,
                    );
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
                    let pane_title = build_pane_title(
                        pane_data,
                        &table_name,
                        right_view_mode,
                        !focus_left,
                    );
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
                let controls: String = match current_view {
                    ViewMode::TableList => format!("{}{}Enter: select, /: filter, q: quit", split_controls, tab_controls),
                    ViewMode::TableData => format!("{}{}+/-: width, H/S: hide/show, E: export, 0: reset, Esc: back, q: quit", split_controls, tab_controls),
                    ViewMode::PipeData => format!("{}{}+/-: width, H/S: hide/show, E: export, 0: reset, q: quit", split_controls, tab_controls),
                };
                let controls_widget = Paragraph::new(format!("{}{}", status_info, controls))
                    .style(Style::default().fg(Color::DarkGray));
                frame.render_widget(controls_widget, controls_area);
            } else {
                // Single pane mode
                if let Some(ref pane_data) = left_pane_data {
                    // Build full title with tab bar, position info, filter, status, and controls
                    let row_info = pane_data.selected_row
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
                        format!(" Col {}/{}{}", pane_data.selected_visible_col + 1, pane_data.visible_count, hidden_info)
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

                    let (context_label, controls): (&str, String) = match current_view {
                        ViewMode::TableList => ("Tables", format!("{}{}Enter: select, /: filter, q: quit", split_controls, tab_controls)),
                        ViewMode::TableData => {
                            let label = table_name.as_deref().unwrap_or("Query Result");
                            (label, format!("{}{}+/-: width, H/S: hide/show, </>: move, E: export, 0: reset, Esc: back, q: quit", split_controls, tab_controls))
                        }
                        ViewMode::PipeData => ("Data", format!("{}{}+/-: width, H/S: hide/show, </>: move, E: export, 0: reset, q: quit", split_controls, tab_controls)),
                    };

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
                let input_area = chunks[1];
                let (prefix, style) = match mode {
                    AppMode::QueryInput => (":", Style::default().fg(Color::Cyan)),
                    AppMode::SearchInput => ("/", Style::default().fg(Color::Yellow)),
                    AppMode::ExportFilename => ("Save as: ", Style::default().fg(Color::Green)),
                    AppMode::Normal | AppMode::ExportFormat => ("", Style::default()),
                };

                let input_text = format!("{}{}", prefix, input_buf);
                let input_widget = Paragraph::new(input_text)
                    .style(style)
                    .block(Block::default().borders(Borders::ALL));

                frame.render_widget(input_widget, input_area);
            }

            // Render format selection prompt
            if show_format_prompt {
                let prompt_area = chunks[1];
                let prompt_text = "Export format: [C]SV or [J]SON (Esc to cancel)";
                let prompt_widget = Paragraph::new(prompt_text)
                    .style(Style::default().fg(Color::Green))
                    .block(Block::default().borders(Borders::ALL));

                frame.render_widget(prompt_widget, prompt_area);
            }
        })?;

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

                // Get fresh mutable reference to focused tab for event handling
                // In split mode, this respects focus_left; otherwise it's the active tab
                let tab = workspace.focused_tab_mut().unwrap();

                match current_mode {
                    AppMode::Normal => {
                        match key.code {
                            // Quit on 'q' or Ctrl+C
                            KeyCode::Char('q') => break,
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                break
                            }

                            // Enter: Select table in TableList mode
                            KeyCode::Enter => {
                                if tab.view_mode == ViewMode::TableList {
                                    if let Some(ref mut client) = db_client {
                                        if let Some(selected) = tab.table_state.selected() {
                                            // Get table name from selected row (first column)
                                            // Recalculate display_rows for event handling
                                            let filter_lower = tab.filter_text.to_lowercase();
                                            let display_rows: Vec<&Vec<String>> = if tab.filter_text.is_empty() {
                                                tab.data.rows.iter().collect()
                                            } else {
                                                tab.data.rows.iter()
                                                    .filter(|row| row.iter().any(|cell| cell.to_lowercase().contains(&filter_lower)))
                                                    .collect()
                                            };
                                            if let Some(row) = display_rows.get(selected) {
                                                if let Some(tbl_name) = row.first() {
                                                    let query = format!(
                                                        "SELECT * FROM \"{}\" LIMIT 1000",
                                                        tbl_name
                                                    );
                                                    match db::execute_query(client, &query) {
                                                        Ok(data) => {
                                                            if data.headers.is_empty()
                                                                && data.rows.is_empty()
                                                            {
                                                                status_message = Some(
                                                                    "Table is empty".to_string(),
                                                                );
                                                                status_message_time =
                                                                    Some(Instant::now());
                                                            } else {
                                                                current_table_name =
                                                                    Some(tbl_name.clone());
                                                                // Queue tab creation (deferred to avoid borrow conflict)
                                                                pending_action = PendingAction::CreateTab {
                                                                    name: tbl_name.clone(),
                                                                    data,
                                                                    view_mode: ViewMode::TableData,
                                                                };
                                                            }
                                                        }
                                                        Err(e) => {
                                                            status_message =
                                                                Some(format!("Error: {}", e));
                                                            status_message_time =
                                                                Some(Instant::now());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Esc: Go back to table list from TableData mode
                            KeyCode::Esc => {
                                if tab.view_mode == ViewMode::TableData {
                                    if let Some(ref cached) = table_list_cache {
                                        tab.data = cached.clone();
                                        tab.column_config = ColumnConfig::new(tab.data.headers.len());
                                        tab.scroll_col_offset = 0;
                                        tab.selected_visible_col = 0;
                                        tab.table_state = TableState::default().with_selected(Some(0));
                                        tab.filter_text.clear();
                                        current_table_name = None;
                                        tab.view_mode = ViewMode::TableList;
                                    }
                                }
                            }

                            // Enter query input mode (only in DB modes, not pipe)
                            KeyCode::Char(':') => {
                                if db_client.is_some() {
                                    current_mode = AppMode::QueryInput;
                                    input_buffer.clear();
                                } else {
                                    status_message =
                                        Some("Query mode requires --connect".to_string());
                                    status_message_time = Some(Instant::now());
                                }
                            }

                            // Enter search input mode
                            KeyCode::Char('/') => {
                                current_mode = AppMode::SearchInput;
                                input_buffer.clear();
                            }

                            // Vertical navigation (bounded by displayed row count)
                            KeyCode::Char('j') | KeyCode::Down => {
                                if let Some(selected) = tab.table_state.selected() {
                                    if selected + 1 < displayed_row_count {
                                        tab.table_state.select(Some(selected + 1));
                                    }
                                }
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                if let Some(selected) = tab.table_state.selected() {
                                    if selected > 0 {
                                        tab.table_state.select(Some(selected - 1));
                                    }
                                }
                            }

                            // Jump to first/last (bounded by displayed row count)
                            KeyCode::Char('g') | KeyCode::Home => tab.table_state.select(Some(0)),
                            KeyCode::Char('G') | KeyCode::End => {
                                if displayed_row_count > 0 {
                                    tab.table_state.select(Some(displayed_row_count - 1));
                                }
                            }

                            // Horizontal column navigation with scrolling
                            KeyCode::Char('h') | KeyCode::Left => {
                                if tab.selected_visible_col > 0 {
                                    tab.selected_visible_col -= 1;
                                    // Scroll left if selected column is before scroll window
                                    if tab.selected_visible_col < tab.scroll_col_offset {
                                        tab.scroll_col_offset = tab.selected_visible_col;
                                    }
                                }
                            }
                            KeyCode::Char('l') | KeyCode::Right => {
                                let visible_cols = tab.column_config.visible_indices();
                                if tab.selected_visible_col + 1 < visible_cols.len() {
                                    tab.selected_visible_col += 1;
                                    // Scroll right will be handled in render loop when needed
                                }
                            }

                            // Page navigation (half-page like vim, bounded by displayed count)
                            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                if let Some(selected) = tab.table_state.selected() {
                                    let new_pos = selected.saturating_sub(10);
                                    tab.table_state.select(Some(new_pos));
                                }
                            }
                            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                if let Some(selected) = tab.table_state.selected() {
                                    let new_pos =
                                        (selected + 10).min(displayed_row_count.saturating_sub(1));
                                    tab.table_state.select(Some(new_pos));
                                }
                            }
                            // Also support Page Up/Page Down
                            KeyCode::PageUp => {
                                if let Some(selected) = tab.table_state.selected() {
                                    let new_pos = selected.saturating_sub(10);
                                    tab.table_state.select(Some(new_pos));
                                }
                            }
                            KeyCode::PageDown => {
                                if let Some(selected) = tab.table_state.selected() {
                                    let new_pos =
                                        (selected + 10).min(displayed_row_count.saturating_sub(1));
                                    tab.table_state.select(Some(new_pos));
                                }
                            }

                            // Column width adjustment (+ and - keys)
                            KeyCode::Char('+') | KeyCode::Char('=') => {
                                let visible = tab.column_config.visible_indices();
                                if tab.selected_visible_col < visible.len() {
                                    let data_idx = visible[tab.selected_visible_col];
                                    let auto_widths = calculate_auto_widths(&tab.data);
                                    let auto_width = auto_widths.get(data_idx).copied().unwrap_or(10);
                                    tab.column_config.adjust_width(data_idx, 2, auto_width);
                                }
                            }
                            KeyCode::Char('-') | KeyCode::Char('_') => {
                                let visible = tab.column_config.visible_indices();
                                if tab.selected_visible_col < visible.len() {
                                    let data_idx = visible[tab.selected_visible_col];
                                    let auto_widths = calculate_auto_widths(&tab.data);
                                    let auto_width = auto_widths.get(data_idx).copied().unwrap_or(10);
                                    tab.column_config.adjust_width(data_idx, -2, auto_width);
                                }
                            }
                            // Reset column widths to auto (also shows hidden columns and scroll)
                            KeyCode::Char('0') => {
                                tab.column_config.reset();
                                tab.scroll_col_offset = 0;
                                tab.selected_visible_col = 0;
                            }

                            // Hide selected column (H key, uppercase to avoid conflict with h/left)
                            KeyCode::Char('H') => {
                                // Don't allow hiding if only one column visible
                                if tab.column_config.visible_count() > 1 {
                                    let visible = tab.column_config.visible_indices();
                                    if tab.selected_visible_col < visible.len() {
                                        let data_idx = visible[tab.selected_visible_col];
                                        tab.column_config.hide(data_idx);
                                        // If we hid the last visible column, select previous
                                        let new_visible = tab.column_config.visible_indices();
                                        if tab.selected_visible_col >= new_visible.len() && tab.selected_visible_col > 0 {
                                            tab.selected_visible_col -= 1;
                                        }
                                    }
                                }
                            }

                            // Show all hidden columns (S key)
                            KeyCode::Char('S') => {
                                tab.column_config.show_all();
                            }

                            // Move column left (<)
                            KeyCode::Char('<') | KeyCode::Char(',') => {
                                let visible = tab.column_config.visible_indices();
                                if tab.selected_visible_col > 0 && tab.selected_visible_col < visible.len() {
                                    // Swap this column with previous in display order
                                    let this_idx = visible[tab.selected_visible_col];
                                    let prev_idx = visible[tab.selected_visible_col - 1];
                                    let this_pos = tab.column_config.display_position(this_idx).unwrap();
                                    let prev_pos = tab.column_config.display_position(prev_idx).unwrap();
                                    // Direct swap in display_order vec
                                    tab.column_config.swap_display(this_pos, prev_pos);
                                    // Selection follows the moved column
                                    tab.selected_visible_col -= 1;
                                    if tab.selected_visible_col < tab.scroll_col_offset {
                                        tab.scroll_col_offset = tab.selected_visible_col;
                                    }
                                }
                            }

                            // Move column right (>)
                            KeyCode::Char('>') | KeyCode::Char('.') => {
                                let visible = tab.column_config.visible_indices();
                                if tab.selected_visible_col + 1 < visible.len() {
                                    // Swap this column with next in display order
                                    let this_idx = visible[tab.selected_visible_col];
                                    let next_idx = visible[tab.selected_visible_col + 1];
                                    let this_pos = tab.column_config.display_position(this_idx).unwrap();
                                    let next_pos = tab.column_config.display_position(next_idx).unwrap();
                                    // Direct swap in display_order vec
                                    tab.column_config.swap_display(this_pos, next_pos);
                                    // Selection follows the moved column
                                    tab.selected_visible_col += 1;
                                }
                            }

                            // Export data (E key)
                            KeyCode::Char('E') => {
                                // Export available in TableData and PipeData modes
                                if tab.view_mode == ViewMode::TableData || tab.view_mode == ViewMode::PipeData {
                                    current_mode = AppMode::ExportFormat;
                                }
                            }

                            // Tab navigation: toggle focus in split mode, cycle tabs otherwise
                            KeyCode::Tab => {
                                if workspace.split_active {
                                    // In split view: Tab toggles focus between panes
                                    workspace.toggle_focus();
                                } else if workspace.tab_count() > 1 {
                                    workspace.next_tab();
                                }
                            }
                            KeyCode::BackTab => {
                                // Shift+Tab: same as Tab (toggle focus in split, prev tab otherwise)
                                if workspace.split_active {
                                    workspace.toggle_focus();
                                } else if workspace.tab_count() > 1 {
                                    workspace.prev_tab();
                                }
                            }

                            // Direct tab selection with number keys 1-9
                            KeyCode::Char(c @ '1'..='9') => {
                                let idx = (c as usize) - ('1' as usize);
                                if idx < workspace.tab_count() {
                                    workspace.switch_to(idx);
                                }
                            }

                            // Close focused tab with W (uppercase)
                            KeyCode::Char('W') => {
                                if workspace.tab_count() > 1 {
                                    let idx = workspace.focused_idx();
                                    workspace.close_tab(idx);
                                }
                            }

                            // Toggle split view with V (uppercase)
                            KeyCode::Char('V') => {
                                workspace.toggle_split();
                            }

                            // Switch focus between panes with Ctrl+W or F6
                            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                workspace.toggle_focus();
                            }
                            KeyCode::F(6) => {
                                workspace.toggle_focus();
                            }

                            _ => {}
                        }
                    }

                    AppMode::QueryInput => {
                        match key.code {
                            // Cancel and return to normal mode
                            KeyCode::Esc => {
                                current_mode = AppMode::Normal;
                                input_buffer.clear();
                            }

                            // Execute query and return to normal mode
                            KeyCode::Enter => {
                                if let Some(ref mut client) = db_client {
                                    // Execute query via database client
                                    let query_str = input_buffer.trim().to_string();
                                    if !query_str.is_empty() {
                                        match db::execute_query(client, &query_str) {
                                            Ok(data) => {
                                                if data.headers.is_empty() && data.rows.is_empty() {
                                                    status_message = Some(
                                                        "Query returned no results".to_string(),
                                                    );
                                                    status_message_time = Some(Instant::now());
                                                } else {
                                                    // Generate tab name from query (truncate if long)
                                                    let tab_name = {
                                                        let q = query_str.trim();
                                                        if q.len() > 20 {
                                                            format!("{}...", &q[..17])
                                                        } else {
                                                            q.to_string()
                                                        }
                                                    };

                                                    // Queue tab creation (deferred to avoid borrow conflict)
                                                    pending_action = PendingAction::CreateTab {
                                                        name: tab_name,
                                                        data,
                                                        view_mode: ViewMode::TableData,
                                                    };
                                                }
                                            }
                                            Err(e) => {
                                                status_message = Some(format!("Error: {}", e));
                                                status_message_time = Some(Instant::now());
                                            }
                                        }
                                    }
                                } else {
                                    // Not in database mode
                                    status_message =
                                        Some("Query mode requires --connect".to_string());
                                    status_message_time = Some(Instant::now());
                                }
                                current_mode = AppMode::Normal;
                                input_buffer.clear();
                            }

                            // Text input
                            KeyCode::Char(c) => {
                                input_buffer.push(c);
                            }

                            // Backspace
                            KeyCode::Backspace => {
                                input_buffer.pop();
                            }

                            _ => {}
                        }
                    }

                    AppMode::SearchInput => {
                        match key.code {
                            // Cancel and return to normal mode
                            KeyCode::Esc => {
                                current_mode = AppMode::Normal;
                                input_buffer.clear();
                            }

                            // Apply filter and return to normal mode
                            KeyCode::Enter => {
                                // Set or clear filter based on input
                                tab.filter_text = input_buffer.trim().to_string();
                                // Reset selection to 0 when filter changes
                                tab.table_state = TableState::default().with_selected(Some(0));
                                current_mode = AppMode::Normal;
                                input_buffer.clear();
                            }

                            // Text input
                            KeyCode::Char(c) => {
                                input_buffer.push(c);
                            }

                            // Backspace
                            KeyCode::Backspace => {
                                input_buffer.pop();
                            }

                            _ => {}
                        }
                    }

                    AppMode::ExportFormat => {
                        match key.code {
                            // Cancel and return to normal mode
                            KeyCode::Esc => {
                                current_mode = AppMode::Normal;
                            }

                            // Select CSV format
                            KeyCode::Char('c') | KeyCode::Char('C') => {
                                export_format = Some(export::ExportFormat::Csv);
                                input_buffer = "export.csv".to_string();
                                current_mode = AppMode::ExportFilename;
                            }

                            // Select JSON format
                            KeyCode::Char('j') | KeyCode::Char('J') => {
                                export_format = Some(export::ExportFormat::Json);
                                input_buffer = "export.json".to_string();
                                current_mode = AppMode::ExportFilename;
                            }

                            _ => {}
                        }
                    }

                    AppMode::ExportFilename => {
                        match key.code {
                            // Cancel and return to normal mode
                            KeyCode::Esc => {
                                current_mode = AppMode::Normal;
                                input_buffer.clear();
                                export_format = None;
                            }

                            // Perform export
                            KeyCode::Enter => {
                                let filename = input_buffer.trim().to_string();
                                if !filename.is_empty() {
                                    if let Some(fmt) = export_format {
                                        let visible_cols = tab.column_config.visible_indices();
                                        match export::export_table(&tab.data, &visible_cols, fmt) {
                                            Ok(content) => {
                                                match export::save_to_file(&content, &filename) {
                                                    Ok(()) => {
                                                        status_message = Some(format!("Exported to {}", filename));
                                                        status_message_time = Some(Instant::now());
                                                    }
                                                    Err(e) => {
                                                        status_message = Some(e);
                                                        status_message_time = Some(Instant::now());
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                status_message = Some(e);
                                                status_message_time = Some(Instant::now());
                                            }
                                        }
                                    }
                                }
                                current_mode = AppMode::Normal;
                                input_buffer.clear();
                                export_format = None;
                            }

                            // Text input
                            KeyCode::Char(c) => {
                                input_buffer.push(c);
                            }

                            // Backspace
                            KeyCode::Backspace => {
                                input_buffer.pop();
                            }

                            _ => {}
                        }
                    }
                }

                // Process pending action (tab borrow has been dropped)
                if let PendingAction::CreateTab { name, data, view_mode } = pending_action {
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
