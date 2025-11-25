//! CodeBlock widget - syntax highlighted code display

use super::*;
use crate::layout::border_renderer;
use crate::style::{BorderStyle, Color, Style, ThemeManager};
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

/// Global syntax set (loaded once)
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);

/// Global theme set (loaded once)
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// Line marker type for diff-style highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineMarker {
    /// Normal line (no marker)
    Normal,
    /// Added line (green background, + prefix)
    Added,
    /// Deleted line (red background, - prefix)
    Deleted,
}

/// CodeBlock widget for displaying syntax-highlighted code
#[derive(Debug, Clone)]
pub struct CodeBlock<M = ()> {
    content: String,
    language: Option<String>,
    show_line_numbers: bool,
    theme_name: String,
    highlighted_lines: HashSet<usize>,
    line_markers: HashMap<usize, LineMarker>,
    start_line: usize,
    border: Option<BorderStyle>,
    use_theme_background: bool,
    _phantom: std::marker::PhantomData<M>,
}

impl<M> CodeBlock<M> {
    /// Create a new code block with the specified code
    ///
    /// # Examples
    /// ```
    /// use tui::widget::CodeBlock;
    ///
    /// let code_block = CodeBlock::new("fn main() {\n    println!(\"Hello\");\n}");
    /// ```
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            language: None,
            show_line_numbers: false,
            theme_name: "base16-ocean.dark".to_string(),
            highlighted_lines: HashSet::new(),
            line_markers: HashMap::new(),
            start_line: 1,
            border: Some(BorderStyle::Rounded),
            use_theme_background: false,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set the programming language for syntax highlighting
    ///
    /// Supported languages include: "rs"/"rust", "py"/"python", "js"/"javascript",
    /// "go", "c", "cpp", "java", "html", "css", "json", "yaml", etc.
    pub fn language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    /// Enable or disable line numbers
    pub fn show_line_numbers(mut self, show: bool) -> Self {
        self.show_line_numbers = show;
        self
    }

    /// Set the color theme
    ///
    /// Available themes: "base16-ocean.dark", "base16-eighties.dark",
    /// "base16-mocha.dark", "base16-ocean.light", "InspiredGitHub", etc.
    pub fn theme(mut self, theme_name: impl Into<String>) -> Self {
        self.theme_name = theme_name.into();
        self
    }

