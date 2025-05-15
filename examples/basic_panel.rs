use std::io::stdout;

use rsille::{
    render,
    tui::{composite::Panel, widgets::Text},
};

fn main() {
    let num = "hello world!!!";
    let text = Text::new(&num.to_string());

    let mut panel = Panel::new(10, 10);
    panel.push(text);

    let render = render::Builder::new().build_render(panel, stdout());
    render.render().unwrap();
}
