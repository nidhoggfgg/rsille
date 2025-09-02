use std::io::stdout;

use rsille::{
    render,
    tui::Document,
};

fn main() {
    let doc = Document::from_html("<p>line1</p><p>line2</p>").unwrap();

    let mut render = render::Builder::default()
        .size((20, 20))
        .clear(false)
        .append_newline(true)
        .build_render(doc, stdout());

    render.render().unwrap();
}
