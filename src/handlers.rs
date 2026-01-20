//! Keyboard input handlers for the table explorer.
//!
//! Contains handler functions for different application modes and the KeyAction
//! enum to represent the result of handling a key event.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;

use crate::column::ColumnConfig;
use crate::db;
use crate::export::{self, ExportFormat};
use crate::parser::TableData;
use crate::render::calculate_auto_widths;
use crate::state::AppMode;
use crate::workspace::{Tab, ViewMode};

/// Result of handling a key event.
/// Tells the main loop what action to take after the handler returns.
#[derive(Debug)]
pub enum KeyAction {
    /// No action needed
    None,
    /// Exit the application
    Quit,
    /// Display a status message
    StatusMessage(String),
    /// Create a new tab with the given data
    CreateTab {
        name: String,
        data: TableData,
        view_mode: ViewMode,
    },
    /// Change input mode
    ModeChange(AppMode),
    /// Perform a workspace operation
    Workspace(WorkspaceOp),
}

/// Operations on the workspace that need to be performed in main.rs
/// (because they require mutable workspace access while we hold a tab borrow)
#[derive(Debug)]
pub enum WorkspaceOp {
    /// Toggle split view
    ToggleSplit,
    /// Toggle focus between panes
    ToggleFocus,
    /// Switch to next tab
    NextTab,
    /// Switch to previous tab
    PrevTab,
    /// Switch to specific tab by index
    SwitchTo(usize),
    /// Close tab at the focused index
    CloseTab,
}

