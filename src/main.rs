mod parser;

use std::io::{self, Read};
use std::time::Duration;

// TableData will be used for rendering in 02-02
#[allow(unused_imports)]
use parser::TableData;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
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
fn calculate_widths(data: &parser::TableData) -> Vec<Constraint> {
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

    // table_data will be used for rendering in 02-02
    let _table_data = match parser::parse_psql(&input) {
        Some(data) => data,
        None => {
            eprintln!("Error: Invalid or empty input. Expected psql table format.");
            eprintln!("Usage: psql -c 'SELECT ...' | pretty-table-explorer");
            std::process::exit(1);
        }
    };

    // Print parsing summary (temporary - will be replaced by table rendering in 02-02)
    eprintln!(
        "Parsed table: {} columns, {} rows",
        _table_data.column_count(),
        _table_data.row_count()
    );

    // Set up panic hook to restore terminal on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    let mut terminal = init_terminal()?;

    // Main event loop
    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            // Create centered layout
            let vertical = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(5),
                Constraint::Fill(1),
            ])
            .split(area);

            let horizontal = Layout::horizontal([
                Constraint::Fill(1),
                Constraint::Length(40),
                Constraint::Fill(1),
            ])
            .split(vertical[1]);

            // Create block with title and borders
            let block = Block::default()
                .title(" Pretty Table Explorer ")
                .borders(Borders::ALL);

            // Create paragraph with quit instruction
            let paragraph = Paragraph::new("Press 'q' to quit")
                .block(block)
                .alignment(Alignment::Center);

            frame.render_widget(paragraph, horizontal[1]);
        })?;

        // Poll with 250ms timeout for responsive feel
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                // Quit on 'q' or Ctrl+C
                if key.code == KeyCode::Char('q')
                    || (key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c'))
                {
                    break;
                }
            }
        }
    }

    // Clear terminal before exit
    terminal.clear()?;
    restore_terminal(&mut terminal)?;
    Ok(())
}
