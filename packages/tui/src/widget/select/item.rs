//! Select item and event types

/// Selection event information
///
/// Contains the selected value and index.
#[derive(Debug, Clone)]
pub struct SelectEvent<T: Clone> {
    /// The selected item value
    pub value: T,
    /// The selected item index
    pub index: usize,
}

/// A single item in the select dropdown
#[derive(Debug, Clone)]
pub struct SelectItem<T: Clone> {
    /// The actual value/data of this item
    pub value: T,
    /// Display label for this item
    pub label: String,
    /// Whether this item is disabled (cannot be selected)
    pub disabled: bool,
}

impl<T: Clone> SelectItem<T> {
    /// Create a new select item with the given value and label
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            disabled: false,
        }
    }

    /// Mark this item as disabled
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}
