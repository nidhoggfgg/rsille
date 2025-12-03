//! Widget lifecycle hooks and metadata
//!
//! This module provides optional lifecycle hooks and metadata for widgets,
//! enabling debugging, monitoring, and advanced widget behaviors.
//!
//! # Design Philosophy
//!
//! In a declarative UI framework where the widget tree is rebuilt on each frame,
//! traditional mount/unmount semantics don't directly apply. Instead, this module
//! provides:
//!
//! - **Metadata hooks**: Widget identification and debugging information
//! - **Render hooks**: Pre/post render notifications for tracking and monitoring
//! - **State hooks**: Notifications for state changes (for stateful widgets)
//!
//! All hooks have default implementations that do nothing, making them opt-in.

use render::area::Area;

/// Widget lifecycle hooks
///
/// This trait provides optional hooks that widgets can implement for debugging,
/// monitoring, state management, and advanced behaviors.
///
/// # When to Use
///
/// Implement WidgetLifecycle when you need:
/// - Custom logging or debugging for specific widgets
/// - Performance monitoring and profiling
/// - State persistence or serialization hooks
/// - Custom behavior on render or layout changes
///
/// # Examples
///
/// ```
/// use tui::widget::{Widget, lifecycle::WidgetLifecycle};
/// use render::chunk::Chunk;
/// use render::area::Area;
///
/// struct MyWidget {
///     name: String,
///     render_count: std::cell::Cell<usize>,
/// }
///
/// impl<M: Send + Sync> WidgetLifecycle<M> for MyWidget {
///     fn widget_name(&self) -> Option<&str> {
///         Some(&self.name)
///     }
///
///     fn on_before_render(&self, area: &Area) {
///         let count = self.render_count.get() + 1;
///         self.render_count.set(count);
///         println!("Rendering {} (count: {}) in {:?}", self.name, count, area);
///     }
///
///     fn on_after_render(&self, area: &Area) {
///         println!("Finished rendering {} in {:?}", self.name, area);
///     }
/// }
/// ```
pub trait WidgetLifecycle<M> {
    /// Get the widget's name for debugging and logging
    ///
    /// This is useful for identifying widgets in logs, profiling data, and error messages.
    ///
    /// # Default
    /// Returns `None` - no name by default.
    fn widget_name(&self) -> Option<&str> {
        None
    }

    /// Get the widget's type name
    ///
    /// Returns the Rust type name of the widget, useful for debugging.
    ///
    /// # Default
    /// Returns the type name using `std::any::type_name`.
    fn widget_type(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// Called before the widget is rendered
    ///
    /// This hook is called just before `render()` with the allocated area.
    /// Useful for:
    /// - Logging and debugging render operations
    /// - Performance profiling
    /// - Last-minute state updates based on allocated space
    ///
    /// # Arguments
    /// * `area` - The area allocated to this widget
    ///
    /// # Default
    /// Does nothing.
    fn on_before_render(&self, _area: &Area) {
        // Default: no-op
    }

    /// Called after the widget is rendered
    ///
    /// This hook is called immediately after `render()` completes.
    /// Useful for:
    /// - Logging render completion
    /// - Performance measurements
    /// - Post-render cleanup or notifications
    ///
    /// # Arguments
    /// * `area` - The area that was rendered to
    ///
    /// # Default
    /// Does nothing.
    fn on_after_render(&self, _area: &Area) {
        // Default: no-op
    }

    /// Called when the widget's allocated area changes
    ///
    /// This hook is called when the layout system assigns a different area
    /// to the widget compared to the previous frame.
    ///
    /// Useful for:
    /// - Responding to resize events
    /// - Updating internal caches or buffers
    /// - Logging layout changes
    ///
    /// # Arguments
    /// * `old_area` - The previous area (if any)
    /// * `new_area` - The new area
    ///
    /// # Note
    /// This requires the framework to track previous areas, which may add overhead.
    /// Consider using `on_before_render` with custom tracking if needed.
    ///
    /// # Default
    /// Does nothing.
    fn on_area_changed(&self, _old_area: Option<&Area>, _new_area: &Area) {
        // Default: no-op
    }

    /// Get debug information about the widget
    ///
    /// Returns a string containing debugging information about the widget's state.
    /// This is useful for debugging tools, logging, and error reporting.
    ///
    /// # Default
    /// Returns the widget type name.
    fn debug_info(&self) -> String {
        format!("{}", self.widget_type())
    }

    /// Get profiling metadata for performance monitoring
    ///
    /// Returns optional profiling data such as render times, event handling times, etc.
    /// This data can be collected by profiling tools or custom monitoring systems.
    ///
    /// # Default
    /// Returns `None` - no profiling data by default.
    fn profiling_metadata(&self) -> Option<ProfilingData> {
        None
    }
}

/// Profiling data for performance monitoring
///
/// This structure contains timing and performance information for a widget.
/// It can be collected by profiling tools to identify performance bottlenecks.
#[derive(Debug, Clone, Default)]
pub struct ProfilingData {
    /// Last render duration in microseconds
    pub last_render_micros: Option<u64>,

