//! Spacer widget - occupies space without rendering

use super::*;
use crate::layout::Constraints;

/// Spacer widget that occupies space but doesn't render anything.
///
/// Useful for creating gaps in layouts or adjusting component positions.
///
/// # Examples
/// ```no_run
/// use tui::widget::spacer;
///
/// // Fixed size spacer
/// let spacer = spacer().fixed(10, 2);
///
/// // Flexible spacer that fills available space
/// let spacer = spacer().flex(1.0);
/// ```
#[derive(Debug, Clone)]
pub struct Spacer<M = ()> {
    constraints: Constraints,
    _phantom: std::marker::PhantomData<M>,
}

impl<M> Spacer<M> {
    /// Create a new spacer with default constraints (no size)
    pub fn new() -> Self {
        Self {
            constraints: Constraints::content(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set fixed width and height
    ///
    /// # Arguments
    /// * `width` - Fixed width in columns
    /// * `height` - Fixed height in rows
    ///
    /// # Examples
    /// ```no_run
    /// use tui::widget::Spacer;
    ///
    /// let spacer = Spacer::new().fixed(10, 2);
    /// ```
    pub fn fixed(mut self, width: u16, height: u16) -> Self {
        self.constraints = Constraints::fixed(width, height);
        self
    }

    /// Set minimum width and height
    ///
    /// # Arguments
    /// * `width` - Minimum width in columns
    /// * `height` - Minimum height in rows
    pub fn min(mut self, width: u16, height: u16) -> Self {
        self.constraints = Constraints::min(width, height);
        self
    }

    /// Set width (fixed)
    ///
    /// # Arguments
    /// * `width` - Width in columns
    pub fn width(mut self, width: u16) -> Self {
        self.constraints.min_width = width;
        self.constraints.max_width = Some(width);
        self
    }

    /// Set height (fixed)
    ///
    /// # Arguments
    /// * `height` - Height in rows
    pub fn height(mut self, height: u16) -> Self {
        self.constraints.min_height = height;
        self.constraints.max_height = Some(height);
        self
    }

    /// Make the spacer flexible with the given flex factor
    ///
    /// # Arguments
    /// * `flex` - Flex factor (typically 1.0)
    ///
    /// # Examples
    /// ```no_run
    /// use tui::widget::Spacer;
    ///
    /// let spacer = Spacer::new().flex(1.0);
    /// ```
    pub fn flex(mut self, flex: f32) -> Self {
        self.constraints.flex = Some(flex);
        self
    }

    /// Fill all available space (equivalent to flex(1.0))
    pub fn fill(mut self) -> Self {
        self.constraints = Constraints::fill();
        self
    }
}

impl<M> Default for Spacer<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Send + Sync> Widget<M> for Spacer<M> {
    fn render(&self, _chunk: &mut render::chunk::Chunk) {
        // Spacer doesn't render anything
    }

    fn handle_event(&mut self, _event: &Event) -> EventResult<M> {
        // Spacer doesn't handle events
        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        self.constraints
    }
}

/// Create a new spacer widget (convenience function)
///
/// # Examples
/// ```no_run
/// use tui::prelude::*;
///
/// // Fixed size spacer
/// let s1 = spacer().fixed(10, 2);
///
/// // Horizontal spacer (width only)
/// let s2 = spacer().width(5).height(1);
///
/// // Flexible spacer
/// let s3 = spacer().fill();
/// ```
pub fn spacer<M>() -> Spacer<M> {
    Spacer::new()
}
