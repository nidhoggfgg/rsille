//! CodeViewer widget.
//!
//! Run with: `cargo run -p tui --example code_viewer`

use tui::prelude::*;

const CODE: &str = r#"fn filter(items: &[String], query: &str) -> Vec<String> {
    items
        .iter()
        .filter(|item| item.contains(query))
        .cloned()
        .collect()
}"#;

fn main() -> WidgetResult<()> {
    App::new(())
        .mode(Mode::Inline)
        .update(|_, _, _ctx| {})
        .view(view)
        .run()
        .map(|_| ())
}

fn view(_: &()) -> impl Widget<()> {
    code_viewer(CODE)
        .key("code")
        .language("rs")
        .height(10)
        .line_numbers(true)
}
