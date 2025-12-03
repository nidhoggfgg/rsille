//! Builder pattern trait for stateful widgets
//!
//! This module provides a trait that can be implemented by widgets using WidgetState,
//! eliminating the need to reimplement common builder methods across different widgets.

use super::state::WidgetState;
use crate::style::Style;

/// Builder trait for widgets that use WidgetState
///
/// This trait provides common builder methods for widgets that contain a WidgetState field.
/// Instead of each widget manually implementing `disabled()`, `style()`, and `focus_style()`
/// methods, they can simply implement this trait.
///
/// # Usage
///
/// Implement this trait for your widget by providing a mutable reference to its WidgetState:
///
/// ```rust
/// use tui::widget::common::{WidgetState, StatefulWidgetBuilder};
///
/// struct MyWidget<M> {
///     state: WidgetState,
///     // ... other fields
/// }
///
/// impl<M> StatefulWidgetBuilder for MyWidget<M> {
///     fn widget_state_mut(&mut self) -> &mut WidgetState {
///         &mut self.state
///     }
/// }
///
/// // Now you can use the builder methods:
/// let widget = MyWidget::new()
///     .disabled(true)
///     .style(Style::default().fg(Color::Cyan));
/// ```
pub trait StatefulWidgetBuilder: Sized {
    /// Get a mutable reference to the widget's state
    ///
    /// This method must be implemented to provide access to the widget's WidgetState.
    fn widget_state_mut(&mut self) -> &mut WidgetState;

    /// Set the disabled state
    ///
    /// Disabled widgets cannot be interacted with and use muted styling.
    ///
    /// # Examples
    /// ```
    /// # use tui::widget::common::{WidgetState, StatefulWidgetBuilder};
    /// # struct MyWidget { state: WidgetState }
    /// # impl StatefulWidgetBuilder for MyWidget {
    /// #     fn widget_state_mut(&mut self) -> &mut WidgetState { &mut self.state }
    /// # }
    /// # impl MyWidget {
    /// #     fn new() -> Self { Self { state: WidgetState::new() } }
    /// # }
    /// let widget = MyWidget::new().disabled(true);
    /// ```
    fn disabled(mut self, disabled: bool) -> Self {
        self.widget_state_mut().set_disabled(disabled);
        self
    }

    /// Set a custom style (overrides theme styling)
    ///
    /// # Examples
    /// ```
    /// # use tui::widget::common::{WidgetState, StatefulWidgetBuilder};
    /// # use tui::style::{Style, Color};
    /// # struct MyWidget { state: WidgetState }
    /// # impl StatefulWidgetBuilder for MyWidget {
    /// #     fn widget_state_mut(&mut self) -> &mut WidgetState { &mut self.state }
    /// # }
    /// # impl MyWidget {
    /// #     fn new() -> Self { Self { state: WidgetState::new() } }
    /// # }
    /// let widget = MyWidget::new()
    ///     .style(Style::default().fg(Color::Cyan));
    /// ```
    fn style(mut self, style: Style) -> Self {
        let state = self.widget_state_mut();
        state.custom_style = Some(style);
        self
    }

    /// Set a custom focus style (overrides theme focus styling)
    ///
    /// This allows advanced customization of the widget's appearance when focused.
    /// For most use cases, the theme's default focus styling is sufficient.
    ///
    /// # Examples
    /// ```
    /// # use tui::widget::common::{WidgetState, StatefulWidgetBuilder};
    /// # use tui::style::{Style, Color};
    /// # struct MyWidget { state: WidgetState }
    /// # impl StatefulWidgetBuilder for MyWidget {
    /// #     fn widget_state_mut(&mut self) -> &mut WidgetState { &mut self.state }
    /// # }
    /// # impl MyWidget {
    /// #     fn new() -> Self { Self { state: WidgetState::new() } }
    /// # }
    /// let widget = MyWidget::new()
    ///     .focus_style(Style::default().fg(Color::Cyan).bold());
    /// ```
    fn focus_style(mut self, style: Style) -> Self {
        let state = self.widget_state_mut();
        state.custom_focus_style = Some(style);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test widget implementation
    struct TestWidget {
        state: WidgetState,
    }

    impl TestWidget {
        fn new() -> Self {
            Self {
                state: WidgetState::new(),
            }
        }
    }

    impl StatefulWidgetBuilder for TestWidget {
        fn widget_state_mut(&mut self) -> &mut WidgetState {
            &mut self.state
        }
    }

    #[test]
    fn test_disabled_builder() {
        let widget = TestWidget::new().disabled(true);
        assert!(widget.state.is_disabled());
    }

    #[test]
    fn test_style_builder() {
        use crate::style::Color;
        let style = Style::default().fg(Color::Cyan);
        let widget = TestWidget::new().style(style);
        assert!(widget.state.custom_style.is_some());
    }

    #[test]
    fn test_focus_style_builder() {
        use crate::style::Color;
        let style = Style::default().fg(Color::Cyan).bold();
        let widget = TestWidget::new().focus_style(style);
        assert!(widget.state.custom_focus_style.is_some());
    }

    #[test]
    fn test_chained_builders() {
        use crate::style::Color;
        let widget = TestWidget::new()
            .disabled(true)
            .style(Style::default().fg(Color::Red))
            .focus_style(Style::default().fg(Color::Blue).bold());

        assert!(widget.state.is_disabled());
        assert!(widget.state.custom_style.is_some());
        assert!(widget.state.custom_focus_style.is_some());
    }
}