    /// Average render duration over recent frames
    pub avg_render_micros: Option<u64>,

    /// Number of times this widget has been rendered
    pub render_count: usize,

    /// Last event handling duration in microseconds
    pub last_event_micros: Option<u64>,

    /// Total number of events handled
    pub event_count: usize,

    /// Custom metrics (widget-specific)
    pub custom_metrics: Vec<(String, String)>,
}

impl ProfilingData {
    /// Create a new empty profiling data
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a custom metric
    pub fn add_metric(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.custom_metrics.push((key.into(), value.into()));
    }
}

/// Helper macro to create a scoped profiler that measures execution time
///
/// # Examples
///
/// ```ignore
/// fn render(&self, chunk: &mut Chunk) {
///     profile_scope!("MyWidget::render");
///     // ... rendering code ...
/// }
/// ```
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        #[cfg(feature = "profiling")]
        let _profiler = $crate::widget::lifecycle::ScopedProfiler::new($name);
    };
}

/// Scoped profiler for measuring execution time
///
/// This struct automatically measures the time between its creation and destruction.
/// Enable the "profiling" feature to activate profiling.
#[cfg(feature = "profiling")]
pub struct ScopedProfiler {
    name: &'static str,
    start: std::time::Instant,
}

#[cfg(feature = "profiling")]
impl ScopedProfiler {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: std::time::Instant::now(),
        }
    }
}

#[cfg(feature = "profiling")]
impl Drop for ScopedProfiler {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        // In a real implementation, this would send data to a profiling system
        eprintln!("[PROFILE] {} took {:?}", self.name, elapsed);
    }
}

#[cfg(not(feature = "profiling"))]
pub struct ScopedProfiler;

#[cfg(not(feature = "profiling"))]
impl ScopedProfiler {
    pub fn new(_name: &'static str) -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use render::area::{Position, Size};

    struct TestWidget {
        name: String,
    }

    impl<M> WidgetLifecycle<M> for TestWidget {
        fn widget_name(&self) -> Option<&str> {
            Some(&self.name)
        }
    }

    #[test]
    fn test_widget_name() {
        let widget = TestWidget {
            name: "test_widget".to_string(),
        };
        assert_eq!(<TestWidget as WidgetLifecycle<()>>::widget_name(&widget), Some("test_widget"));
    }

    #[test]
    fn test_default_hooks() {
        let widget = TestWidget {
            name: "test".to_string(),
        };

        let area = Area::new(
            Position { x: 0, y: 0 },
            Size { width: 10, height: 10 },
        );

        // These should not panic
        <TestWidget as WidgetLifecycle<()>>::on_before_render(&widget, &area);
        <TestWidget as WidgetLifecycle<()>>::on_after_render(&widget, &area);
        <TestWidget as WidgetLifecycle<()>>::on_area_changed(&widget, None, &area);
    }

    #[test]
    fn test_profiling_data() {
        let mut data = ProfilingData::new();
        data.render_count = 5;
        data.add_metric("custom", "value");

        assert_eq!(data.render_count, 5);
        assert_eq!(data.custom_metrics.len(), 1);
        assert_eq!(data.custom_metrics[0].0, "custom");
    }
}