    /// Set the border style
    ///
    /// # Examples
    /// ```
    /// use tui::widget::CodeBlock;
    /// use tui::style::BorderStyle;
    ///
    /// let code = CodeBlock::new("fn main() {}")
    ///     .border(BorderStyle::Single);  // Use single line border
    ///
    /// let no_border = CodeBlock::new("fn main() {}")
    ///     .border(BorderStyle::None);    // Disable border
    /// ```
    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = match border {
            BorderStyle::None => None,
            other => Some(other),
        };
        self
    }

    /// Use TUI theme background instead of syntect theme background
    ///
    /// By default, code blocks use the background color from the syntect theme.
    /// When set to true, code blocks will use the TUI framework's theme background
    /// color instead, making them blend better with the overall UI.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::CodeBlock;
    ///
    /// let code = CodeBlock::new("fn main() {}")
    ///     .language("rust")
    ///     .use_theme_background(true);  // Use TUI theme background
    /// ```
    pub fn use_theme_background(mut self, use_theme: bool) -> Self {
        self.use_theme_background = use_theme;
        self
    }

    /// Set the starting line number for code snippets
    ///
    /// Use this when displaying a code snippet that doesn't start from line 1.
    /// All line numbers, highlights, and markers should use the actual line numbers
    /// from the original file.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::CodeBlock;
    ///
    /// // Display a code snippet from lines 50-55
    /// let code = CodeBlock::new("    return result;\n}\n")
    ///     .language("rust")
    ///     .start_line(50)      // Line numbers start at 50
    ///     .highlight_line(50); // Highlight line 50
    /// ```
    pub fn start_line(mut self, line: usize) -> Self {
        self.start_line = line.max(1); // Ensure at least 1
        self
    }

    /// Highlight a specific line (1-indexed)
    ///
    /// # Examples
    /// ```
    /// use tui::widget::CodeBlock;
    ///
    /// let code = CodeBlock::new("line 1\nline 2\nline 3")
    ///     .highlight_line(2); // Highlight line 2
    /// ```
    pub fn highlight_line(mut self, line: usize) -> Self {
        if line > 0 {
            self.highlighted_lines.insert(line);
        }
        self
    }

    /// Highlight multiple lines (1-indexed)
    ///
    /// # Examples
    /// ```
    /// use tui::widget::CodeBlock;
    ///
    /// let code = CodeBlock::new("line 1\nline 2\nline 3\nline 4")
    ///     .highlight_lines(vec![2, 4]); // Highlight lines 2 and 4
    /// ```
    pub fn highlight_lines(mut self, lines: impl IntoIterator<Item = usize>) -> Self {
        for line in lines {
            if line > 0 {
                self.highlighted_lines.insert(line);
            }
        }
        self
    }

    /// Mark a line as added (git diff style, 1-indexed)
    ///
    /// # Examples
    /// ```
    /// use tui::widget::CodeBlock;
    ///
    /// let code = CodeBlock::new("line 1\nline 2")
    ///     .line_added(2); // Mark line 2 as added
    /// ```
    pub fn line_added(mut self, line: usize) -> Self {
        if line > 0 {
            self.line_markers.insert(line, LineMarker::Added);
        }
        self
    }

    /// Mark a line as deleted (git diff style, 1-indexed)
    ///
    /// # Examples
    /// ```
    /// use tui::widget::CodeBlock;
    ///
    /// let code = CodeBlock::new("line 1\nline 2")
    ///     .line_deleted(1); // Mark line 1 as deleted
    /// ```
    pub fn line_deleted(mut self, line: usize) -> Self {
        if line > 0 {
            self.line_markers.insert(line, LineMarker::Deleted);
        }
        self
    }

    /// Mark a line with a specific marker type (1-indexed)
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{CodeBlock, LineMarker};
    ///
    /// let code = CodeBlock::new("line 1\nline 2")
    ///     .mark_line(1, LineMarker::Added);
    /// ```
    pub fn mark_line(mut self, line: usize, marker: LineMarker) -> Self {
        if line > 0 {
            if marker == LineMarker::Normal {
                self.line_markers.remove(&line);
            } else {
                self.line_markers.insert(line, marker);
            }
        }
        self
    }

    /// Mark multiple lines with diff markers
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{CodeBlock, LineMarker};
    ///
    /// let code = CodeBlock::new("line 1\nline 2\nline 3")
    ///     .mark_lines(vec![(1, LineMarker::Deleted), (3, LineMarker::Added)]);
    /// ```
    pub fn mark_lines(mut self, lines: impl IntoIterator<Item = (usize, LineMarker)>) -> Self {
        for (line, marker) in lines {
            if line > 0 {
                if marker == LineMarker::Normal {
                    self.line_markers.remove(&line);
                } else {
                    self.line_markers.insert(line, marker);
                }
            }
        }
        self
    }

    /// Get the text content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Find the syntax definition for the current language
    fn find_syntax(&self) -> &'static SyntaxReference {
        if let Some(ref lang) = self.language {
            // Try to normalize language name to extension or proper name
            let lang_lower = lang.to_lowercase();
            let normalized = match lang_lower.as_str() {
                "rust" => "rs",
                "python" => "py",
                "javascript" => "js",
                "typescript" => "ts",
                "markdown" => "md",
                "yaml" => "yml",
                other => other,
            };

            SYNTAX_SET
                .find_syntax_by_extension(normalized)
                .or_else(|| SYNTAX_SET.find_syntax_by_name(lang))
                .or_else(|| {
                    // Try with capitalized name (e.g., "rust" -> "Rust")
                    let mut chars = lang.chars();
                    let capitalized = chars
                        .next()
                        .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
                        .unwrap_or_default();
                    SYNTAX_SET.find_syntax_by_name(&capitalized)
                })
                .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text())
        } else {
            SYNTAX_SET.find_syntax_plain_text()
        }
    }

    /// Get the theme for highlighting
    fn get_theme(&self) -> &'static Theme {
        THEME_SET
            .themes
            .get(&self.theme_name)
            .unwrap_or_else(|| &THEME_SET.themes["base16-ocean.dark"])
    }

    /// Convert syntect color to TUI color
    fn syntect_to_color(color: syntect::highlighting::Color) -> Color {
        Color::Rgb(color.r, color.g, color.b)
    }

    /// Convert syntect style to TUI style
    fn syntect_to_style(style: SyntectStyle) -> Style {
        // Only use foreground color for syntax highlighting
        // Don't set background to preserve terminal's default background
        let mut tui_style = Style::default().fg(Self::syntect_to_color(style.foreground));

        if style
            .font_style
            .contains(syntect::highlighting::FontStyle::BOLD)
        {
            tui_style = tui_style.bold();
        }
        if style
            .font_style
            .contains(syntect::highlighting::FontStyle::ITALIC)
        {
            tui_style = tui_style.italic();
        }
        if style
            .font_style
            .contains(syntect::highlighting::FontStyle::UNDERLINE)
        {
            tui_style = tui_style.underlined();
        }

        tui_style
    }

    /// Calculate the width needed for line numbers
    fn line_number_width(&self) -> usize {
        if !self.show_line_numbers {
            return 0;
        }
        let line_count = self.content.lines().count();
        if line_count == 0 {
            return 0;
        }
        // Calculate the maximum line number that will be displayed
        let max_line_number = self.start_line + line_count - 1;
        // Width = digits + space + separator + space
        max_line_number.to_string().len() + 3
    }
}

