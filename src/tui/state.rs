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
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            marker: PhantomData,
        }
    }

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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contains<T>(&self, key: &StoreKey<T>) -> bool {
        self.values.contains_key(key.name())
    }

    pub fn get<T: 'static>(&self, key: &StoreKey<T>) -> Option<&T> {
        self.values.get(key.name())?.downcast_ref()
    }

    pub fn get_mut<T: 'static>(&mut self, key: &StoreKey<T>) -> Option<&mut T> {
        self.values.get_mut(key.name())?.downcast_mut()
    }

    pub fn get_or_default<T: Default + 'static>(&mut self, key: &StoreKey<T>) -> &mut T {
        self.values
            .entry(key.name().to_owned())
            .or_insert_with(|| Box::new(T::default()))
            .downcast_mut()
            .expect("Store type mismatch: a different type was stored for this key")
    }

    pub fn insert<T: 'static>(&mut self, key: StoreKey<T>, value: T) -> Option<T> {
        self.values
            .insert(key.name, Box::new(value))
            .and_then(|previous| previous.downcast::<T>().ok())
            .map(|boxed| *boxed)
    }

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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self) -> Option<&T> {
        self.value.as_ref()
    }

    pub fn invalidate(&mut self) {
        self.fingerprint = None;
        self.value = None;
    }

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
    pub fn new(value: T) -> Self {
        Self {
            initial: value.clone(),
            value,
            touched: false,
            submitting: false,
            errors: HashMap::new(),
        }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut T {
        self.touched = true;
        &mut self.value
    }

    pub fn set_value(&mut self, value: T) {
        self.value = value;
        self.touched = true;
    }

    pub fn update<F>(&mut self, update: F)
    where
        F: FnOnce(&mut T),
    {
        update(&mut self.value);
        self.touched = true;
    }

    pub fn commit(&mut self) {
        self.initial = self.value.clone();
        self.touched = false;
    }

    pub fn reset(&mut self) {
        self.value = self.initial.clone();
        self.touched = false;
        self.errors.clear();
    }

    pub fn replace_initial(&mut self, value: T) {
        self.initial = value.clone();
        self.value = value;
        self.touched = false;
        self.errors.clear();
    }

    pub fn is_dirty(&self) -> bool
    where
        T: PartialEq,
    {
        self.value != self.initial
    }

    pub fn is_touched(&self) -> bool {
        self.touched
    }

    pub fn mark_touched(&mut self) {
        self.touched = true;
    }

    pub fn is_submitting(&self) -> bool {
        self.submitting
    }

    pub fn set_submitting(&mut self, submitting: bool) {
        self.submitting = submitting;
    }

    pub fn sync_submitting_with_task(&mut self, status: Option<&TaskStatus>) -> bool {
        let submitting = status.is_some_and(TaskStatus::is_active);
        self.submitting = submitting;
        submitting
    }

    pub fn sync_submitting_with_request<V, Err>(&mut self, request: &RequestState<V, Err>) -> bool {
        let submitting = request.is_loading();
        self.submitting = submitting;
        submitting
    }

    pub fn set_error(&mut self, field: impl Into<String>, error: E) {
        self.errors.entry(field.into()).or_default().push(error);
    }

    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }

    pub fn clear_field_errors(&mut self, field: &str) {
        self.errors.remove(field);
    }

    pub fn errors(&self) -> &HashMap<String, Vec<E>> {
        &self.errors
    }

    pub fn field_errors(&self, field: &str) -> Option<&[E]> {
        self.errors.get(field).map(Vec::as_slice)
    }

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
