//! ProgressBar widget - visual progress indicator

use super::*;
use crate::style::{Color, Style, ThemeManager};
use unicode_width::UnicodeWidthStr;

/// Progress display mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProgressMode {
    /// Determinate mode with specific progress value (0.0 to 1.0)
    Determinate(f64),
    /// Indeterminate mode for unknown duration tasks
    Indeterminate,
}

/// Label display position relative to the progress bar
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelPosition {
    /// No label displayed
    #[default]
    None,
    /// Label displayed inside the bar (centered)
    Inside,
    /// Label displayed to the right of the bar
    Right,
}

/// ProgressBar widget for displaying task progress
///
/// Supports both determinate (0-100%) and indeterminate (unknown duration) modes.
/// Leverages the framework's Style and ThemeManager for consistent theming.
///
/// # Examples
/// ```
/// use tui::widget::{progress_bar, ProgressMode, LabelPosition};
///
/// // Determinate progress (50%)
/// let bar = progress_bar()
///     .progress(0.5)
///     .show_percentage();
///
/// // Indeterminate progress with label
/// let loading = progress_bar()
///     .indeterminate()
///     .label("Loading...");
///
/// // Custom styled progress bar
/// let custom = progress_bar()
///     .progress(0.75)
///     .width(40)
///     .label_position(LabelPosition::Right);
/// ```
#[derive(Clone)]
pub struct ProgressBar<M = ()> {
    mode: ProgressMode,
    width: Option<u16>,
    style: Style,
    filled_char: char,
    empty_char: char,
    label: Option<String>,
    label_position: LabelPosition,
    show_percentage: bool,
    _phantom: std::marker::PhantomData<M>,
}

impl<M> std::fmt::Debug for ProgressBar<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressBar")
            .field("mode", &self.mode)
            .field("width", &self.width)
            .field("style", &self.style)
            .field("filled_char", &self.filled_char)
            .field("empty_char", &self.empty_char)
            .field("label", &self.label)
            .field("label_position", &self.label_position)
            .field("show_percentage", &self.show_percentage)
            .finish()
    }
}

