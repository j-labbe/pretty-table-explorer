mod db;
mod parser;

use std::env;
use std::io::{self, Read};
use std::time::Duration;

use parser::TableData;

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
fn calculate_widths(data: &TableData) -> Vec<Constraint> {
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

    // Convert to Constraints (add 1 for padding)
    widths
        .iter()
        .map(|w| Constraint::Length((*w + 1) as u16))
        .collect()
}

/// Print usage information and exit.
fn print_usage() {
    eprintln!("Usage: pretty-table-explorer [OPTIONS]");
    eprintln!("       cat data.txt | pretty-table-explorer");
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

/// Parse command-line arguments.
/// Returns (connection_string, query) if --connect is provided.
fn parse_args() -> Option<(String, String)> {
    let args: Vec<String> = env::args().collect();
    let mut connection_string: Option<String> = None;
    let mut query: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--connect" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --connect requires a connection string argument");
                    print_usage();
                }
                connection_string = Some(args[i + 1].clone());
                i += 2;
            }
            "--query" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --query requires a SQL query argument");
                    print_usage();
                }
                query = Some(args[i + 1].clone());
                i += 2;
            }
            "--help" | "-h" => {
                print_usage();
            }
            _ => {
                eprintln!("Error: Unknown argument: {}", args[i]);
                print_usage();
            }
        }
    }

    // If --connect provided, return connection info with default query
    connection_string.map(|conn| {
        let default_query =
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' LIMIT 20"
                .to_string();
        (conn, query.unwrap_or(default_query))
    })
}

fn main() -> io::Result<()> {
    // Parse CLI arguments
    let db_config = parse_args();

    // Get table data and optional database client from either database or stdin
    let (mut table_data, mut db_client) = if let Some((conn_string, query)) = db_config {
        // Direct database connection mode
        match db::connect(&conn_string) {
            Ok(mut client) => match db::execute_query(&mut client, &query) {
                Ok(data) => {
                    if data.headers.is_empty() && data.rows.is_empty() {
                        eprintln!("Query returned no results.");
                        std::process::exit(0);
                    }
                    (data, Some(client))
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
                    eprintln!(
                        "Error: Could not connect to PostgreSQL at specified host."
                    );
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
            Some(data) => (data, None),
            None => {
                eprintln!("Error: Invalid or empty input. Expected psql table format.");
                eprintln!("Usage: psql -c 'SELECT ...' | pretty-table-explorer");
                std::process::exit(1);
            }
        }
    };

    // Calculate column widths (recalculated when data changes)
    let mut widths = calculate_widths(&table_data);

    // Application state for input modes
    let mut current_mode = AppMode::Normal;
    let mut input_buffer = String::new();
    let mut filter_text = String::new();
    let mut status_message: Option<String> = None;

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

    // Main event loop
    loop {
        // Capture state needed for rendering (to avoid borrow issues)
        let mode = current_mode;
        let input_buf = input_buffer.clone();
        let filter = filter_text.clone();
        let status = status_message.clone();

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
                .map(|r| format!("Row {}/{}", r + 1, table_data.rows.len()))
                .unwrap_or_default();

            let col_info = table_state
                .selected_column()
                .map(|c| format!(" Col {}/{}", c + 1, table_data.headers.len()))
                .unwrap_or_default();

            let position = if row_info.is_empty() {
                String::new()
            } else {
                format!(" [{}{}] ", row_info, col_info)
            };

            // Show filter in title if active
            let filter_info = if !filter.is_empty() {
                format!(" / {} ", filter)
            } else {
                String::new()
            };

            // Show status message or error in title if present
            let status_info = status.as_ref().map(|s| format!(" {} ", s)).unwrap_or_default();

            let title = format!(
                " Pretty Table Explorer{}{}{}- hjkl: nav, :/: query/search, q: quit ",
                position, filter_info, status_info
            );

            // Create header row with bold style
            let header_cells = table_data
                .headers
                .iter()
                .map(|h| Cell::from(h.as_str()).style(Style::default().add_modifier(Modifier::BOLD)));
            let header_row = Row::new(header_cells).style(Style::default().fg(Color::Yellow));

            // Create data rows
            let data_rows = table_data.rows.iter().map(|row| {
                let cells = row.iter().map(|c| Cell::from(c.as_str()));
                Row::new(cells)
            });

            // Build table with calculated widths
            let table = Table::new(data_rows, widths.clone())
                .header(header_row)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL),
                )
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

        // Clear status message after rendering
        status_message = None;

        // Poll with 250ms timeout for responsive feel
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match current_mode {
                    AppMode::Normal => {
                        match key.code {
                            // Quit on 'q' or Ctrl+C
                            KeyCode::Char('q') => break,
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,

                            // Enter query input mode
                            KeyCode::Char(':') => {
                                current_mode = AppMode::QueryInput;
                                input_buffer.clear();
                            }

                            // Enter search input mode
                            KeyCode::Char('/') => {
                                current_mode = AppMode::SearchInput;
                                input_buffer.clear();
                            }

                            // Vertical navigation
                            KeyCode::Char('j') | KeyCode::Down => table_state.select_next(),
                            KeyCode::Char('k') | KeyCode::Up => table_state.select_previous(),

                            // Jump to first/last
                            KeyCode::Char('g') | KeyCode::Home => table_state.select_first(),
                            KeyCode::Char('G') | KeyCode::End => table_state.select_last(),

                            // Horizontal column navigation
                            KeyCode::Char('h') | KeyCode::Left => table_state.select_previous_column(),
                            KeyCode::Char('l') | KeyCode::Right => table_state.select_next_column(),

                            // Page navigation (half-page like vim)
                            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                table_state.scroll_up_by(10);
                            }
                            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                table_state.scroll_down_by(10);
                            }
                            // Also support Page Up/Page Down
                            KeyCode::PageUp => table_state.scroll_up_by(10),
                            KeyCode::PageDown => table_state.scroll_down_by(10),

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
                                                    status_message = Some("Query returned no results".to_string());
                                                } else {
                                                    // Update table data and recalculate widths
                                                    table_data = data;
                                                    widths = calculate_widths(&table_data);
                                                    table_state = TableState::default().with_selected(Some(0));
                                                    filter_text.clear(); // Clear filter when new data loaded
                                                }
                                            }
                                            Err(e) => {
                                                status_message = Some(format!("Error: {}", e));
                                            }
                                        }
                                    }
                                } else {
                                    // Not in database mode
                                    status_message = Some("Query mode requires --connect".to_string());
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
                                // Filter will be implemented in Task 3
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
