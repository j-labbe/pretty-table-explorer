mod parser;

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
    widgets::{Block, Borders, Cell, Row, Table, TableState},
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

fn main() -> io::Result<()> {
    // Read and parse stdin before initializing TUI
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let table_data = match parser::parse_psql(&input) {
        Some(data) => data,
        None => {
            eprintln!("Error: Invalid or empty input. Expected psql table format.");
            eprintln!("Usage: psql -c 'SELECT ...' | pretty-table-explorer");
            std::process::exit(1);
        }
    };

    // Calculate column widths once before entering the render loop
    let widths = calculate_widths(&table_data);

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
        terminal.draw(|frame| {
            let area = frame.area();

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

            let title = format!(
                " Pretty Table Explorer{}- hjkl: nav, q: quit ",
                position
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

            frame.render_stateful_widget(table, area, &mut table_state);
        })?;

        // Poll with 250ms timeout for responsive feel
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    // Quit on 'q' or Ctrl+C
                    KeyCode::Char('q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,

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
        }
    }

    // Clear terminal before exit
    terminal.clear()?;
    restore_terminal(&mut terminal)?;
    Ok(())
}
