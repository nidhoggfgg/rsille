//! Stable namespaces shared by widget instances and their resources.
//!
//! A scope is a sequence of literal names, not a slash-separated path. Use the
//! same segments for widget scopes, effects, and subscriptions so resource
//! lifetimes follow the mounted component instance.
//!
//! # Examples
//!
//! ```
//! use rsille::tui::scope::Scope;
//!
//! let nested = Scope::new("editor").child("search");
//! assert_eq!(nested.segments(), &["editor", "search"]);
//! assert_ne!(nested, Scope::new("editor/search"));
//! assert!(Scope::root().segments().is_empty());
//! ```

use std::fmt;

/// A resource namespace. Segments remain distinct even when names contain `/`.
/// Match this path with nested `WidgetExt::scope` declarations in the view.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Scope(Vec<String>);

impl Scope {
    /// Returns the empty root namespace.
    pub fn root() -> Self {
        Self::default()
    }

    /// Creates a scope containing one literal segment.
    ///
    /// Slashes in `name` are not interpreted as separators.
    pub fn new(name: impl Into<String>) -> Self {
        Self(vec![name.into()])
    }

    /// Returns a new scope with one literal segment appended, leaving this scope unchanged.
    pub fn child(&self, name: impl Into<String>) -> Self {
        let mut scope = self.clone();
        scope.0.push(name.into());
        scope
    }

    /// Returns the namespace segments in outermost-to-innermost order.
    pub fn segments(&self) -> &[String] {
        &self.0
    }

    pub(crate) fn join(&self, inner: &Self) -> Self {
        let mut scope = self.clone();
        scope.0.extend(inner.0.iter().cloned());
        scope
    }
}

impl From<&str> for Scope {
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

impl From<String> for Scope {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, segment) in self.0.iter().enumerate() {
            if index > 0 {
                write!(f, "/")?;
            }
            write!(f, "{segment:?}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ResourceKey {
    pub scope: Scope,
    pub key: String,
}

impl ResourceKey {
    pub fn new(scope: Scope, key: impl Into<String>) -> Self {
        Self {
            scope,
            key: key.into(),
        }
    }
}
