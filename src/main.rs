mod column;
mod db;
mod parser;
mod update;

use std::io::{self, Read};
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use column::ColumnConfig;
use parser::TableData;

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
    Normal,      // Regular table navigation
    QueryInput,  // ':' pressed, entering SQL query
    SearchInput, // '/' pressed, entering search filter
}

/// View mode for database browser.
#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    TableList, // Viewing list of tables (can select with Enter)
    TableData, // Viewing table contents (Esc to go back)
    PipeData,  // Viewing piped data (no back navigation)
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

/// Calculate column widths from table data.
/// Returns a Constraint for each column sized to fit the maximum content width.
/// If a ColumnConfig is provided, uses width overrides where set.
fn calculate_widths(data: &TableData, config: Option<&ColumnConfig>) -> Vec<Constraint> {
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

    // Convert to Constraints (add 1 for padding), respecting config overrides
    widths
        .iter()
        .enumerate()
        .map(|(i, w)| {
            // Check for width override in config
            if let Some(cfg) = config {
                if let Some(override_width) = cfg.get_width(i) {
                    return Constraint::Length(override_width);
                }
            }
            Constraint::Length((*w + 1) as u16)
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
    let (mut table_data, mut db_client, mut view_mode) =
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

    // Column configuration for width overrides
    let mut column_config = ColumnConfig::new(table_data.headers.len());

    // Calculate column widths (recalculated when data changes)
    let mut widths = calculate_widths(&table_data, Some(&column_config));

    // Application state for input modes
    let mut current_mode = AppMode::Normal;
    let mut input_buffer = String::new();
    let mut filter_text = String::new();
    let mut status_message: Option<String> = None;
    let mut status_message_time: Option<Instant> = None;

    // Set up panic hook to restore terminal on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    let mut terminal = init_terminal()?;

    // Initialize table state with first row selected
    let mut table_state = TableState::default().with_selected(Some(0));

    // Track the count of displayed rows for navigation bounds
    #[allow(unused_assignments)]
    let mut displayed_row_count = 0;

    // Main event loop
    loop {
        // Capture state needed for rendering (to avoid borrow issues)
        let mode = current_mode;
        let input_buf = input_buffer.clone();
        let filter = filter_text.clone();
        let status = status_message.clone();

        // Calculate filtered rows outside draw closure so we can use count for navigation
        let filter_lower = filter.to_lowercase();
        let display_rows: Vec<&Vec<String>> = if filter.is_empty() {
            table_data.rows.iter().collect()
        } else {
            table_data
                .rows
                .iter()
                .filter(|row| {
                    row.iter()
                        .any(|cell| cell.to_lowercase().contains(&filter_lower))
                })
                .collect()
        };

        let total_rows = table_data.rows.len();
        displayed_row_count = display_rows.len();

        // Capture view mode for closure
        let current_view = view_mode;
        let table_name = current_table_name.clone();

        terminal.draw(|frame| {
            let area = frame.area();

            // Split layout: table area + optional input bar at bottom
            let show_input_bar = mode != AppMode::Normal;
            let chunks = if show_input_bar {
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

            let col_info = table_state
                .selected_column()
                .map(|c| format!(" Col {}/{}", c + 1, table_data.headers.len()))
                .unwrap_or_default();

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

            // Build context-appropriate title
            let (context_label, controls) = match current_view {
                ViewMode::TableList => ("Tables", "Enter: select, /: filter, q: quit"),
                ViewMode::TableData => {
                    let label = table_name.as_deref().unwrap_or("Query Result");
                    (label, "Esc: back, /: filter, :: query, +/-: resize col, q: quit")
                }
                ViewMode::PipeData => ("Data", "/: filter, +/-: resize col, q: quit"),
            };

            let title = format!(
                " {} {} {}{}{} ",
                context_label, position, filter_info, status_info, controls
            );

            // Create header row with bold style
            let header_cells = table_data.headers.iter().map(|h| {
                Cell::from(h.as_str()).style(Style::default().add_modifier(Modifier::BOLD))
            });
            let header_row = Row::new(header_cells).style(Style::default().fg(Color::Yellow));

            // Create data rows from filtered set
            let data_rows = display_rows.iter().map(|row| {
                let cells = row.iter().map(|c| Cell::from(c.as_str()));
                Row::new(cells)
            });

            // Build table with calculated widths
            let table = Table::new(data_rows, widths.clone())
                .header(header_row)
                .block(Block::default().title(title).borders(Borders::ALL))
                .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .column_highlight_style(Style::default().fg(Color::Cyan))
                .highlight_symbol(">> ");

            frame.render_stateful_widget(table, table_area, &mut table_state);

            // Render input bar when in input mode
            if show_input_bar {
                let input_area = chunks[1];
                let (prefix, style) = match mode {
                    AppMode::QueryInput => (":", Style::default().fg(Color::Cyan)),
                    AppMode::SearchInput => ("/", Style::default().fg(Color::Yellow)),
                    AppMode::Normal => ("", Style::default()),
                };

                let input_text = format!("{}{}", prefix, input_buf);
                let input_widget = Paragraph::new(input_text)
                    .style(style)
                    .block(Block::default().borders(Borders::ALL));

                frame.render_widget(input_widget, input_area);
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
                                        if let Some(selected) = table_state.selected() {
                                            // Get table name from selected row (first column)
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
                                                                table_data = data;
                                                                column_config = ColumnConfig::new(table_data.headers.len());
                                                                widths =
                                                                    calculate_widths(&table_data, Some(&column_config));
                                                                table_state = TableState::default()
                                                                    .with_selected(Some(0));
                                                                filter_text.clear();
                                                                view_mode = ViewMode::TableData;
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
                                        table_data = cached.clone();
                                        column_config = ColumnConfig::new(table_data.headers.len());
                                        widths = calculate_widths(&table_data, Some(&column_config));
                                        table_state = TableState::default().with_selected(Some(0));
                                        filter_text.clear();
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
                                if let Some(selected) = table_state.selected() {
                                    if selected + 1 < displayed_row_count {
                                        table_state.select(Some(selected + 1));
                                    }
                                }
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                if let Some(selected) = table_state.selected() {
                                    if selected > 0 {
                                        table_state.select(Some(selected - 1));
                                    }
                                }
                            }

                            // Jump to first/last (bounded by displayed row count)
                            KeyCode::Char('g') | KeyCode::Home => table_state.select(Some(0)),
                            KeyCode::Char('G') | KeyCode::End => {
                                if displayed_row_count > 0 {
                                    table_state.select(Some(displayed_row_count - 1));
                                }
                            }

                            // Horizontal column navigation
                            KeyCode::Char('h') | KeyCode::Left => {
                                table_state.select_previous_column()
                            }
                            KeyCode::Char('l') | KeyCode::Right => table_state.select_next_column(),

                            // Page navigation (half-page like vim, bounded by displayed count)
                            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                if let Some(selected) = table_state.selected() {
                                    let new_pos = selected.saturating_sub(10);
                                    table_state.select(Some(new_pos));
                                }
                            }
                            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                if let Some(selected) = table_state.selected() {
                                    let new_pos =
                                        (selected + 10).min(displayed_row_count.saturating_sub(1));
                                    table_state.select(Some(new_pos));
                                }
                            }
                            // Also support Page Up/Page Down
                            KeyCode::PageUp => {
                                if let Some(selected) = table_state.selected() {
                                    let new_pos = selected.saturating_sub(10);
                                    table_state.select(Some(new_pos));
                                }
                            }
                            KeyCode::PageDown => {
                                if let Some(selected) = table_state.selected() {
                                    let new_pos =
                                        (selected + 10).min(displayed_row_count.saturating_sub(1));
                                    table_state.select(Some(new_pos));
                                }
                            }

                            // Column width adjustment (+ and - keys)
                            KeyCode::Char('+') | KeyCode::Char('=') => {
                                if let Some(col) = table_state.selected_column() {
                                    column_config.adjust_width(col, 2);
                                    widths = calculate_widths(&table_data, Some(&column_config));
                                }
                            }
                            KeyCode::Char('-') | KeyCode::Char('_') => {
                                if let Some(col) = table_state.selected_column() {
                                    column_config.adjust_width(col, -2);
                                    widths = calculate_widths(&table_data, Some(&column_config));
                                }
                            }
                            // Reset column widths to auto
                            KeyCode::Char('0') => {
                                column_config.reset();
                                widths = calculate_widths(&table_data, Some(&column_config));
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
                                    let query = input_buffer.trim();
                                    if !query.is_empty() {
                                        match db::execute_query(client, query) {
                                            Ok(data) => {
                                                if data.headers.is_empty() && data.rows.is_empty() {
                                                    status_message = Some(
                                                        "Query returned no results".to_string(),
                                                    );
                                                    status_message_time = Some(Instant::now());
                                                } else {
                                                    // Update table data and recalculate widths
                                                    table_data = data;
                                                    column_config = ColumnConfig::new(table_data.headers.len());
                                                    widths = calculate_widths(&table_data, Some(&column_config));
                                                    table_state = TableState::default()
                                                        .with_selected(Some(0));
                                                    filter_text.clear(); // Clear filter when new data loaded
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
                                filter_text = input_buffer.trim().to_string();
                                // Reset selection to 0 when filter changes
                                table_state = TableState::default().with_selected(Some(0));
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
                }
            }
        }
    }

    // Clear terminal before exit
    terminal.clear()?;
    restore_terminal(&mut terminal)?;
    Ok(())
}
