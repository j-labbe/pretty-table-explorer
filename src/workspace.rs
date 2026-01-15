//! Workspace module for managing multiple table tabs.
//!
//! Provides Tab and Workspace structs to organize multiple query results
//! as named tabs, each with its own TableData, ColumnConfig, and navigation state.

use ratatui::widgets::TableState;

use crate::column::ColumnConfig;
use crate::parser::TableData;

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
}

impl Tab {
    /// Create a new tab with the given name and data.
    pub fn new(name: String, data: TableData) -> Self {
        let num_cols = data.headers.len();
        Self {
            name,
            data,
            column_config: ColumnConfig::new(num_cols),
            filter_text: String::new(),
            table_state: TableState::default().with_selected(Some(0)),
            scroll_col_offset: 0,
            selected_visible_col: 0,
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
}

impl Workspace {
    /// Create a new empty workspace.
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_idx: 0,
        }
    }

    /// Add a new tab with the given name and data.
    /// Returns the index of the new tab.
    pub fn add_tab(&mut self, name: String, data: TableData) -> usize {
        let tab = Tab::new(name, data);
        self.tabs.push(tab);
        self.tabs.len() - 1
    }

    /// Get a reference to the active tab, if any.
    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_idx)
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
    /// Adjusts active_idx if needed.
    pub fn close_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.tabs.remove(idx);
            // Adjust active_idx if we removed a tab before or at the active position
            if !self.tabs.is_empty() {
                if self.active_idx >= self.tabs.len() {
                    self.active_idx = self.tabs.len() - 1;
                } else if idx < self.active_idx {
                    self.active_idx -= 1;
                }
            } else {
                self.active_idx = 0;
            }
        }
    }

    /// Returns the number of tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Returns the names of all tabs (for rendering tab bar).
    pub fn tab_names(&self) -> Vec<&str> {
        self.tabs.iter().map(|t| t.name.as_str()).collect()
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
        assert!(ws.active_tab().is_none());
    }

    #[test]
    fn test_add_tab() {
        let mut ws = Workspace::new();
        let idx = ws.add_tab("Test".to_string(), sample_data());
        assert_eq!(idx, 0);
        assert_eq!(ws.tab_count(), 1);
        assert!(ws.active_tab().is_some());
        assert_eq!(ws.active_tab().unwrap().name, "Test");
    }

    #[test]
    fn test_switch_tabs() {
        let mut ws = Workspace::new();
        ws.add_tab("Tab1".to_string(), sample_data());
        ws.add_tab("Tab2".to_string(), sample_data());
        ws.add_tab("Tab3".to_string(), sample_data());

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
        ws.add_tab("Tab1".to_string(), sample_data());
        ws.add_tab("Tab2".to_string(), sample_data());

        ws.switch_to(1);
        assert_eq!(ws.active_idx, 1);

        // Clamp to valid range
        ws.switch_to(100);
        assert_eq!(ws.active_idx, 1);
    }

    #[test]
    fn test_close_tab() {
        let mut ws = Workspace::new();
        ws.add_tab("Tab1".to_string(), sample_data());
        ws.add_tab("Tab2".to_string(), sample_data());
        ws.add_tab("Tab3".to_string(), sample_data());

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
    fn test_tab_names() {
        let mut ws = Workspace::new();
        ws.add_tab("Alpha".to_string(), sample_data());
        ws.add_tab("Beta".to_string(), sample_data());

        let names = ws.tab_names();
        assert_eq!(names, vec!["Alpha", "Beta"]);
    }

    #[test]
    fn test_tab_initialization() {
        let data = sample_data();
        let tab = Tab::new("Test".to_string(), data);

        assert_eq!(tab.name, "Test");
        assert_eq!(tab.filter_text, "");
        assert_eq!(tab.scroll_col_offset, 0);
        assert_eq!(tab.selected_visible_col, 0);
        assert_eq!(tab.table_state.selected(), Some(0));
    }
}
