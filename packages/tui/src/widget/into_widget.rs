use super::Widget;

/// Trait for types that can be converted into a widget
///
/// This trait enables automatic type conversion, eliminating the need
/// for explicit `as Box<dyn Widget<M>>` casts throughout the codebase.
pub trait IntoWidget<M: Send + Sync> {
    /// Convert this type into a boxed widget
    fn into_widget(self) -> Box<dyn Widget<M>>;
}

// Implement for all Widget types
impl<M: Send + Sync, W: Widget<M> + 'static> IntoWidget<M> for W {
    fn into_widget(self) -> Box<dyn Widget<M>> {
        Box::new(self)
    }
}
