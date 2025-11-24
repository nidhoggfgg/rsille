//! Test with a minimal macro

use tui::prelude::*;

macro_rules! test_ui {
    // Entry point
    (col { $($children:tt)* }) => {{
        let container = col();
        test_ui!(@children container; $($children)*)
    }};

    // Add children - label followed by comma
    (@children $c:expr; label($text:expr), $($rest:tt)*) => {
        test_ui!(@children $c.child(label($text)); $($rest)*)
    };

    // Add children - last label without comma
    (@children $c:expr; label($text:expr)) => {
        $c.child(label($text))
    };

    // Base case - trailing comma
    (@children $c:expr; ,) => {
        $c
    };

    // Base case - empty
    (@children $c:expr;) => {
        $c
    };
}

#[derive(Clone, Debug)]
enum Message {
    Quit,
}

fn update(_state: &mut (), _msg: Message) {}

fn view(_state: &()) -> Container<Message> {
    test_ui! {
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
