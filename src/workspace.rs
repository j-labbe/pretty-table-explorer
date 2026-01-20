//! Workspace module for managing multiple table tabs.
//!
//! Provides Tab and Workspace structs to organize multiple query results
//! as named tabs, each with its own TableData, ColumnConfig, and navigation state.

use ratatui::widgets::TableState;

use crate::column::ColumnConfig;
use crate::parser::TableData;

/// View mode for database browser.
/// Determines what controls are shown and how navigation behaves.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ViewMode {
    TableList, // Viewing list of tables (can select with Enter)
    TableData, // Viewing table contents (Esc to go back)
    PipeData,  // Viewing piped data (no back navigation)
}

/// A single tab containing table data and its display state.
#[derive(Debug, Clone)]
pub struct Tab {
    /// Tab label (e.g., "users", "Query 1")
    pub name: String,
    /// The table content
    pub data: TableData,
    /// Per-tab column configuration (width, visibility, order)
    pub column_config: ColumnConfig,
    /// Per-tab filter text
    pub filter_text: String,
    /// Row selection state
    pub table_state: TableState,
    /// Horizontal scroll offset (index into visible columns)
    pub scroll_col_offset: usize,
    /// Selected column index within visible_cols
    pub selected_visible_col: usize,
    /// View mode for this tab (determines available controls)
    pub view_mode: ViewMode,
}

impl Tab {
    /// Create a new tab with the given name, data, and view mode.
    pub fn new(name: String, data: TableData, view_mode: ViewMode) -> Self {
        let num_cols = data.headers.len();
        Self {
            name,
            data,
            column_config: ColumnConfig::new(num_cols),
            filter_text: String::new(),
            table_state: TableState::default().with_selected(Some(0)),
            scroll_col_offset: 0,
            selected_visible_col: 0,
            view_mode,
        }
    }
}

/// Workspace managing multiple tabs.
#[derive(Debug)]
pub struct Workspace {
    /// Collection of tabs
    pub tabs: Vec<Tab>,
    /// Index of the currently active tab
    pub active_idx: usize,
    /// Is split view enabled
    pub split_active: bool,
    /// Tab index shown in right pane
    pub split_idx: usize,
    /// Which pane has focus (true = left/main)
    pub focus_left: bool,
}