/// Handle key events in normal mode.
///
/// Takes references to the focused tab and other state needed for handling.
/// Returns a KeyAction indicating what the main loop should do.
pub fn handle_normal_mode(
    key: &KeyEvent,
    tab: &mut Tab,
    db_client: &mut Option<postgres::Client>,
    table_list_cache: &Option<TableData>,
    current_table_name: &mut Option<String>,
    displayed_row_count: usize,
    has_split: bool,
    tab_count: usize,
) -> KeyAction {
    match key.code {
        // Quit on 'q' or Ctrl+C
        KeyCode::Char('q') => KeyAction::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyAction::Quit,

        // Enter: Select table in TableList mode
        KeyCode::Enter => {
            if tab.view_mode == ViewMode::TableList {
                if let Some(ref mut client) = db_client {
                    if let Some(selected) = tab.table_state.selected() {
                        // Recalculate display_rows for event handling
                        let filter_lower = tab.filter_text.to_lowercase();
                        let display_rows: Vec<&Vec<String>> = if tab.filter_text.is_empty() {
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
                        if let Some(row) = display_rows.get(selected) {
                            if let Some(tbl_name) = row.first() {
                                let query =
                                    format!("SELECT * FROM \"{}\" LIMIT 1000", tbl_name);
                                match db::execute_query(client, &query) {
                                    Ok(data) => {
                                        if data.headers.is_empty() && data.rows.is_empty() {
                                            return KeyAction::StatusMessage(
                                                "Table is empty".to_string(),
                                            );
                                        } else {
                                            *current_table_name = Some(tbl_name.clone());
                                            return KeyAction::CreateTab {
                                                name: tbl_name.clone(),
                                                data,
                                                view_mode: ViewMode::TableData,
                                            };
                                        }
                                    }
                                    Err(e) => {
                                        return KeyAction::StatusMessage(format!("Error: {}", e));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            KeyAction::None
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
                    *current_table_name = None;
                    tab.view_mode = ViewMode::TableList;
                }
            }
            KeyAction::None
        }

        // Enter query input mode (only in DB modes, not pipe)
        KeyCode::Char(':') => {
            if db_client.is_some() {
                KeyAction::ModeChange(AppMode::QueryInput)
            } else {
                KeyAction::StatusMessage("Query mode requires --connect".to_string())
            }
        }

        // Enter search input mode
        KeyCode::Char('/') => KeyAction::ModeChange(AppMode::SearchInput),

        // Vertical navigation (bounded by displayed row count)
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(selected) = tab.table_state.selected() {
                if selected + 1 < displayed_row_count {
                    tab.table_state.select(Some(selected + 1));
                }
            }
            KeyAction::None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Some(selected) = tab.table_state.selected() {
                if selected > 0 {
                    tab.table_state.select(Some(selected - 1));
                }
            }
            KeyAction::None
        }

        // Jump to first/last (bounded by displayed row count)
        KeyCode::Char('g') | KeyCode::Home => {
            tab.table_state.select(Some(0));
            KeyAction::None
        }
        KeyCode::Char('G') | KeyCode::End => {
            if displayed_row_count > 0 {
                tab.table_state.select(Some(displayed_row_count - 1));
            }
            KeyAction::None
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
            KeyAction::None
        }
        KeyCode::Char('l') | KeyCode::Right => {
            let visible_cols = tab.column_config.visible_indices();
            if tab.selected_visible_col + 1 < visible_cols.len() {
                tab.selected_visible_col += 1;
                // Scroll right will be handled in render loop when needed
            }
            KeyAction::None
        }

        // Page navigation (half-page like vim, bounded by displayed count)
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(selected) = tab.table_state.selected() {
                let new_pos = selected.saturating_sub(10);
                tab.table_state.select(Some(new_pos));
            }
            KeyAction::None
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(selected) = tab.table_state.selected() {
                let new_pos = (selected + 10).min(displayed_row_count.saturating_sub(1));
                tab.table_state.select(Some(new_pos));
            }
            KeyAction::None
        }
        // Also support Page Up/Page Down
        KeyCode::PageUp => {
            if let Some(selected) = tab.table_state.selected() {
                let new_pos = selected.saturating_sub(10);
                tab.table_state.select(Some(new_pos));
            }
            KeyAction::None
        }
        KeyCode::PageDown => {
            if let Some(selected) = tab.table_state.selected() {
                let new_pos = (selected + 10).min(displayed_row_count.saturating_sub(1));
                tab.table_state.select(Some(new_pos));
            }
            KeyAction::None
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
            KeyAction::None
        }
        KeyCode::Char('-') | KeyCode::Char('_') => {
            let visible = tab.column_config.visible_indices();
            if tab.selected_visible_col < visible.len() {
                let data_idx = visible[tab.selected_visible_col];
                let auto_widths = calculate_auto_widths(&tab.data);
                let auto_width = auto_widths.get(data_idx).copied().unwrap_or(10);
                tab.column_config.adjust_width(data_idx, -2, auto_width);
            }
            KeyAction::None
        }
        // Reset column widths to auto (also shows hidden columns and scroll)
        KeyCode::Char('0') => {
            tab.column_config.reset();
            tab.scroll_col_offset = 0;
            tab.selected_visible_col = 0;
            KeyAction::None
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
                    if tab.selected_visible_col >= new_visible.len() && tab.selected_visible_col > 0
                    {
                        tab.selected_visible_col -= 1;
                    }
                }
            }
            KeyAction::None
        }

        // Show all hidden columns (S key)
        KeyCode::Char('S') => {
            tab.column_config.show_all();
            KeyAction::None
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
            KeyAction::None
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
            KeyAction::None
        }

        // Export data (E key)
        KeyCode::Char('E') => {
            // Export available in TableData and PipeData modes
            if tab.view_mode == ViewMode::TableData || tab.view_mode == ViewMode::PipeData {
                KeyAction::ModeChange(AppMode::ExportFormat)
            } else {
                KeyAction::None
            }
        }

        // Tab navigation: toggle focus in split mode, cycle tabs otherwise
        KeyCode::Tab => {
            if has_split {
                // In split view: Tab toggles focus between panes
                KeyAction::Workspace(WorkspaceOp::ToggleFocus)
            } else if tab_count > 1 {
                KeyAction::Workspace(WorkspaceOp::NextTab)
            } else {
                KeyAction::None
            }
        }
        KeyCode::BackTab => {
            // Shift+Tab: same as Tab (toggle focus in split, prev tab otherwise)
            if has_split {
                KeyAction::Workspace(WorkspaceOp::ToggleFocus)
            } else if tab_count > 1 {
                KeyAction::Workspace(WorkspaceOp::PrevTab)
            } else {
                KeyAction::None
            }
        }

        // Direct tab selection with number keys 1-9
        KeyCode::Char(c @ '1'..='9') => {
            let idx = (c as usize) - ('1' as usize);
            if idx < tab_count {
                KeyAction::Workspace(WorkspaceOp::SwitchTo(idx))
            } else {
                KeyAction::None
            }
        }

        // Close focused tab with W (uppercase)
        KeyCode::Char('W') => {
            if tab_count > 1 {
                KeyAction::Workspace(WorkspaceOp::CloseTab)
            } else {
                KeyAction::None
            }
        }

        // Toggle split view with V (uppercase)
        KeyCode::Char('V') => KeyAction::Workspace(WorkspaceOp::ToggleSplit),

        // Switch focus between panes with Ctrl+W or F6
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            KeyAction::Workspace(WorkspaceOp::ToggleFocus)
        }
        KeyCode::F(6) => KeyAction::Workspace(WorkspaceOp::ToggleFocus),

        _ => KeyAction::None,
    }
}

/// Handle key events in query input mode.
///
/// Returns (KeyAction, bool) where bool indicates whether to return to Normal mode.
pub fn handle_query_input(
    key: &KeyEvent,
    input_buffer: &mut String,
    db_client: &mut Option<postgres::Client>,
) -> (KeyAction, bool) {
    match key.code {
        // Cancel and return to normal mode
        KeyCode::Esc => {
            input_buffer.clear();
            (KeyAction::None, true)
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
                                input_buffer.clear();
                                return (
                                    KeyAction::StatusMessage("Query returned no results".to_string()),
                                    true,
                                );
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

                                input_buffer.clear();
                                return (
                                    KeyAction::CreateTab {
                                        name: tab_name,
                                        data,
                                        view_mode: ViewMode::TableData,
                                    },
                                    true,
                                );
                            }
                        }
                        Err(e) => {
                            input_buffer.clear();
                            return (KeyAction::StatusMessage(format!("Error: {}", e)), true);
                        }
                    }
                }
            } else {
                // Not in database mode
                input_buffer.clear();
                return (
                    KeyAction::StatusMessage("Query mode requires --connect".to_string()),
                    true,
                );
            }
            input_buffer.clear();
            (KeyAction::None, true)
        }

        // Text input
        KeyCode::Char(c) => {
            input_buffer.push(c);
            (KeyAction::None, false)
        }

        // Backspace
        KeyCode::Backspace => {
            input_buffer.pop();
            (KeyAction::None, false)
        }

        _ => (KeyAction::None, false),
    }
}

