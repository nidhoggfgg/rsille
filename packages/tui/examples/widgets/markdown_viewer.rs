//! MarkdownViewer widget.
//!
//! Run with: `cargo run -p tui --example markdown_viewer`

use tui::prelude::*;

const MARKDOWN: &str = r#"# Release Notes

- focused examples
- scrollable content
- basic formatting

`markdown_viewer` renders common Markdown blocks.
"#;

fn main() -> WidgetResult<()> {
    App::new(())
        .mode(Mode::Inline)
        .update(|_, _, _ctx| {})
        .view(view)
        .run()
        .map(|_| ())
}

fn view(_: &()) -> impl Widget<()> {
    markdown_viewer(MARKDOWN).key("markdown").height(10)
}
