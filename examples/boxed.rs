use std::io::stdout;

use rsille::{render, tui::widgets::Boxed, tui::widgets::Text};

fn main() {
    let b = Boxed::new(Text::new(
        "Hello, world!\n你好，世界\nlong long long long\n长长长长长长长长长长",
    ));
    let mut render = render::Builder::new()
        .pos((0, 0))
        .size((20, 7))
        .append_newline(true)
        .build_render(b, stdout());

    render.render().unwrap();
}