impl<M> ProgressBar<M> {
    /// Create a new progress bar with 0% progress
    ///
    /// # Examples
    /// ```
    /// use tui::widget::ProgressBar;
    ///
    /// let bar = ProgressBar::<()>::new();
    /// ```
    pub fn new() -> Self {
        Self {
            mode: ProgressMode::Determinate(0.0),
            width: None,
            style: Style::default(),
            filled_char: '█',
            empty_char: '░',
            label: None,
            label_position: LabelPosition::default(),
            show_percentage: false,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set the progress value (0.0 to 1.0)
    ///
    /// Values outside this range will be clamped.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::ProgressBar;
    ///
    /// let bar = ProgressBar::<()>::new().progress(0.5); // 50%
    /// ```
    pub fn progress(mut self, value: f64) -> Self {
        let clamped = value.clamp(0.0, 1.0);
        self.mode = ProgressMode::Determinate(clamped);
        self
    }

    /// Set the progress bar to indeterminate mode
    ///
    /// Used for tasks with unknown duration.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::ProgressBar;
    ///
    /// let bar = ProgressBar::<()>::new().indeterminate();
    /// ```
    pub fn indeterminate(mut self) -> Self {
        self.mode = ProgressMode::Indeterminate;
        self
    }

    /// Set a custom width for the progress bar
    ///
    /// If not set, the bar will expand to fill available space.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::ProgressBar;
    ///
    /// let bar = ProgressBar::<()>::new().width(40);
    /// ```
    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the filled character
    ///
    /// # Examples
    /// ```
    /// use tui::widget::ProgressBar;
    ///
    /// let bar = ProgressBar::<()>::new().filled_char('━');
    /// ```
    pub fn filled_char(mut self, ch: char) -> Self {
        self.filled_char = ch;
        self
    }

    /// Set the empty/background character
    ///
    /// # Examples
    /// ```
    /// use tui::widget::ProgressBar;
    ///
    /// let bar = ProgressBar::<()>::new().empty_char('─');
    /// ```
    pub fn empty_char(mut self, ch: char) -> Self {
        self.empty_char = ch;
        self
    }

    /// Set a custom label to display
    ///
    /// # Examples
    /// ```
    /// use tui::widget::ProgressBar;
    ///
    /// let bar = ProgressBar::<()>::new()
    ///     .label("Processing...")
    ///     .progress(0.5);
    /// ```
    pub fn label(mut self, text: impl Into<String>) -> Self {
        self.label = Some(text.into());
        self.label_position = LabelPosition::Inside;
        self
    }

    /// Set the label display position
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{ProgressBar, LabelPosition};
    ///
    /// let bar = ProgressBar::<()>::new()
    ///     .label("Loading")
    ///     .label_position(LabelPosition::Right);
    /// ```
    pub fn label_position(mut self, position: LabelPosition) -> Self {
        self.label_position = position;
        self
    }

    /// Show percentage text inside or next to the bar
    ///
    /// # Examples
    /// ```
    /// use tui::widget::ProgressBar;
    ///
    /// let bar = ProgressBar::<()>::new()
    ///     .progress(0.75)
    ///     .show_percentage();
    /// ```
    pub fn show_percentage(mut self) -> Self {
        self.show_percentage = true;
        if self.label_position == LabelPosition::None {
            self.label_position = LabelPosition::Inside;
        }
        self
    }

    /// Set the progress bar style
    ///
    /// # Examples
    /// ```
    /// use tui::widget::ProgressBar;
    /// use tui::style::{Style, Color};
    ///
    /// let bar = ProgressBar::<()>::new()
    ///     .style(Style::default().fg(Color::Green));
    /// ```
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the foreground color (fluent API)
    pub fn fg(mut self, color: Color) -> Self {
        self.style = self.style.fg(color);
        self
    }

    /// Set the background color (fluent API)
    pub fn bg(mut self, color: Color) -> Self {
        self.style = self.style.bg(color);
        self
    }

    /// Get the display text based on mode and settings
    fn get_display_text(&self) -> Option<String> {
        if self.label_position == LabelPosition::None {
            return None;
        }

        match self.mode {
            ProgressMode::Determinate(progress) => {
                if self.show_percentage {
                    let percentage = (progress * 100.0).round() as u8;
                    if let Some(ref label) = self.label {
                        Some(format!("{} {}%", label, percentage))
                    } else {
                        Some(format!("{}%", percentage))
                    }
                } else {
                    self.label.clone()
                }
            }
            ProgressMode::Indeterminate => self.label.clone(),
        }
    }

    /// Render indeterminate progress animation
    fn render_indeterminate(&self, chunk: &mut render::chunk::Chunk, bar_width: u16) {
        // Simple indeterminate animation: moving block
        // In a real implementation, this would use a tick/frame counter
        // For now, just show a repeating pattern
        let pattern = "▓▒░";
        let pattern_len = pattern.chars().count();

        let mut bar = String::new();
        for i in 0..bar_width as usize {
            let ch = pattern.chars().nth(i % pattern_len).unwrap();
            bar.push(ch);
        }

        let theme_style = ThemeManager::global()
            .with_theme(|theme| Style::default().fg(theme.colors.primary));
        let final_style = self.style.merge(theme_style);
        let render_style = final_style.to_render_style();

        let _ = chunk.set_string(0, 0, &bar, render_style);
    }
}

impl<M> Default for ProgressBar<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Send + Sync> Widget<M> for ProgressBar<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        // Calculate bar width based on label position
        let display_text = self.get_display_text();
        let (bar_width, label_x) = match self.label_position {
            LabelPosition::Right => {
                if let Some(ref text) = display_text {
                    let text_width = text.width() as u16;
                    let bar_w = area.width().saturating_sub(text_width + 1);
                    (bar_w, bar_w + 1)
                } else {
                    (area.width(), 0)
                }
            }
            _ => (area.width(), 0),
        };

        if bar_width == 0 {
            return;
        }

        // Handle indeterminate mode
        if matches!(self.mode, ProgressMode::Indeterminate) {
            self.render_indeterminate(chunk, bar_width);

            // Render label if positioned outside
            if self.label_position == LabelPosition::Right {
                if let Some(ref text) = display_text {
                    let text_style = ThemeManager::global()
                        .with_theme(|theme| theme.styles.text.to_render_style());
                    let _ = chunk.set_string(label_x, 0, text, text_style);
                }
            }
            return;
        }

        // Determinate mode rendering
        let progress = if let ProgressMode::Determinate(p) = self.mode {
            p
        } else {
            0.0
        };

        let filled_width = (bar_width as f64 * progress).round() as u16;
        let empty_width = bar_width.saturating_sub(filled_width);

        // Get theme colors
        let (filled_style, empty_style) = ThemeManager::global().with_theme(|theme| {
            let filled = Style::default().fg(theme.colors.primary).bg(theme.colors.primary);
            let empty = Style::default().fg(theme.colors.border);
            (filled, empty)
        });

        // Apply custom style on top of theme
        let final_filled_style = self.style.merge(filled_style);
        let final_empty_style = empty_style;

        // Build the progress bar string
        let filled_bar = self.filled_char.to_string().repeat(filled_width as usize);
        let empty_bar = self.empty_char.to_string().repeat(empty_width as usize);

        // Render filled portion
        if filled_width > 0 {
            let _ = chunk.set_string(0, 0, &filled_bar, final_filled_style.to_render_style());
        }

        // Render empty portion
        if empty_width > 0 {
            let _ = chunk.set_string(
                filled_width,
                0,
                &empty_bar,
                final_empty_style.to_render_style(),
            );
        }

        // Render label text
        if let Some(ref text) = display_text {
            match self.label_position {
                LabelPosition::Inside => {
                    // Center text inside the bar
                    let text_width = text.width() as u16;
                    if text_width <= bar_width {
                        let text_x = (bar_width - text_width) / 2;

                        // Use contrasting color for text visibility
                        let text_style = ThemeManager::global().with_theme(|theme| {
                            Style::default()
                                .fg(theme.colors.background)
                                .bold()
                                .to_render_style()
                        });

                        let _ = chunk.set_string(text_x, 0, text, text_style);
                    }
                }
                LabelPosition::Right => {
                    // Text already positioned, render it
                    let text_style = ThemeManager::global()
                        .with_theme(|theme| theme.styles.text.to_render_style());
                    let _ = chunk.set_string(label_x, 0, text, text_style);
                }
                LabelPosition::None => {}
            }
        }
    }

