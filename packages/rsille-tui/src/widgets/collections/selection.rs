//! Shared selection state used by data-oriented widgets.

/// Selection behavior for data widgets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SelectionMode {
    /// Keeps at most one committed selection during widget interaction (default).
    #[default]
    Single,
    /// Allows independent selection of multiple IDs.
    Multiple,
}

impl SelectionMode {
    /// Returns whether this mode permits multiple selected IDs.
    pub fn is_multiple(self) -> bool {
        matches!(self, Self::Multiple)
    }
}

/// Shared selection model for widgets that distinguish keyboard cursor from committed selection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectionState {
    /// ID at the navigation cursor, independent of committed selection.
    pub cursor: Option<String>,
    /// Committed selected IDs in insertion order.
    pub selected: Vec<String>,
}

impl SelectionState {
    /// Returns the navigation cursor ID, if any.
    pub fn cursor(&self) -> Option<&str> {
        self.cursor.as_deref()
    }

    /// Returns selected IDs in their stored order.
    pub fn selected_ids(&self) -> &[String] {
        &self.selected
    }

    /// Returns the first selected ID, or `None` when the selection is empty.
    pub fn primary_selected(&self) -> Option<&str> {
        self.selected.first().map(String::as_str)
    }

    /// Returns whether `id` is in the committed selection.
    pub fn is_selected(&self, id: &str) -> bool {
        self.selected.iter().any(|selected| selected == id)
    }

    /// Moves or clears the navigation cursor without changing selected IDs.
    pub fn set_cursor(&mut self, id: Option<String>) {
        self.cursor = id;
    }

    /// Replaces the selection with exactly one ID, leaving the cursor unchanged.
    pub fn replace_selection(&mut self, id: impl Into<String>) {
        self.selected.clear();
        self.selected.push(id.into());
    }

    /// Replaces selected IDs, removing duplicates while preserving first-occurrence order.
    ///
    /// The navigation cursor is unchanged.
    pub fn set_selected_ids<I>(&mut self, ids: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.selected.clear();

        for id in ids {
            if !self.selected.iter().any(|selected| selected == &id) {
                self.selected.push(id);
            }
        }
    }

    /// Clears committed selection without changing the navigation cursor.
    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    /// Toggles membership of `id`, returning `true` if it is selected afterward.
    ///
    /// New IDs are appended; the navigation cursor is unchanged.
    pub fn toggle(&mut self, id: &str) -> bool {
        if let Some(index) = self.selected.iter().position(|selected| selected == id) {
            self.selected.remove(index);
            false
        } else {
            self.selected.push(id.to_owned());
            true
        }
    }
}
