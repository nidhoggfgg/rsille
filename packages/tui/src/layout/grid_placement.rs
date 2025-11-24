//! Grid item placement types

/// Grid line position
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridLine {
    /// Start at a specific line number (1-indexed)
    Line(i16),
    /// Auto placement
    Auto,
}

impl Default for GridLine {
    fn default() -> Self {
        GridLine::Auto
    }
}

impl GridLine {
    /// Parse a grid line from a string
    ///
    /// # Examples
    /// ```
    /// use tui::layout::GridLine;
    ///
    /// assert_eq!(GridLine::parse("1"), Some(GridLine::Line(1)));
    /// assert_eq!(GridLine::parse("auto"), Some(GridLine::Auto));
    /// ```
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s == "auto" {
            Some(GridLine::Auto)
        } else if let Ok(line) = s.parse::<i16>() {
            Some(GridLine::Line(line))
        } else {
            None
        }
    }
}

/// Grid item placement information
///
/// Defines where a grid item should be placed in the grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridPlacement {
    /// Column start position
    pub column_start: GridLine,
    /// Column end position (or span)
    pub column_end: GridLine,
    /// Row start position
    pub row_start: GridLine,
    /// Row end position (or span)
    pub row_end: GridLine,
}

impl Default for GridPlacement {
    fn default() -> Self {
        Self {
            column_start: GridLine::Auto,
            column_end: GridLine::Auto,
            row_start: GridLine::Auto,
            row_end: GridLine::Auto,
        }
    }
}

impl GridPlacement {
    /// Create a new grid placement with all auto
    pub fn new() -> Self {
        Self::default()
    }

    /// Set column position (1-indexed)
    ///
    /// # Examples
    /// ```
    /// use tui::layout::GridPlacement;
    ///
    /// let placement = GridPlacement::new().column(2); // Second column
    /// ```
    pub fn column(mut self, line: i16) -> Self {
        self.column_start = GridLine::Line(line);
        self
    }

    /// Set column span (start line and span amount)
    ///
    /// # Examples
    /// ```
    /// use tui::layout::GridPlacement;
    ///
    /// let placement = GridPlacement::new().column_span(1, 2); // Starts at column 1, spans 2 columns
    /// ```
    pub fn column_span(mut self, start: i16, span: u16) -> Self {
        self.column_start = GridLine::Line(start);
        self.column_end = GridLine::Line(start + span as i16);
        self
    }

    /// Set row position (1-indexed)
    ///
    /// # Examples
    /// ```
    /// use tui::layout::GridPlacement;
    ///
    /// let placement = GridPlacement::new().row(2); // Second row
    /// ```
    pub fn row(mut self, line: i16) -> Self {
        self.row_start = GridLine::Line(line);
        self
    }

    /// Set row span (start line and span amount)
    ///
    /// # Examples
    /// ```
    /// use tui::layout::GridPlacement;
    ///
    /// let placement = GridPlacement::new().row_span(1, 2); // Starts at row 1, spans 2 rows
    /// ```
    pub fn row_span(mut self, start: i16, span: u16) -> Self {
        self.row_start = GridLine::Line(start);
        self.row_end = GridLine::Line(start + span as i16);
        self
    }

    /// Set grid area (shorthand for column and row)
    ///
    /// # Arguments
    /// * `column` - Column line (1-indexed)
    /// * `row` - Row line (1-indexed)
    ///
    /// # Examples
    /// ```
    /// use tui::layout::GridPlacement;
    ///
    /// let placement = GridPlacement::new().area(2, 3); // Column 2, Row 3
    /// ```
    pub fn area(mut self, column: i16, row: i16) -> Self {
        self.column_start = GridLine::Line(column);
        self.row_start = GridLine::Line(row);
        self
    }

    /// Set grid area with span
    ///
    /// # Arguments
    /// * `column_start` - Start column (1-indexed)
    /// * `row_start` - Start row (1-indexed)
    /// * `column_span` - Number of columns to span
    /// * `row_span` - Number of rows to span
    ///
    /// # Examples
    /// ```
    /// use tui::layout::GridPlacement;
    ///
    /// // Starts at column 1, row 1, spans 2 columns and 2 rows
    /// let placement = GridPlacement::new().area_span(1, 1, 2, 2);
    /// ```
    pub fn area_span(mut self, column_start: i16, row_start: i16, column_span: u16, row_span: u16) -> Self {
        self.column_start = GridLine::Line(column_start);
        self.column_end = GridLine::Line(column_start + column_span as i16);
        self.row_start = GridLine::Line(row_start);
        self.row_end = GridLine::Line(row_start + row_span as i16);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_line_parse() {
        assert_eq!(GridLine::parse("1"), Some(GridLine::Line(1)));
        assert_eq!(GridLine::parse("5"), Some(GridLine::Line(5)));
        assert_eq!(GridLine::parse("auto"), Some(GridLine::Auto));
        assert_eq!(GridLine::parse("-1"), Some(GridLine::Line(-1)));
        assert_eq!(GridLine::parse("invalid"), None);
    }

    #[test]
    fn test_grid_placement_column() {
        let placement = GridPlacement::new().column(2);
        assert_eq!(placement.column_start, GridLine::Line(2));
    }

    #[test]
    fn test_grid_placement_span() {
        let placement = GridPlacement::new().column_span(1, 2);
        assert_eq!(placement.column_start, GridLine::Line(1));
        assert_eq!(placement.column_end, GridLine::Line(3));
    }

    #[test]
    fn test_grid_placement_area() {
        let placement = GridPlacement::new().area(2, 3);
        assert_eq!(placement.column_start, GridLine::Line(2));
        assert_eq!(placement.row_start, GridLine::Line(3));
    }

    #[test]
    fn test_grid_placement_area_span() {
        let placement = GridPlacement::new().area_span(1, 1, 2, 3);
        assert_eq!(placement.column_start, GridLine::Line(1));
        assert_eq!(placement.column_end, GridLine::Line(3));
        assert_eq!(placement.row_start, GridLine::Line(1));
        assert_eq!(placement.row_end, GridLine::Line(4));
    }
}
