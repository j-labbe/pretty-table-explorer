/// Per-column display configuration
#[derive(Debug, Clone)]
pub struct ColumnState {
    /// Width override (None = auto-size)
    pub width_override: Option<u16>,
    /// Whether column is visible
    pub visible: bool,
}

impl Default for ColumnState {
    fn default() -> Self {
        Self {
            width_override: None,
            visible: true,
        }
    }
}

/// Manages column display configuration for a table
#[derive(Debug, Clone)]
pub struct ColumnConfig {
    columns: Vec<ColumnState>,
}

impl ColumnConfig {
    /// Create config for n columns (all visible, auto-width)
    pub fn new(num_columns: usize) -> Self {
        Self {
            columns: vec![ColumnState::default(); num_columns],
        }
    }

    /// Reset to auto-size for all columns
    pub fn reset(&mut self) {
        for col in &mut self.columns {
            col.width_override = None;
        }
    }

    /// Adjust width override for column (min 3, max 100)
    /// When adjusting, if override is None, start from a default of 10.
    pub fn adjust_width(&mut self, col: usize, delta: i16) {
        if let Some(column) = self.columns.get_mut(col) {
            let current = column.width_override.unwrap_or(10) as i16;
            let new_width = (current + delta).clamp(3, 100) as u16;
            column.width_override = Some(new_width);
        }
    }

    /// Get width override for column (None = auto)
    pub fn get_width(&self, col: usize) -> Option<u16> {
        self.columns.get(col).and_then(|c| c.width_override)
    }

    /// Check if column is visible
    pub fn is_visible(&self, col: usize) -> bool {
        self.columns.get(col).map(|c| c.visible).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_config() {
        let config = ColumnConfig::new(3);
        assert_eq!(config.columns.len(), 3);
        for i in 0..3 {
            assert!(config.is_visible(i));
            assert_eq!(config.get_width(i), None);
        }
    }

    #[test]
    fn test_adjust_width_from_none() {
        let mut config = ColumnConfig::new(2);
        config.adjust_width(0, 5);
        // Starting from 10, +5 = 15
        assert_eq!(config.get_width(0), Some(15));
        assert_eq!(config.get_width(1), None);
    }

    #[test]
    fn test_adjust_width_min_bound() {
        let mut config = ColumnConfig::new(1);
        config.adjust_width(0, -20);
        // Starting from 10, -20 should clamp to 3
        assert_eq!(config.get_width(0), Some(3));
    }

    #[test]
    fn test_adjust_width_max_bound() {
        let mut config = ColumnConfig::new(1);
        config.adjust_width(0, 200);
        // Starting from 10, +200 should clamp to 100
        assert_eq!(config.get_width(0), Some(100));
    }

    #[test]
    fn test_reset() {
        let mut config = ColumnConfig::new(2);
        config.adjust_width(0, 5);
        config.adjust_width(1, 10);
        config.reset();
        assert_eq!(config.get_width(0), None);
        assert_eq!(config.get_width(1), None);
    }

    #[test]
    fn test_out_of_bounds() {
        let config = ColumnConfig::new(2);
        assert_eq!(config.get_width(5), None);
        assert!(!config.is_visible(5));
    }
}