    fn handle_event(&mut self, _event: &Event) -> EventResult<M> {
        // ProgressBar doesn't handle events
        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        // Calculate width based on label position
        let display_text = self.get_display_text();

        let (min_width, max_width) = if let Some(width) = self.width {
            // Fixed width specified
            let total_width = if self.label_position == LabelPosition::Right {
                if let Some(ref text) = display_text {
                    width + text.width() as u16 + 1
                } else {
                    width
                }
            } else {
                width
            };
            (total_width, Some(total_width))
        } else {
            // Flexible width - expand to fill
            let min = if self.label_position == LabelPosition::Right {
                if let Some(ref text) = display_text {
                    10 + text.width() as u16 + 1 // Minimum bar width + label
                } else {
                    10
                }
            } else {
                10 // Minimum bar width
            };
            (min, None)
        };

        Constraints {
            min_width,
            max_width,
            min_height: 1,
            max_height: Some(1),
            flex: if max_width.is_none() {
                Some(1.0) // Expand to fill available space
            } else {
                None
            },
        }
    }
}

/// Create a new progress bar widget (convenience function)
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// let bar = progress_bar()
///     .progress(0.5)
///     .show_percentage();
///
/// let loading = progress_bar()
///     .indeterminate()
///     .label("Loading...");
/// ```
pub fn progress_bar<M>() -> ProgressBar<M> {
    ProgressBar::new()
}
