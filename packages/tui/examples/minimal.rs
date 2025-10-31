//! Minimal test - just text, no styles

use tui::prelude::*;

#[derive(Debug)]
struct Test;

#[derive(Clone, Debug)]
enum Message {}

fn update(_state: &mut Test, _msg: Message) {}

fn view(_state: &Test) -> Container<Message> {
    Container::vertical(vec![
        Label::new("Line 1").into(),
        Label::new("Line 2").into(),
        Label::new("Line 3").into(),
    ])
    .gap(1)
    .padding(Padding::uniform(2))
}

fn main() -> Result<()> {
    let app = App::new(Test);
    app.run(update, view)?;
    Ok(())
}