/// Handle key events in search input mode.
///
/// Returns bool indicating whether to return to Normal mode.
pub fn handle_search_input(key: &KeyEvent, input_buffer: &mut String, tab: &mut Tab) -> bool {
    match key.code {
        // Cancel and return to normal mode
        KeyCode::Esc => {
            input_buffer.clear();
            true
        }

        // Apply filter and return to normal mode
        KeyCode::Enter => {
            // Set or clear filter based on input
            tab.filter_text = input_buffer.trim().to_string();
            // Reset selection to 0 when filter changes
            tab.table_state = TableState::default().with_selected(Some(0));
            input_buffer.clear();
            true
        }

        // Text input
        KeyCode::Char(c) => {
            input_buffer.push(c);
            false
        }

        // Backspace
        KeyCode::Backspace => {
            input_buffer.pop();
            false
        }

        _ => false,
    }
}

/// Handle key events in export format selection mode.
///
/// Returns Option<AppMode> for the next mode:
/// - None = stay in current mode
/// - Some(Normal) = cancel
/// - Some(ExportFilename) = proceed to filename input
pub fn handle_export_format(
    key: &KeyEvent,
    export_format: &mut Option<ExportFormat>,
    input_buffer: &mut String,
) -> Option<AppMode> {
    match key.code {
        // Cancel and return to normal mode
        KeyCode::Esc => Some(AppMode::Normal),

        // Select CSV format
        KeyCode::Char('c') | KeyCode::Char('C') => {
            *export_format = Some(ExportFormat::Csv);
            *input_buffer = "export.csv".to_string();
            Some(AppMode::ExportFilename)
        }

        // Select JSON format
        KeyCode::Char('j') | KeyCode::Char('J') => {
            *export_format = Some(ExportFormat::Json);
            *input_buffer = "export.json".to_string();
            Some(AppMode::ExportFilename)
        }

        _ => None,
    }
}

/// Handle key events in export filename input mode.
///
/// Returns (Option<String>, bool) where:
/// - Option<String> is a status message (success or error)
/// - bool indicates whether we're done (return to Normal mode)
pub fn handle_export_filename(
    key: &KeyEvent,
    input_buffer: &mut String,
    export_format: Option<ExportFormat>,
    tab: &Tab,
) -> (Option<String>, bool) {
    match key.code {
        // Cancel and return to normal mode
        KeyCode::Esc => {
            input_buffer.clear();
            (None, true)
        }

        // Perform export
        KeyCode::Enter => {
            let filename = input_buffer.trim().to_string();
            if !filename.is_empty() {
                if let Some(fmt) = export_format {
                    let visible_cols = tab.column_config.visible_indices();
                    match export::export_table(&tab.data, &visible_cols, fmt) {
                        Ok(content) => match export::save_to_file(&content, &filename) {
                            Ok(()) => {
                                input_buffer.clear();
                                return (Some(format!("Exported to {}", filename)), true);
                            }
                            Err(e) => {
                                input_buffer.clear();
                                return (Some(e), true);
                            }
                        },
                        Err(e) => {
                            input_buffer.clear();
                            return (Some(e), true);
                        }
                    }
                }
            }
            input_buffer.clear();
            (None, true)
        }

        // Text input
        KeyCode::Char(c) => {
            input_buffer.push(c);
            (None, false)
        }

        // Backspace
        KeyCode::Backspace => {
            input_buffer.pop();
            (None, false)
        }

        _ => (None, false),
    }
}
