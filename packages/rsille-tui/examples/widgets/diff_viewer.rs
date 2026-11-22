//! DiffViewer widget.
//!
//! Run with: `cargo run -p rsille-tui --example diff_viewer`

use rsille_tui::prelude::*;

const DIFF: &str = r#"diff --git a/example.rs b/example.rs
@@ -1,4 +1,4 @@
-let label = "old";
+let label = "new";
 let enabled = true;
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
    diff_viewer(DIFF).key("diff").height(8)
}
