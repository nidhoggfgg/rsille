use rsille::{
    render::Render,
    tui::{composite::Panel, widgets::Text},
};

fn main() {
    let num = "hello world!!!";
    let text = Text::new(&num.to_string());

    let mut panel = Panel::new(10, 10);
    panel.push(text);

    let render = Render::new(panel);
    render.render().unwrap();
}
