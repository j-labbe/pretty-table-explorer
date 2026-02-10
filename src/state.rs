//! Application state types for the table explorer.
//!
//! Contains core types for managing application mode, deferred actions,
//! and pre-computed render state for table panes.

use ratatui::prelude::Constraint;

use crate::parser::TableData;
use crate::workspace::ViewMode;

/// Application mode for handling different input states.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AppMode {
    Normal,         // Regular table navigation
    QueryInput,     // ':' pressed, entering SQL query
    SearchInput,    // '/' pressed, entering search filter
    ExportFormat,   // 'E' pressed, selecting export format (CSV/JSON)
    ExportFilename, // Format selected, entering filename
}

/// Pending action to be executed after dropping mutable tab reference.
/// Used to avoid borrow conflicts when creating new tabs.
pub enum PendingAction {
    None,
    CreateTab {
        name: String,
        data: TableData,
        view_mode: ViewMode,
    },
}

/// Data needed to render a single table pane.
pub struct PaneRenderData {
    /// Tab name
    pub name: String,
    /// Filtered display rows (copies for render closure)
    pub display_rows: Vec<Vec<String>>,
    /// Headers
    pub headers: Vec<String>,
    /// Total rows (before filter)
    pub total_rows: usize,
    /// Displayed row count (after filter)
    pub displayed_row_count: usize,
    /// Visible column indices
    pub visible_cols: Vec<usize>,
    /// Column widths
    pub widths: Vec<Constraint>,
    /// Filter text
    pub filter_text: String,
    /// Scroll column offset
    pub scroll_col_offset: usize,
    /// Selected visible column
    pub selected_visible_col: usize,
    /// Visible column count
    pub visible_count: usize,
    /// Hidden column count
    pub hidden_count: usize,
    /// Selected row
    pub selected_row: Option<usize>,
    /// Viewport row offset (first row index relative to full filtered dataset)
    pub viewport_row_offset: usize,
}
