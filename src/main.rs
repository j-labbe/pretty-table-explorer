mod column;
mod db;
mod export;
mod parser;
mod update;
mod workspace;

use std::cell::Cell as StdCell;
use std::io::{self, Read};
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use column::ColumnConfig;
use parser::TableData;
use workspace::Workspace;

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
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};

/// Application mode for handling different input states.
#[derive(Clone, Copy, PartialEq)]
enum AppMode {
    Normal,         // Regular table navigation
    QueryInput,     // ':' pressed, entering SQL query
    SearchInput,    // '/' pressed, entering search filter
    ExportFormat,   // 'E' pressed, selecting export format (CSV/JSON)
    ExportFilename, // Format selected, entering filename
}

/// View mode for database browser.
#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    TableList, // Viewing list of tables (can select with Enter)
    TableData, // Viewing table contents (Esc to go back)
    PipeData,  // Viewing piped data (no back navigation)
}

/// Pending action to be executed after dropping mutable tab reference.
/// Used to avoid borrow conflicts when creating new tabs.
enum PendingAction {
    None,
    CreateTab { name: String, data: TableData },
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

/// Calculate auto-sized column widths from table data (raw values, no overrides).
/// Returns width for each column sized to fit the maximum content width + 1 for padding.
fn calculate_auto_widths(data: &TableData) -> Vec<u16> {
    let num_cols = data.headers.len();
    let mut widths = vec![0usize; num_cols];

    // Check header widths
    for (i, header) in data.headers.iter().enumerate() {
        widths[i] = widths[i].max(header.len());
    }

    // Check data row widths
    for row in &data.rows {
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    // Add 1 for padding
    widths.iter().map(|w| (*w + 1) as u16).collect()
}

/// Calculate column widths from table data.
/// Returns a Constraint for each column sized to fit the maximum content width.
/// If a ColumnConfig is provided, uses width overrides where set.
fn calculate_widths(data: &TableData, config: Option<&ColumnConfig>) -> Vec<Constraint> {
    let auto_widths = calculate_auto_widths(data);

    // Convert to Constraints, respecting config overrides
    auto_widths
        .iter()
        .enumerate()
        .map(|(i, &w)| {
            // Check for width override in config
            if let Some(cfg) = config {
                if let Some(override_width) = cfg.get_width(i) {
                    return Constraint::Length(override_width);
                }
            }
            Constraint::Length(w)
        })
        .collect()
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

    // Get table data, database client, and view mode from either database or stdin
    let (table_data, mut db_client, mut view_mode) =
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
    let table_list_cache: Option<TableData> = if view_mode == ViewMode::TableList {
        Some(table_data.clone())
    } else {
        None
    };

    // Track current table name when viewing table data
    let mut current_table_name: Option<String> = None;

    // Create workspace and add initial tab
    let mut workspace = Workspace::new();
    let tab_name = match view_mode {
        ViewMode::TableList => "Tables".to_string(),
        ViewMode::TableData => current_table_name.clone().unwrap_or_else(|| "Query".to_string()),
        ViewMode::PipeData => "Data".to_string(),
    };
    workspace.add_tab(tab_name, table_data);

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
        let tab_bar = if tab_count > 1 {
            let names: Vec<String> = workspace.tabs.iter().enumerate().map(|(i, t)| {
                // Truncate long tab names to prevent title overflow
                let name = if t.name.len() > 15 {
                    format!("{}...", &t.name[..12])
                } else {
                    t.name.clone()
                };
                // Show index number (1-based) with each tab name
                if i == workspace.active_idx {
                    format!("[{}:{}]", i + 1, name)
                } else {
                    format!("{}:{}", i + 1, name)
                }
            }).collect();
            format!("{} | ", names.join(" "))
        } else {
            String::new()
        };

        // Get active tab for this iteration (must exist since we added one above)
        let tab = workspace.active_tab_mut().unwrap();

        // Calculate widths using active tab's data and config
        let widths = calculate_widths(&tab.data, Some(&tab.column_config));

        // Capture state needed for rendering (to avoid borrow issues)
        let mode = current_mode;
        let input_buf = input_buffer.clone();
        let filter = tab.filter_text.clone();
        let status = status_message.clone();

        // Calculate filtered rows outside draw closure so we can use count for navigation
        let filter_lower = filter.to_lowercase();
        let display_rows: Vec<&Vec<String>> = if filter.is_empty() {
            tab.data.rows.iter().collect()
        } else {
            tab.data
                .rows
                .iter()
                .filter(|row| {
                    row.iter()
                        .any(|cell| cell.to_lowercase().contains(&filter_lower))
                })
                .collect()
        };

        let total_rows = tab.data.rows.len();
        displayed_row_count = display_rows.len();

        // Capture view mode for closure
        let current_view = view_mode;
        let table_name = current_table_name.clone();

        // Get visible column indices for rendering
        let visible_cols = tab.column_config.visible_indices();
        let visible_count = tab.column_config.visible_count();
        let hidden_count = tab.data.headers.len() - visible_count;

        // Clamp selected_visible_col and scroll_col_offset to valid range
        if !visible_cols.is_empty() {
            if tab.selected_visible_col >= visible_cols.len() {
                tab.selected_visible_col = visible_cols.len() - 1;
            }
            if tab.scroll_col_offset >= visible_cols.len() {
                tab.scroll_col_offset = visible_cols.len() - 1;
            }
            // Ensure selected column is not before scroll offset
            if tab.selected_visible_col < tab.scroll_col_offset {
                tab.scroll_col_offset = tab.selected_visible_col;
            }
            // Scroll right if selected column is beyond last visible
            // (uses last_visible_col_idx from previous frame)
            while tab.selected_visible_col > last_visible_col_idx.get() && tab.scroll_col_offset < visible_cols.len() - 1 {
                tab.scroll_col_offset += 1;
                // Update to prevent infinite loop
                last_visible_col_idx.set(tab.selected_visible_col);
            }
        }

        // Capture scroll state for rendering
        let scroll_col_offset = tab.scroll_col_offset;
        let selected_visible_col = tab.selected_visible_col;

        // Capture headers reference for draw closure
        let headers = &tab.data.headers;

        // Get mutable reference to table_state for rendering
        let table_state = &mut tab.table_state;

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

            // Calculate available width for columns (subtract borders and highlight symbol)
            let available_width = table_area.width.saturating_sub(2 + 3); // 2 for borders, 3 for ">> "

            // Determine which columns fit in the viewport starting from scroll_col_offset
            let mut render_cols: Vec<usize> = Vec::new();
            let mut cumulative_width: u16 = 0;
            let mut last_render_idx = scroll_col_offset;

            for (vis_idx, &data_idx) in visible_cols.iter().enumerate().skip(scroll_col_offset) {
                let col_width = match widths.get(data_idx) {
                    Some(Constraint::Length(w)) => *w,
                    _ => 10, // fallback
                };
                if cumulative_width + col_width <= available_width || render_cols.is_empty() {
                    // Always include at least one column
                    render_cols.push(data_idx);
                    cumulative_width += col_width + 1; // +1 for column separator
                    last_render_idx = vis_idx;
                } else {
                    break;
                }
            }

            // Track overflow states
            let has_left_overflow = scroll_col_offset > 0;
            let has_right_overflow = last_render_idx + 1 < visible_cols.len();

            // Update last visible column index for scroll-right detection in next frame
            last_visible_col_idx.set(last_render_idx);

            // Calculate relative column position for table_state
            let render_col_position = selected_visible_col.saturating_sub(scroll_col_offset);
            // Clamp to render_cols range
            let render_col_position = render_col_position.min(render_cols.len().saturating_sub(1));

            // Create dynamic title showing position
            let row_info = table_state
                .selected()
                .map(|r| {
                    if filter.is_empty() {
                        format!("Row {}/{}", r + 1, total_rows)
                    } else {
                        format!(
                            "Row {}/{} (filtered from {})",
                            r + 1,
                            displayed_row_count,
                            total_rows
                        )
                    }
                })
                .unwrap_or_default();

            let hidden_info = if hidden_count > 0 {
                format!(" ({} hidden)", hidden_count)
            } else {
                String::new()
            };
            // Show global column position (selected_visible_col is 0-indexed)
            let col_info = if !visible_cols.is_empty() {
                format!(" Col {}/{}{}", selected_visible_col + 1, visible_count, hidden_info)
            } else {
                String::new()
            };

            let position = if row_info.is_empty() {
                String::new()
            } else {
                format!("[{}{}] ", row_info, col_info)
            };

            // Show filter in title if active
            let filter_info = if !filter.is_empty() {
                format!("/{} ", filter)
            } else {
                String::new()
            };

            // Show status message or error in title if present
            let status_info = status
                .as_ref()
                .map(|s| format!("{} ", s))
                .unwrap_or_default();

            // Build overflow indicators
            let left_indicator = if has_left_overflow { "◀ " } else { "" };
            let right_indicator = if has_right_overflow { " ▶" } else { "" };

            // Build context-appropriate title
            // Add tab controls when multiple tabs exist (include 1-9 hint for direct selection)
            let tab_controls = if tab_count > 1 { "1-9: tab, W: close, " } else { "" };
            let (context_label, controls): (&str, String) = match current_view {
                ViewMode::TableList => ("Tables", format!("{}Enter: select, /: filter, q: quit", tab_controls)),
                ViewMode::TableData => {
                    let label = table_name.as_deref().unwrap_or("Query Result");
                    (label, format!("{}+/-: width, H/S: hide/show, </>: move, E: export, 0: reset, Esc: back, q: quit", tab_controls))
                }
                ViewMode::PipeData => ("Data", format!("{}+/-: width, H/S: hide/show, </>: move, E: export, 0: reset, q: quit", tab_controls)),
            };

            let title = format!(
                " {}{}{}{} {} {}{}{} ",
                tab_bar, left_indicator, context_label, right_indicator, position, filter_info, status_info, controls
            );

            // Create header row with bold style (only columns in scroll window)
            let header_cells = render_cols.iter().map(|&i| {
                Cell::from(headers[i].as_str())
                    .style(Style::default().add_modifier(Modifier::BOLD))
            });
            let header_row = Row::new(header_cells).style(Style::default().fg(Color::Yellow));

            // Create data rows from filtered set (only columns in scroll window)
            let data_rows = display_rows.iter().map(|row| {
                let cells = render_cols.iter().map(|&i| {
                    Cell::from(row.get(i).map(|s| s.as_str()).unwrap_or(""))
                });
                Row::new(cells)
            });

            // Build widths for columns in scroll window
            let render_widths: Vec<Constraint> = render_cols
                .iter()
                .map(|&i| widths[i])
                .collect();

            // Build table with calculated widths
            let table = Table::new(data_rows, render_widths)
                .header(header_row)
                .block(Block::default().title(title).borders(Borders::ALL))
                .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .column_highlight_style(Style::default().fg(Color::Cyan))
                .highlight_symbol(">> ");

            // Sync table_state's column selection with our scroll-aware position
            table_state.select_column(Some(render_col_position));

            frame.render_stateful_widget(table, table_area, &mut *table_state);

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

                // Get fresh mutable reference to active tab for event handling
                let tab = workspace.active_tab_mut().unwrap();

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
                                if view_mode == ViewMode::TableList {
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
                                if view_mode == ViewMode::TableData {
                                    if let Some(ref cached) = table_list_cache {
                                        tab.data = cached.clone();
                                        tab.column_config = ColumnConfig::new(tab.data.headers.len());
                                        tab.scroll_col_offset = 0;
                                        tab.selected_visible_col = 0;
                                        tab.table_state = TableState::default().with_selected(Some(0));
                                        tab.filter_text.clear();
                                        current_table_name = None;
                                        view_mode = ViewMode::TableList;
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
                                if view_mode == ViewMode::TableData || view_mode == ViewMode::PipeData {
                                    current_mode = AppMode::ExportFormat;
                                }
                            }

                            // Tab navigation: cycle with Tab/Shift+Tab
                            KeyCode::Tab => {
                                if workspace.tab_count() > 1 {
                                    workspace.next_tab();
                                }
                            }
                            KeyCode::BackTab => {
                                // Shift+Tab
                                if workspace.tab_count() > 1 {
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

                            // Close current tab with W (uppercase)
                            KeyCode::Char('W') => {
                                if workspace.tab_count() > 1 {
                                    workspace.close_tab(workspace.active_idx);
                                }
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
                if let PendingAction::CreateTab { name, data } = pending_action {
                    let new_idx = workspace.add_tab(name, data);
                    workspace.switch_to(new_idx);
                    view_mode = ViewMode::TableData;
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
