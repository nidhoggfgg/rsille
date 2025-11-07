//! Spacer widget - empty space for layout

use super::*;

/// Spacer widget for adding empty space in layouts
///
/// # Examples
/// ```
/// use tui::widget::Spacer;
///
/// let spacer = Spacer::new(10, 1); // 10 width, 1 height
/// let flexible_spacer = Spacer::flexible(); // Grows to fill available space
/// ```
#[derive(Debug, Clone)]
pub struct Spacer {
    width: Option<u16>,
    height: Option<u16>,
    flex: Option<f32>,
}

impl Spacer {
    /// Create a new spacer with fixed dimensions
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width: Some(width),
            height: Some(height),
            flex: None,
        }
    }

    /// Create a flexible spacer that grows to fill available space
    pub fn flexible() -> Self {
        Self {
            width: None,
            height: None,
            flex: Some(1.0),
        }
    }

    /// Create a spacer with custom flex value
    pub fn with_flex(flex: f32) -> Self {
        Self {
            width: None,
            height: None,
            flex: Some(flex),
        }
    }

    /// Create a horizontal spacer with fixed width
    pub fn horizontal(width: u16) -> Self {
        Self {
            width: Some(width),
            height: Some(1),
            flex: None,
        }
    }

    /// Create a vertical spacer with fixed height
    pub fn vertical(height: u16) -> Self {
        Self {
            width: Some(1),
            height: Some(height),
            flex: None,
        }
    }
}

impl Widget for Spacer {
    type Message = ();

    fn render(&self, _chunk: &mut render::chunk::Chunk) {
        // Spacer doesn't render anything
    }

    fn handle_event(&mut self, _event: &Event) -> EventResult<()> {
        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        Constraints {
            min_width: self.width.unwrap_or(0),
            max_width: self.width,
            min_height: self.height.unwrap_or(0),
            max_height: self.height,
            flex: self.flex,
        }
    }
}
