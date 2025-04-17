use rsille::{render, tui::widgets::Text};

fn main() {
    let num = "hello world!!!";
    let text = Text::new(&num.to_string());

    let el = render::Builder::new().full_screen().build_eventloop(text);
    el.run();
}
