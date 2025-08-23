use std::io::stdout;

use rsille::{render, tui::widgets::boxed::Boxed, tui::widgets::Text};

fn main() {
    let b = Boxed::new(Text::new("Hello, world!"));
    let mut render = render::Builder::new()
        .pos((0, 0))
        .size((10, 3))
        .append_newline(true)
        .build_render(b, stdout());

    render.render().unwrap();
}