impl<M: Send + Sync> Widget<M> for CodeBlock<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        // Calculate offset and available size (account for border if present)
        let (x_offset, y_offset, available_width, available_height) = if self.border.is_some() {
            // Border takes 1 cell on each side
            if area.width() < 2 || area.height() < 2 {
                return; // Not enough space for border
            }
            (1u16, 1u16, area.width() - 2, area.height() - 2)
        } else {
            (0u16, 0u16, area.width(), area.height())
        };

        let syntax = self.find_syntax();
        let theme = self.get_theme();
        let mut highlighter = HighlightLines::new(syntax, theme);

        let line_num_width = self.line_number_width();
        // Check if we need to show diff markers at all
        let has_diff_markers = !self.line_markers.is_empty();
        // Get background color for normal lines
        let theme_bg = if self.use_theme_background {
            // Use TUI theme background
            ThemeManager::global().with_theme(|theme| theme.colors.background)
        } else {
            // Use syntect theme background
            Self::syntect_to_color(theme.settings.background.unwrap_or(
                syntect::highlighting::Color { r: 0, g: 0, b: 0, a: 255 }
            ))
        };
        let mut y = 0u16;

        for (line_idx, line) in LinesWithEndings::from(&self.content).enumerate() {
            if y >= available_height {
                break;
            }

            let line_number = self.start_line + line_idx; // Use start_line offset
            let mut x = 0u16;

            // Determine line background color and diff marker
            let line_marker = self.line_markers.get(&line_number).copied();
            let is_highlighted = self.highlighted_lines.contains(&line_number);

            let (bg_color, diff_prefix) = match line_marker {
                Some(LineMarker::Added) => (Some(Color::Rgb(0, 64, 0)), "+"),
                Some(LineMarker::Deleted) => (Some(Color::Rgb(64, 0, 0)), "-"),
                Some(LineMarker::Normal) | None => {
                    if is_highlighted {
                        (Some(Color::Rgb(40, 40, 40)), " ")
                    } else {
                        // Use theme background color for normal lines
                        (Some(theme_bg), " ")
                    }
                }
            };

            // Render line number if enabled
            if self.show_line_numbers {
                let line_num = format!("{:>width$} │ ", line_number, width = line_num_width - 3);
                let mut line_num_style = Style::default().fg(Color::Rgb(100, 100, 100));
                if let Some(bg) = bg_color {
                    line_num_style = line_num_style.bg(bg);
                }
                let _ = chunk.set_string(x_offset + x, y_offset + y, &line_num, line_num_style.to_render_style());
                x += line_num_width as u16;
            }

            // Render diff marker if any line has markers (to maintain alignment)
            if has_diff_markers {
                let mut prefix_style = Style::default();
                if let Some(bg) = bg_color {
                    prefix_style = prefix_style.bg(bg);
                }
                if let Some(LineMarker::Added) = line_marker {
                    prefix_style = prefix_style.fg(Color::Green);
                } else if let Some(LineMarker::Deleted) = line_marker {
                    prefix_style = prefix_style.fg(Color::Red);
                }
                let _ = chunk.set_string(x_offset + x, y_offset + y, diff_prefix, prefix_style.to_render_style());
                x += 1;
                // Add a space after the marker
                let _ = chunk.set_string(x_offset + x, y_offset + y, " ", prefix_style.to_render_style());
                x += 1;
            }

            // Highlight and render code
            match highlighter.highlight_line(line, &SYNTAX_SET) {
                Ok(ranges) => {
                    for (style, text) in ranges {
                        let mut tui_style = Self::syntect_to_style(style);
                        // Apply background color if needed
                        if let Some(bg) = bg_color {
                            tui_style = tui_style.bg(bg);
                        }
                        let render_style = tui_style.to_render_style();

                        // Remove trailing newline for rendering
                        let text_to_render = text.trim_end_matches('\n').trim_end_matches('\r');
                        if !text_to_render.is_empty() && x < available_width {
                            // Limit text to not exceed available width
                            let max_chars = (available_width - x) as usize;
                            let text_slice = if text_to_render.len() > max_chars {
                                &text_to_render[..max_chars]
                            } else {
                                text_to_render
                            };
                            let _ = chunk.set_string(x_offset + x, y_offset + y, text_slice, render_style);
                            x += text_slice.len() as u16;
                        }
                    }

                    // Fill rest of line with background color if needed
                    if let Some(bg) = bg_color {
                        if x < available_width {
                            let fill_width = available_width - x;
                            if fill_width > 0 {
                                let fill = " ".repeat(fill_width as usize);
                                let fill_style = Style::default().bg(bg).to_render_style();
                                let _ = chunk.set_string(x_offset + x, y_offset + y, &fill, fill_style);
                            }
                        }
                    }
                }
                Err(_) => {
                    // Fallback: render without highlighting
                    let text = line.trim_end_matches('\n').trim_end_matches('\r');
                    let mut fallback_style = Style::default();
                    if let Some(bg) = bg_color {
                        fallback_style = fallback_style.bg(bg);
                    }

                    if x < available_width {
                        // Limit text to not exceed available width
                        let max_chars = (available_width - x) as usize;
                        let text_slice = if text.len() > max_chars {
                            &text[..max_chars]
                        } else {
                            text
                        };
                        let _ = chunk.set_string(x_offset + x, y_offset + y, text_slice, fallback_style.to_render_style());
                        let text_width = text_slice.len() as u16;

                        // Fill rest of line with background color if needed
                        if let Some(bg) = bg_color {
                            let new_x = x + text_width;
                            if new_x < available_width {
                                let fill_width = available_width - new_x;
                                if fill_width > 0 {
                                    let fill = " ".repeat(fill_width as usize);
                                    let fill_style = Style::default().bg(bg).to_render_style();
                                    let _ = chunk.set_string(x_offset + new_x, y_offset + y, &fill, fill_style);
                                }
                            }
                        }
                    }
                }
            }

            y += 1;
        }

        // Render border last with theme background to cover any overflow
        if let Some(border_style) = self.border {
            let border_color = ThemeManager::global()
                .with_theme(|theme| theme.colors.border);
            // Use theme surface background for border to avoid code background bleeding through
            let border_bg = ThemeManager::global()
                .with_theme(|theme| theme.colors.background);
            let border_style_obj = Style::default().fg(border_color).bg(border_bg);
            border_renderer::render_border(chunk, border_style, border_style_obj.to_render_style());
        }
    }

    fn handle_event(&mut self, _event: &Event) -> EventResult<M> {
        // CodeBlocks don't handle events by default
        EventResult::Ignored
    }

    fn constraints(&self) -> Constraints {
        use unicode_width::UnicodeWidthStr;

        let line_num_width = self.line_number_width();
        let line_count = self.content.lines().count().max(1);

        // Calculate max line width
        let max_width = self
            .content
            .lines()
            .map(|line| line.width())
            .max()
            .unwrap_or(0);

        // Account for diff markers if any
        let diff_marker_width = if !self.line_markers.is_empty() { 2 } else { 0 };

        let content_width = (line_num_width + diff_marker_width + max_width) as u16;
        let content_height = line_count as u16;

        // Add border space if border is present (2 cells total: 1 on each side)
        let border_offset = if self.border.is_some() { 2 } else { 0 };

        let total_width = content_width + border_offset;
        let total_height = content_height + border_offset;

        Constraints {
            min_width: total_width,
            max_width: Some(total_width),
            min_height: total_height,
            max_height: Some(total_height),
            flex: None,
        }
    }
}

/// Create a new code block widget (convenience function)
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// let code = code_block("fn main() {\n    println!(\"Hello\");\n}")
///     .language("rust")
///     .show_line_numbers(true)
///     .theme("base16-ocean.dark");
/// ```
pub fn code_block<M>(content: impl Into<String>) -> CodeBlock<M> {
    CodeBlock::new(content)
}
