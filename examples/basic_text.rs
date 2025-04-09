use rsille::{render::event_loop, tui::widgets::Text};

fn main() {
    let num = "hello world!!!";
    let text = Text::new(&num.to_string());

    let event_loop = event_loop::Builder::new().enable_all().build(text).unwrap();

    event_loop.run();
}
