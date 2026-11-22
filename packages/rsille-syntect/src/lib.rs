//! Highlight source code for rsille's code viewer.
//!
//! ```
//! use rsille::tui::prelude::CodeViewer;
//! use rsille_syntect::highlight;
//!
//! let lines = highlight("fn main() {}", "rs");
//! let viewer = CodeViewer::<()>::from_lines(lines).height(6);
//! ```

use std::sync::LazyLock;

use rsille::render::text::{Line, Span};
use rsille::tui::style::{Color, Style};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_nonewlines);
static THEMES: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Highlights a document using the bundled base16-ocean.dark theme.
///
/// `language` accepts a syntax name or file extension, such as `"Rust"` or `"rs"`.
/// Unknown languages produce plain lines. Line boundaries follow [`str::lines`].
/// The returned lines own their text and can be reused as the viewer scrolls.
pub fn highlight(source: &str, language: &str) -> Vec<Line<'static>> {
    let Some(syntax) = SYNTAXES.find_syntax_by_token(language) else {
        return source
            .lines()
            .map(|text| Line::from(text.to_owned()))
            .collect();
    };
    let mut highlighter = HighlightLines::new(syntax, &THEMES.themes["base16-ocean.dark"]);
    source
        .lines()
        .map(|line| {
            let Ok(ranges) = highlighter.highlight_line(line, &SYNTAXES) else {
                return Line::from(line.to_owned());
            };
            let spans = ranges
                .into_iter()
                .map(|(style, text)| {
                    let fg = style.foreground;
                    Span::new(
                        text.to_owned(),
                        Style::default()
                            .fg(Color::Rgb(fg.r, fg.g, fg.b))
                            .to_render_style(),
                    )
                })
                .collect();
            Line::from_texts(spans)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_comment_state_spans_lines_and_ends_at_delimiter() {
        let lines = highlight("/* open\r\nfn\r\n*/\r\nfn main() {}\n", "rs");
        assert_eq!(lines.len(), 4);
        let comment = &lines[1].content[0];
        let keyword = &lines[3].content[0];
        assert_eq!(comment.content, "fn");
        assert_eq!(keyword.content, "fn");
        assert_ne!(comment.style, keyword.style);
        assert_eq!(
            keyword.style,
            highlight("fn main() {}", "Rust")[0].content[0].style
        );
    }

    #[test]
    fn line_boundaries_match_the_plain_viewer() {
        for language in ["rs", "unknown-language"] {
            for source in ["", "\n", "let π = 2;\r\n\r\n// 界\n", "a\tb", "end\r"] {
                let lines = highlight(source, language);
                let text: Vec<String> = lines
                    .iter()
                    .map(|line| {
                        line.content
                            .iter()
                            .map(|span| span.content.as_ref())
                            .collect()
                    })
                    .collect();
                assert_eq!(text, source.lines().collect::<Vec<_>>());
            }
        }
    }
}