impl Workspace {
    /// Create a new empty workspace.
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_idx: 0,
            split_active: false,
            split_idx: 0,
            focus_left: true,
        }
    }

    /// Add a new tab with the given name, data, and view mode.
    /// Returns the index of the new tab.
    pub fn add_tab(&mut self, name: String, data: TableData, view_mode: ViewMode) -> usize {
        let tab = Tab::new(name, data, view_mode);
        self.tabs.push(tab);
        self.tabs.len() - 1
    }

    /// Get a mutable reference to the active tab, if any.
    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_idx)
    }

    /// Switch to the tab at the given index.
    /// Index is clamped to valid range.
    pub fn switch_to(&mut self, idx: usize) {
        if !self.tabs.is_empty() {
            self.active_idx = idx.min(self.tabs.len() - 1);
        }
    }

    /// Switch to the next tab (wraps around).
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_idx = (self.active_idx + 1) % self.tabs.len();
        }
    }

    /// Switch to the previous tab (wraps around).
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            if self.active_idx == 0 {
                self.active_idx = self.tabs.len() - 1;
            } else {
                self.active_idx -= 1;
            }
        }
    }

    /// Close the tab at the given index.
    /// Adjusts active_idx and split_idx if needed.
    pub fn close_tab(&mut self, idx: usize) {
        if self.tabs.len() <= 1 {
            return;
        }
        if idx < self.tabs.len() {
            // Track if we're closing the right pane's tab while focused on it
            let closing_focused_right =
                self.split_active && !self.focus_left && idx == self.split_idx;

            self.tabs.remove(idx);

            // Adjust active_idx
            if self.active_idx >= self.tabs.len() {
                self.active_idx = self.tabs.len() - 1;
            } else if self.active_idx > idx {
                self.active_idx -= 1;
            }

            // Adjust split_idx
            if self.split_idx >= self.tabs.len() {
                self.split_idx = self.tabs.len() - 1;
            } else if self.split_idx > idx {
                self.split_idx -= 1;
            }

            // Disable split if only one tab left
            if self.tabs.len() == 1 {
                self.split_active = false;
                self.focus_left = true;
            } else if self.split_active {
                // Ensure split_idx != active_idx when split is active
                if self.split_idx == self.active_idx {
                    // Pick a different tab for the right pane
                    self.split_idx = (self.active_idx + 1) % self.tabs.len();
                }
                // If we closed the focused right pane's tab, move focus to left
                if closing_focused_right {
                    self.focus_left = true;
                }
            }
        }
    }

    /// Returns the number of tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Toggle split view on/off.
    /// Requires at least 2 tabs to enable.
    pub fn toggle_split(&mut self) {
        if self.tabs.len() > 1 {
            self.split_active = !self.split_active;
            if self.split_active {
                // Default right pane to next tab after active
                self.split_idx = (self.active_idx + 1) % self.tabs.len();
            }
        }
    }

    /// Toggle focus between left and right panes.
    pub fn toggle_focus(&mut self) {
        if self.split_active {
            self.focus_left = !self.focus_left;
        }
    }

    /// Get a mutable reference to the focused tab.
    pub fn focused_tab_mut(&mut self) -> Option<&mut Tab> {
        if self.split_active && !self.focus_left {
            self.tabs.get_mut(self.split_idx)
        } else {
            self.active_tab_mut()
        }
    }

    /// Get the index of the focused tab.
    pub fn focused_idx(&self) -> usize {
        if self.split_active && !self.focus_left {
            self.split_idx
        } else {
            self.active_idx
        }
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> TableData {
        TableData {
            headers: vec!["id".to_string(), "name".to_string()],
            rows: vec![
                vec!["1".to_string(), "Alice".to_string()],
                vec!["2".to_string(), "Bob".to_string()],
            ],
        }
    }

    #[test]
    fn test_workspace_new() {
        let ws = Workspace::new();
        assert_eq!(ws.tab_count(), 0);
        assert!(ws.tabs.is_empty());
    }

    #[test]
    fn test_add_tab() {
        let mut ws = Workspace::new();
        let idx = ws.add_tab("Test".to_string(), sample_data(), ViewMode::TableData);
        assert_eq!(idx, 0);
        assert_eq!(ws.tab_count(), 1);
        assert!(ws.tabs.get(ws.active_idx).is_some());
        assert_eq!(ws.tabs[ws.active_idx].name, "Test");
    }

    #[test]
    fn test_switch_tabs() {
        let mut ws = Workspace::new();
        ws.add_tab("Tab1".to_string(), sample_data(), ViewMode::TableData);
        ws.add_tab("Tab2".to_string(), sample_data(), ViewMode::TableData);
        ws.add_tab("Tab3".to_string(), sample_data(), ViewMode::TableData);

        assert_eq!(ws.active_idx, 0);

        ws.next_tab();
        assert_eq!(ws.active_idx, 1);

        ws.next_tab();
        assert_eq!(ws.active_idx, 2);

        // Wrap around
        ws.next_tab();
        assert_eq!(ws.active_idx, 0);

        ws.prev_tab();
        assert_eq!(ws.active_idx, 2);
    }

    #[test]
    fn test_switch_to() {
        let mut ws = Workspace::new();
        ws.add_tab("Tab1".to_string(), sample_data(), ViewMode::TableData);
        ws.add_tab("Tab2".to_string(), sample_data(), ViewMode::TableData);

        ws.switch_to(1);
        assert_eq!(ws.active_idx, 1);

        // Clamp to valid range
        ws.switch_to(100);
        assert_eq!(ws.active_idx, 1);
    }

    #[test]
    fn test_close_tab() {
        let mut ws = Workspace::new();
        ws.add_tab("Tab1".to_string(), sample_data(), ViewMode::TableData);
        ws.add_tab("Tab2".to_string(), sample_data(), ViewMode::TableData);
        ws.add_tab("Tab3".to_string(), sample_data(), ViewMode::TableData);

        ws.switch_to(2);
        ws.close_tab(2);
        assert_eq!(ws.tab_count(), 2);
        assert_eq!(ws.active_idx, 1);

        // Close tab before active
        ws.close_tab(0);
        assert_eq!(ws.tab_count(), 1);
        assert_eq!(ws.active_idx, 0);
    }

    #[test]
    fn test_tab_initialization() {
        let data = sample_data();
        let tab = Tab::new("Test".to_string(), data, ViewMode::TableData);

        assert_eq!(tab.name, "Test");
        assert_eq!(tab.filter_text, "");
        assert_eq!(tab.scroll_col_offset, 0);
        assert_eq!(tab.selected_visible_col, 0);
        assert_eq!(tab.table_state.selected(), Some(0));
        assert_eq!(tab.view_mode, ViewMode::TableData);
    }
}
