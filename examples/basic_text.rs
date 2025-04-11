use rsille::{render::Render, tui::widgets::Text};

fn main() {
    let num = "hello world!!!";
    let text = Text::new(&num.to_string());

    let render = Render::new(text);
    render.render().unwrap();
}
