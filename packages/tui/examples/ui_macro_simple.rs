//! Simple test for ui! macro

#![recursion_limit = "512"]

use tui::prelude::*;
use tui::ui;

#[derive(Clone, Debug)]
enum Message {
    Quit,
}

fn update(_state: &mut (), _msg: Message) {}

fn view(_state: &()) -> Container<Message> {
    ui! {
        col {
            label("Test 1"),
            label("Test 2"),
        }
    }
}

fn main() -> Result<()> {
    let app = App::new(());
    app.run(update, view)?;
    Ok(())
}
