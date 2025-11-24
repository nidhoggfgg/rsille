//! Test without macro

use tui::prelude::*;

#[derive(Clone, Debug)]
enum Message {
    Quit,
}

fn update(_state: &mut (), _msg: Message) {}

fn view(_state: &()) -> Container<Message> {
    col()
        .child(label("Test 1"))
        .child(label("Test 2"))
}

fn main() -> Result<()> {
    let app = App::new(());
    app.run(update, view)?;
    Ok(())
}
