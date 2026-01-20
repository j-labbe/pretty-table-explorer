//! Table rendering functions for the terminal UI.
//!
//! Contains functions for calculating column widths, building render data,
//! and rendering table panes with scroll indicators.

use crate::column::ColumnConfig;
use crate::parser::TableData;
use crate::state::{AppMode, PaneRenderData};
use crate::workspace::{Tab, ViewMode, Workspace};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};
use std::cell::Cell as StdCell;

/// Calculate auto-sized column widths from table data (raw values, no overrides).
/// Returns width for each column sized to fit the maximum content width + 1 for padding.
pub(crate) fn calculate_auto_widths(data: &TableData) -> Vec<u16> {
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
pub(crate) fn calculate_widths(data: &TableData, config: Option<&ColumnConfig>) -> Vec<Constraint> {
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

/// Build render data for a tab.
pub fn build_pane_render_data(tab: &Tab) -> PaneRenderData {
    // Calculate widths using tab's data and config
    let widths = calculate_widths(&tab.data, Some(&tab.column_config));

    // Get visible column indices
    let visible_cols = tab.column_config.visible_indices();
    let visible_count = tab.column_config.visible_count();
    let hidden_count = tab.data.headers.len() - visible_count;

    // Calculate filtered rows
    let filter_lower = tab.filter_text.to_lowercase();
    let display_rows: Vec<Vec<String>> = if tab.filter_text.is_empty() {
        tab.data.rows.clone()
    } else {
        tab.data
            .rows
            .iter()
            .filter(|row| {
                row.iter()
                    .any(|cell| cell.to_lowercase().contains(&filter_lower))
            })
            .cloned()
            .collect()
    };

    PaneRenderData {
        name: tab.name.clone(),
        total_rows: tab.data.rows.len(),
        displayed_row_count: display_rows.len(),
        display_rows,
        headers: tab.data.headers.clone(),
        visible_cols,
        widths,
        filter_text: tab.filter_text.clone(),
        scroll_col_offset: tab.scroll_col_offset,
        selected_visible_col: tab.selected_visible_col,
        visible_count,
        hidden_count,
        selected_row: tab.table_state.selected(),
    }
}

/// Render a single table pane.
pub fn render_table_pane(
    frame: &mut Frame,
    area: Rect,
    pane: &PaneRenderData,
    title: String,
    is_focused: bool,
    table_state: &mut TableState,
    last_visible_col_idx: &StdCell<usize>,
) {
    // Determine if left indicator will be shown (known before column calculation)
    let has_left_overflow = pane.scroll_col_offset > 0;

    // Calculate available width for columns (subtract borders, highlight symbol, and indicators if present)
    // We need to reserve space for right indicator upfront since we don't know if we'll need it
    // until we calculate which columns fit - use a two-pass approach
    //
    // Layout: | >> | [left_ind] | col1 | col2 | ... | [right_ind] |
    // - 2 chars for left/right borders (the | characters)
    // - 3 chars for highlight symbol ">> "
    // - 1 char for left indicator (if present) + 1 for its separator
    // - 1 char for right indicator (if present) - no trailing separator needed
    let base_width = area.width.saturating_sub(2 + 3); // 2 for borders, 3 for ">> "
    let width_minus_left = if has_left_overflow {
        base_width.saturating_sub(2) // Reserve 1 char for left indicator + 1 for separator
    } else {
        base_width
    };

    // First pass: calculate with reserved right indicator space to determine if overflow exists
    // Reserve 2 chars: 1 for separator before right indicator, 1 for indicator itself
    let width_with_right_reserved = width_minus_left.saturating_sub(2);

    // Determine which columns fit in the viewport starting from scroll_col_offset
    // Track if the last column needs to be truncated (partial content for wide columns)
    let mut render_cols: Vec<usize> = Vec::new();
    let mut cumulative_width: u16 = 0;
    let mut last_render_idx = pane.scroll_col_offset;
    let mut last_col_truncated_width: Option<u16> = None;

    for (vis_idx, &data_idx) in pane
        .visible_cols
        .iter()
        .enumerate()
        .skip(pane.scroll_col_offset)
    {
        let col_width = match pane.widths.get(data_idx) {
            Some(Constraint::Length(w)) => *w,
            _ => 10, // fallback
        };
        // Check if this column fits (including its trailing separator)
        // Note: we use col_width + 1 for separator between columns
        let width_needed = if render_cols.is_empty() {
            col_width // First column doesn't need leading separator
        } else {
            col_width + 1 // Subsequent columns need separator
        };
        if cumulative_width + width_needed <= width_with_right_reserved {
            // Column fully fits
            render_cols.push(data_idx);
            cumulative_width += width_needed;
            last_render_idx = vis_idx;
        } else if render_cols.is_empty() {
            // First column is wider than viewport - show partial content
            // Use all available width for this single column
            render_cols.push(data_idx);
            last_col_truncated_width = Some(width_with_right_reserved);
            last_render_idx = vis_idx;
            break;
        } else {
            // Column doesn't fully fit, but there might be remaining space
            // Calculate remaining space (accounting for separator)
            let remaining = width_with_right_reserved.saturating_sub(cumulative_width + 1);
            if remaining >= 3 {
                // Show partial content if at least 3 chars available (meaningful preview)
                render_cols.push(data_idx);
                last_col_truncated_width = Some(remaining);
                last_render_idx = vis_idx;
            }
            break;
        }
    }

    // Check if right overflow actually exists
    let has_right_overflow = last_render_idx + 1 < pane.visible_cols.len();

    // If no right overflow, we can recalculate with the extra space
    if !has_right_overflow && width_minus_left > width_with_right_reserved {
        // Recalculate without right indicator reservation
        render_cols.clear();
        cumulative_width = 0;
        last_render_idx = pane.scroll_col_offset;
        last_col_truncated_width = None;

        for (vis_idx, &data_idx) in pane
            .visible_cols
            .iter()
            .enumerate()
            .skip(pane.scroll_col_offset)
        {
            let col_width = match pane.widths.get(data_idx) {
                Some(Constraint::Length(w)) => *w,
                _ => 10, // fallback
            };
            let width_needed = if render_cols.is_empty() {
                col_width
            } else {
                col_width + 1
            };
            if cumulative_width + width_needed <= width_minus_left {
                render_cols.push(data_idx);
                cumulative_width += width_needed;
                last_render_idx = vis_idx;
            } else if render_cols.is_empty() {
                // First column wider than viewport - show partial
                render_cols.push(data_idx);
                last_col_truncated_width = Some(width_minus_left);
                last_render_idx = vis_idx;
                break;
            } else {
                // Check for partial content opportunity
                let remaining = width_minus_left.saturating_sub(cumulative_width + 1);
                if remaining >= 3 {
                    render_cols.push(data_idx);
                    last_col_truncated_width = Some(remaining);
                    last_render_idx = vis_idx;
                }
                break;
            }
        }
    }

    // Recheck right overflow after potential recalculation
    let has_right_overflow = last_render_idx + 1 < pane.visible_cols.len();

    // Update last visible column index for scroll-right detection in next frame
    if is_focused {
        last_visible_col_idx.set(last_render_idx);
    }

    // Calculate relative column position for table_state
    // This is the position within the DATA columns (not including indicator columns)
    let data_col_position = pane
        .selected_visible_col
        .saturating_sub(pane.scroll_col_offset);
    // Clamp to render_cols range (ensures we never select beyond data columns)
    let data_col_position = data_col_position.min(render_cols.len().saturating_sub(1));

    // Adjust for left indicator column if present
    // The actual column position in the rendered table needs to account for the indicator
    // Layout of rendered cells: [left_ind?] [data_col_0] [data_col_1] ... [data_col_n] [right_ind?]
    // If left indicator present, data columns start at index 1
    // Selection should NEVER land on indicator columns (only on data columns)
    let render_col_position = if has_left_overflow {
        data_col_position + 1 // Skip the left indicator column (index 0)
    } else {
        data_col_position
    };
    // Final safety clamp: ensure we don't exceed the last data column index
    // Total rendered columns = (1 if left) + render_cols.len() + (1 if right)
    // Valid data column indices: (1 if left) to (1 if left) + render_cols.len() - 1
    let max_data_col_position = if has_left_overflow {
        render_cols.len() // Last data column at index render_cols.len() (0 is left indicator)
    } else {
        render_cols.len().saturating_sub(1) // Last data column at index render_cols.len() - 1
    };
    let render_col_position = render_col_position.min(max_data_col_position);

    // Build overflow indicators
    let left_indicator = if has_left_overflow { "◀" } else { "" };
    let right_indicator = if has_right_overflow { "▶" } else { "" };

    // Build final title with overflow indicators
    let full_title = format!(" {}{}{} {} ", left_indicator, title, right_indicator, " ");

    // Style for indicator cells
    let indicator_style = Style::default().bg(Color::DarkGray).fg(Color::Gray);

    // Create header row with bold style (only columns in scroll window)
    // Prepend/append indicator cells if needed
    let mut header_cells: Vec<Cell> = Vec::new();
    if has_left_overflow {
        header_cells.push(Cell::from(" ").style(indicator_style));
    }
    for &i in &render_cols {
        header_cells.push(
            Cell::from(pane.headers[i].as_str())
                .style(Style::default().add_modifier(Modifier::BOLD)),
        );
    }
    if has_right_overflow {
        header_cells.push(Cell::from(" ").style(indicator_style));
    }
    let header_row = Row::new(header_cells).style(Style::default().fg(Color::Yellow));

    // Create data rows from filtered set (only columns in scroll window)
    // Prepend/append indicator cells if needed
    let data_rows: Vec<Row> = pane
        .display_rows
        .iter()
        .map(|row| {
            let mut cells: Vec<Cell> = Vec::new();
            if has_left_overflow {
                cells.push(Cell::from("◀").style(indicator_style));
            }
            for &i in &render_cols {
                cells.push(Cell::from(row.get(i).map(|s| s.as_str()).unwrap_or("")));
            }
            if has_right_overflow {
                cells.push(Cell::from("▶").style(indicator_style));
            }
            Row::new(cells)
        })
        .collect();

    // Build widths for columns in scroll window
    // Prepend/append indicator widths if needed
    // If right overflow exists, use Fill(1) for the last data column to push indicator to edge
    let mut render_widths: Vec<Constraint> = Vec::new();
    if has_left_overflow {
        render_widths.push(Constraint::Length(1));
    }
    let last_data_col_idx = render_cols.len().saturating_sub(1);
    for (idx, &i) in render_cols.iter().enumerate() {
        let is_last_data_col = idx == last_data_col_idx;
        if is_last_data_col && has_right_overflow {
            // Use Fill(1) for last data column when right indicator exists
            // This makes the column expand to fill remaining space, pushing indicator to edge
            render_widths.push(Constraint::Fill(1));
        } else if is_last_data_col && last_col_truncated_width.is_some() {
            // Use truncated width for partially displayed column
            render_widths.push(Constraint::Length(last_col_truncated_width.unwrap()));
        } else {
            render_widths.push(pane.widths[i]);
        }
    }
    if has_right_overflow {
        render_widths.push(Constraint::Length(1));
    }

    // Build border style based on focus
    let border_style = if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // Build table with calculated widths
    let table = Table::new(data_rows, render_widths)
        .header(header_row)
        .block(
            Block::default()
                .title(full_title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .column_highlight_style(Style::default().fg(Color::Cyan))
        .highlight_symbol(">> ");

    // Sync table_state's column selection with our scroll-aware position
    table_state.select_column(Some(render_col_position));

    frame.render_stateful_widget(table, area, table_state);
}

/// Build a title for a pane in split view.
pub fn build_pane_title(
    pane: &PaneRenderData,
    _table_name: &Option<String>,
    _view_mode: ViewMode,
    is_focused: bool,
) -> String {
    // Build position info
    let row_info = pane
        .selected_row
        .map(|r| {
            if pane.filter_text.is_empty() {
                format!("Row {}/{}", r + 1, pane.total_rows)
            } else {
                format!(
                    "Row {}/{} (from {})",
                    r + 1,
                    pane.displayed_row_count,
                    pane.total_rows
                )
            }
        })
        .unwrap_or_default();

    let hidden_info = if pane.hidden_count > 0 {
        format!(" ({} hid)", pane.hidden_count)
    } else {
        String::new()
    };
    let col_info = if !pane.visible_cols.is_empty() {
        format!(
            " C{}/{}{}",
            pane.selected_visible_col + 1,
            pane.visible_count,
            hidden_info
        )
    } else {
        String::new()
    };

    let position = if row_info.is_empty() {
        String::new()
    } else {
        format!("[{}{}]", row_info, col_info)
    };

    let filter_info = if !pane.filter_text.is_empty() {
        format!(" /{}", pane.filter_text)
    } else {
        String::new()
    };

    let focus_indicator = if is_focused { "*" } else { "" };

    format!(
        "{}{} {}{}",
        focus_indicator, pane.name, position, filter_info
    )
}

/// Build tab bar string for multi-tab display.
/// Format: "1:name 2:name [3:active] 4:name | " with numbers matching keyboard shortcuts.
pub fn build_tab_bar(workspace: &Workspace) -> String {
    let tab_count = workspace.tab_count();
    let is_split = workspace.split_active && tab_count > 1;
    if tab_count > 1 {
        let names: Vec<String> = workspace
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| {
                // Truncate long tab names to prevent title overflow
                let name = if t.name.len() > 15 {
                    format!("{}...", &t.name[..12])
                } else {
                    t.name.clone()
                };
                // Mark both active and split tabs in split mode
                if is_split && i == workspace.split_idx && i != workspace.active_idx {
                    format!("<{}:{}>", i + 1, name)
                } else if i == workspace.active_idx {
                    format!("[{}:{}]", i + 1, name)
                } else {
                    format!("{}:{}", i + 1, name)
                }
            })
            .collect();
        format!("{} | ", names.join(" "))
    } else {
        String::new()
    }
}

/// Build context-appropriate controls hint string.
pub fn build_controls_hint(view_mode: ViewMode, is_split: bool, tab_count: usize) -> String {
    let split_controls = if is_split {
        "Tab: switch pane, V: unsplit, "
    } else if tab_count > 1 {
        "V: split, "
    } else {
        ""
    };
    let tab_controls = if tab_count > 1 {
        "1-9: tab, W: close, "
    } else {
        ""
    };

    match view_mode {
        ViewMode::TableList => format!(
            "{}{}Enter: select, /: filter, q: quit",
            split_controls, tab_controls
        ),
        ViewMode::TableData => format!(
            "{}{}+/-: width, H/S: hide/show, </>: move, E: export, 0: reset, Esc: back, q: quit",
            split_controls, tab_controls
        ),
        ViewMode::PipeData => format!(
            "{}{}+/-: width, H/S: hide/show, </>: move, E: export, 0: reset, q: quit",
            split_controls, tab_controls
        ),
    }
}

/// Render input bar for query/search/export modes.
pub fn render_input_bar(frame: &mut Frame, area: Rect, mode: AppMode, input_buffer: &str) {
    let (prefix, style) = match mode {
        AppMode::QueryInput => (":", Style::default().fg(Color::Cyan)),
        AppMode::SearchInput => ("/", Style::default().fg(Color::Yellow)),
        AppMode::ExportFilename => ("Save as: ", Style::default().fg(Color::Green)),
        AppMode::Normal | AppMode::ExportFormat => ("", Style::default()),
    };

    let input_text = format!("{}{}", prefix, input_buffer);
    let input_widget = Paragraph::new(input_text)
        .style(style)
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(input_widget, area);
}

/// Render export format selection prompt.
pub fn render_format_prompt(frame: &mut Frame, area: Rect) {
    let prompt_text = "Export format: [C]SV or [J]SON (Esc to cancel)";
    let prompt_widget = Paragraph::new(prompt_text)
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(prompt_widget, area);
}
