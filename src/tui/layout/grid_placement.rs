//! Grid-line coordinates and builders for explicit child placement.

/// Grid line position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridLine {
    /// Selects a grid line using CSS-style numbering.
    ///
    /// Positive numbers are one-based; negative numbers count backward from the
    /// end of the explicit grid (`-1` is its final line). Zero is not a valid
    /// explicit line; the underlying layout engine treats it as automatic.
    Line(i16),
    /// Auto placement
    #[default]
    Auto,
}

impl GridLine {
    /// Parses `"auto"` or an `i16` line number after trimming whitespace.
    ///
    /// Returns `None` for unknown keywords or out-of-range integers. Parsing
    /// does not validate line semantics: zero and negative numbers are accepted.
    ///
    /// # Examples
    /// ```
    /// use rsille::tui::layout::GridLine;
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
/// Defines start and end grid lines for each axis. All lines default to
/// [`GridLine::Auto`]. End lines are exclusive, not span counts. The bridge
/// currently ignores an explicit end when the corresponding start is automatic.
///
/// Span builders compute `end = start + span as i16`; use positive starts and
/// spans with both the span and `start + span` representable in `i16`.
/// These builders do not store a CSS
/// `span` value, so negative starts need particular care when crossing zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridPlacement {
    /// Column start position
    pub column_start: GridLine,
    /// Exclusive column end line; not a span count.
    pub column_end: GridLine,
    /// Row start position
    pub row_start: GridLine,
    /// Exclusive row end line; not a span count.
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
    /// use rsille::tui::layout::GridPlacement;
    ///
    /// let placement = GridPlacement::new().column(2); // Second column
    /// ```
    ///
    /// Existing end lines are preserved; use a fresh placement for automatic ends.
    pub fn column(mut self, line: i16) -> Self {
        self.column_start = GridLine::Line(line);
        self
    }

    /// Set column span (start line and span amount)
    ///
    /// # Examples
    /// ```
    /// use rsille::tui::layout::GridPlacement;
    ///
    /// let placement = GridPlacement::new().column_span(1, 2); // Starts at column 1, spans 2 columns
    /// ```
    ///
    /// # Panics
    ///
    /// In builds with overflow checks enabled, panics if adding the span (cast
    /// to `i16`) to the start overflows. Use positive starts and spans, with
    /// both the span and the resulting end line representable in `i16`.
    pub fn column_span(mut self, start: i16, span: u16) -> Self {
        self.column_start = GridLine::Line(start);
        self.column_end = GridLine::Line(start + span as i16);
        self
    }

    /// Set row position (1-indexed)
    ///
    /// # Examples
    /// ```
    /// use rsille::tui::layout::GridPlacement;
    ///
    /// let placement = GridPlacement::new().row(2); // Second row
    /// ```
    ///
    /// Existing end lines are preserved; use a fresh placement for automatic ends.
    pub fn row(mut self, line: i16) -> Self {
        self.row_start = GridLine::Line(line);
        self
    }

    /// Set row span (start line and span amount)
    ///
    /// # Examples
    /// ```
    /// use rsille::tui::layout::GridPlacement;
    ///
    /// let placement = GridPlacement::new().row_span(1, 2); // Starts at row 1, spans 2 rows
    /// ```
    ///
    /// # Panics
    ///
    /// In builds with overflow checks enabled, panics if adding the span (cast
    /// to `i16`) to the start overflows. Use positive starts and spans, with
    /// both the span and the resulting end line representable in `i16`.
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
    /// use rsille::tui::layout::GridPlacement;
    ///
    /// let placement = GridPlacement::new().area(2, 3); // Column 2, Row 3
    /// ```
    ///
    /// Existing end lines are preserved; use a fresh placement for automatic ends.
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
    /// use rsille::tui::layout::GridPlacement;
    ///
    /// // Starts at column 1, row 1, spans 2 columns and 2 rows
    /// let placement = GridPlacement::new().area_span(1, 1, 2, 2);
    /// ```
    ///
    /// # Panics
    ///
    /// In builds with overflow checks enabled, panics if adding the span (cast
    /// to `i16`) to the start overflows. Use positive starts and spans, with
    /// both the span and the resulting end line representable in `i16`.
    pub fn area_span(
        mut self,
        column_start: i16,
        row_start: i16,
        column_span: u16,
        row_span: u16,
    ) -> Self {
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
