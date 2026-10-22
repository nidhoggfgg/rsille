//! Shared selection state used by data-oriented widgets.

/// Selection behavior for data widgets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SelectionMode {
    #[default]
    Single,
    Multiple,
}

impl SelectionMode {
    pub fn is_multiple(self) -> bool {
        matches!(self, Self::Multiple)
    }
}

/// Shared selection model for widgets that distinguish keyboard cursor from committed selection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectionState {
    pub cursor: Option<String>,
    pub selected: Vec<String>,
}

impl SelectionState {
    pub fn cursor(&self) -> Option<&str> {
        self.cursor.as_deref()
    }

    pub fn selected_ids(&self) -> &[String] {
        &self.selected
    }

    pub fn primary_selected(&self) -> Option<&str> {
        self.selected.first().map(String::as_str)
    }

    pub fn is_selected(&self, id: &str) -> bool {
        self.selected.iter().any(|selected| selected == id)
    }

    pub fn set_cursor(&mut self, id: Option<String>) {
        self.cursor = id;
    }

    pub fn replace_selection(&mut self, id: impl Into<String>) {
        self.selected.clear();
        self.selected.push(id.into());
    }

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

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

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
