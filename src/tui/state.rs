//! Named application state, cached derived values, and form drafts.
//!
//! [`Store`] holds named typed values independently of widget lifetimes. Use
//! [`crate::tui::widget::WidgetStore`] for state attached to widget identity.
//! [`Derived`] caches a computation by an input hash; [`FormState`] tracks a
//! draft against a committed baseline and records validation errors supplied
//! by application code.
//!
//! # Examples
//!
//! ```
//! use rsille::tui::state::{FormState, Store, StoreKey};
//!
//! let mut store = Store::new();
//! let count = StoreKey::<u32>::new("count");
//! *store.get_or_default(&count) += 1;
//! assert_eq!(store.get(&count), Some(&1));
//!
//! let mut form = FormState::<String>::new("draft".into());
//! form.set_value("revised".into());
//! assert!(form.is_dirty());
//! form.set_error("title", "Please review".into());
//! assert!(!form.is_valid());
//! form.reset();
//! assert_eq!(form.value(), "draft");
//! assert!(form.is_valid());
//! ```

use std::any::Any;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use crate::tui::effect::{RequestState, TaskStatus};

/// Typed key for values stored in [`Store`].
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StoreKey<T> {
    name: String,
    marker: PhantomData<fn() -> T>,
}

impl<T> StoreKey<T> {
    /// Creates a typed key for a named slot.
    ///
    /// Slot identity is the name alone; using the same name with different types
    /// does not create separate slots.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            marker: PhantomData,
        }
    }

    /// Returns the slot name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<T> fmt::Debug for StoreKey<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("StoreKey").field(&self.name).finish()
    }
}

impl<T> From<&str> for StoreKey<T> {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl<T> From<String> for StoreKey<T> {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Lightweight typed store for app-level helper state.
///
/// Use [`crate::tui::widget::WidgetStore`] for widget-local state bound to widget identities,
/// and use this store when a component or helper needs named app-level slots.
#[derive(Default)]
pub struct Store {
    values: HashMap<String, Box<dyn Any>>,
}

impl Store {
    /// Creates an empty typed store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns whether the name is occupied, regardless of the stored value's type.
    pub fn contains<T>(&self, key: &StoreKey<T>) -> bool {
        self.values.contains_key(key.name())
    }

    /// Returns the value if the name exists and its concrete type matches `T`.
    pub fn get<T: 'static>(&self, key: &StoreKey<T>) -> Option<&T> {
        self.values.get(key.name())?.downcast_ref()
    }

    /// Returns mutable access if the name exists and its concrete type matches `T`.
    pub fn get_mut<T: 'static>(&mut self, key: &StoreKey<T>) -> Option<&mut T> {
        self.values.get_mut(key.name())?.downcast_mut()
    }

    /// Returns the stored value, inserting `T::default()` when the name is absent.
    ///
    /// # Panics
    ///
    /// Panics if the name already contains a value of a different type.
    pub fn get_or_default<T: Default + 'static>(&mut self, key: &StoreKey<T>) -> &mut T {
        self.values
            .entry(key.name().to_owned())
            .or_insert_with(|| Box::new(T::default()))
            .downcast_mut()
            .expect("Store type mismatch: a different type was stored for this key")
    }

    /// Replaces the named slot and returns the previous value if its type matches `T`.
    ///
    /// A previous value of another type is discarded and the return value is `None`.
    pub fn insert<T: 'static>(&mut self, key: StoreKey<T>, value: T) -> Option<T> {
        self.values
            .insert(key.name, Box::new(value))
            .and_then(|previous| previous.downcast::<T>().ok())
            .map(|boxed| *boxed)
    }

    /// Removes the named slot and returns its value if its type matches `T`.
    ///
    /// A value of another type is still removed, but the return value is `None`.
    pub fn remove<T: 'static>(&mut self, key: &StoreKey<T>) -> Option<T> {
        self.values
            .remove(key.name())
            .and_then(|value| value.downcast::<T>().ok())
            .map(|boxed| *boxed)
    }
}

impl fmt::Debug for Store {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut keys: Vec<_> = self.values.keys().map(String::as_str).collect();
        keys.sort_unstable();
        f.debug_struct("Store").field("keys", &keys).finish()
    }
}

/// Cached derived value keyed by a hash of the source inputs.
#[derive(Debug, Clone)]
pub struct Derived<T> {
    fingerprint: Option<u64>,
    value: Option<T>,
}

impl<T> Default for Derived<T> {
    fn default() -> Self {
        Self {
            fingerprint: None,
            value: None,
        }
    }
}

impl<T> Derived<T> {
    /// Creates an empty derived-value cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the cached value without recomputing it.
    pub fn get(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Clears the value and input fingerprint, forcing the next recomputation.
    pub fn invalidate(&mut self) {
        self.fingerprint = None;
        self.value = None;
    }

    /// Rebuilds the cached value only when the input hash changes.
    ///
    /// `build` is called on the first use or after invalidation. Equality is based
    /// on a 64-bit hash, so hash collisions can reuse a value for distinct inputs.
    pub fn recompute<I, F>(&mut self, input: &I, build: F) -> &T
    where
        I: Hash,
        F: FnOnce() -> T,
    {
        let fingerprint = hash_value(input);
        if self.fingerprint != Some(fingerprint) {
            self.value = Some(build());
            self.fingerprint = Some(fingerprint);
        }

        self.value
            .as_ref()
            .expect("Derived value should exist after recompute")
    }

    /// Consumes the cache and returns its stored value, if any.
    pub fn into_inner(self) -> Option<T> {
        self.value
    }
}

/// Form helper for app-level draft state, dirty tracking, and validation.
#[derive(Debug, Clone)]
pub struct FormState<T, E = String> {
    value: T,
    initial: T,
    touched: bool,
    submitting: bool,
    errors: HashMap<String, Vec<E>>,
}

impl<T: Default + Clone, E> Default for FormState<T, E> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Clone, E> FormState<T, E> {
    /// Creates an untouched, non-submitting form with `value` as both draft and baseline.
    pub fn new(value: T) -> Self {
        Self {
            initial: value.clone(),
            value,
            touched: false,
            submitting: false,
            errors: HashMap::new(),
        }
    }

