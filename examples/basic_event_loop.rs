use rsille::{render::event_loop, tui::widgets::Text};

fn main() {
    let num = "hello world!!!";
    let text = Text::new(&num.to_string());

    let el = event_loop::Builder::new()
        .full_screen()
        .build(text)
        .unwrap();
    el.run();
}