    /// Returns the current draft.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Marks the form touched and returns mutable access to the draft.
    pub fn value_mut(&mut self) -> &mut T {
        self.touched = true;
        &mut self.value
    }

    /// Replaces the draft and marks the form touched, preserving errors and baseline.
    pub fn set_value(&mut self, value: T) {
        self.value = value;
        self.touched = true;
    }

    /// Mutates the draft with a closure and marks the form touched.
    pub fn update<F>(&mut self, update: F)
    where
        F: FnOnce(&mut T),
    {
        update(&mut self.value);
        self.touched = true;
    }

    /// Makes the current draft the baseline and clears touched state.
    ///
    /// Validation errors and the submitting flag are preserved.
    pub fn commit(&mut self) {
        self.initial = self.value.clone();
        self.touched = false;
    }

    /// Restores the baseline draft and clears touched state and errors.
    ///
    /// The submitting flag is preserved.
    pub fn reset(&mut self) {
        self.value = self.initial.clone();
        self.touched = false;
        self.errors.clear();
    }

    /// Replaces both baseline and draft, clearing touched state and errors.
    ///
    /// The submitting flag is preserved.
    pub fn replace_initial(&mut self, value: T) {
        self.initial = value.clone();
        self.value = value;
        self.touched = false;
        self.errors.clear();
    }

    /// Returns whether the draft differs from the baseline, independently of touched state.
    pub fn is_dirty(&self) -> bool
    where
        T: PartialEq,
    {
        self.value != self.initial
    }

    /// Returns whether the draft has been marked or accessed for mutation.
    pub fn is_touched(&self) -> bool {
        self.touched
    }

    /// Marks the draft as touched without changing its value.
    pub fn mark_touched(&mut self) {
        self.touched = true;
    }

    /// Returns the submitting flag.
    pub fn is_submitting(&self) -> bool {
        self.submitting
    }

    /// Sets the submitting flag without scheduling work.
    pub fn set_submitting(&mut self, submitting: bool) {
        self.submitting = submitting;
    }

    /// Sets and returns the submitting flag from whether the task status is active.
    ///
    /// `None` clears the flag.
    pub fn sync_submitting_with_task(&mut self, status: Option<&TaskStatus>) -> bool {
        let submitting = status.is_some_and(TaskStatus::is_active);
        self.submitting = submitting;
        submitting
    }

    /// Sets and returns the submitting flag from the request's loading phase.
    pub fn sync_submitting_with_request<V, Err>(&mut self, request: &RequestState<V, Err>) -> bool {
        let submitting = request.is_loading();
        self.submitting = submitting;
        submitting
    }

    /// Appends an error to a named field; existing field errors are preserved.
    pub fn set_error(&mut self, field: impl Into<String>, error: E) {
        self.errors.entry(field.into()).or_default().push(error);
    }

    /// Removes all validation errors.
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }

    /// Removes every error for the named field.
    pub fn clear_field_errors(&mut self, field: &str) {
        self.errors.remove(field);
    }

    /// Returns errors grouped by field name.
    pub fn errors(&self) -> &HashMap<String, Vec<E>> {
        &self.errors
    }

    /// Returns the field's errors, or `None` when the field has no stored entry.
    pub fn field_errors(&self, field: &str) -> Option<&[E]> {
        self.errors.get(field).map(Vec::as_slice)
    }

    /// Returns whether all stored error lists are empty.
    ///
    /// This checks recorded errors only; it does not run validation rules.
    pub fn is_valid(&self) -> bool {
        self.errors.values().all(Vec::is_empty)
    }
}

fn hash_value<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::{Derived, FormState, Store, StoreKey};

    #[test]
    fn derived_recomputes_only_when_input_changes() {
        let mut derived = Derived::new();
        let mut runs = 0;

        let first = *derived.recompute(&(1, 2), || {
            runs += 1;
            3
        });
        let second = *derived.recompute(&(1, 2), || {
            runs += 1;
            99
        });
        let third = *derived.recompute(&(2, 3), || {
            runs += 1;
            5
        });

        assert_eq!(first, 3);
        assert_eq!(second, 3);
        assert_eq!(third, 5);
        assert_eq!(runs, 2);
    }

    #[test]
    fn form_state_tracks_dirty_and_errors() {
        let mut form = FormState::<String>::new("hello".into());
        assert!(!form.is_dirty());

        form.set_value("world".into());
        assert!(form.is_dirty());
        assert!(form.is_touched());

        form.set_error("name", "required".into());
        assert!(!form.is_valid());
        assert_eq!(form.field_errors("name").unwrap(), ["required"]);

        form.commit();
        form.clear_errors();
        assert!(!form.is_dirty());
        assert!(form.is_valid());
    }

    #[test]
    fn store_preserves_types_per_key() {
        let mut store = Store::new();
        let count = StoreKey::<u32>::new("count");
        let title = StoreKey::<String>::new("title");

        *store.get_or_default(&count) += 1;
        store.insert(title.clone(), "hello".to_owned());

        assert_eq!(store.get(&count), Some(&1));
        assert_eq!(store.get(&title).map(String::as_str), Some("hello"));
        assert_eq!(store.remove(&title).as_deref(), Some("hello"));
        assert!(store.get(&title).is_none());
    }
}
