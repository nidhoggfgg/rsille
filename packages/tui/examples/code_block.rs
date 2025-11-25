//! CodeBlock Example
//!
//! Demonstrates syntax highlighting for various programming languages,
//! line highlighting, and git diff-style markers.
//!
//! Run with: cargo run --example code_block

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct State {
    rust_code: String,
    javascript_code: String,
    snippet_code: String,
}

/// No messages needed for this static example
#[derive(Clone, Debug)]
enum Message {}

/// Update function - no changes for this example
fn update(_state: &mut State, _msg: Message) {}

/// View function - displays code blocks with syntax highlighting
fn view(state: &State) -> Container<Message> {
    col()
        .gap(1)
        .padding(Padding::new(0, 2, 1, 1))
        .child(label("Code Highlighting Demo").fg(Color::Cyan).bold())
        .child(label("Press Esc to quit").fg(Color::Indexed(8)))
        // Rust code example with line numbers and highlighted lines
        .child(
            label("Rust Example (with line highlighting on lines 3-5):")
                .fg(Color::Green)
                .bold(),
        )
        .child(
            code_block(&state.rust_code)
                .language("rust")
                .show_line_numbers(true)
                .highlight_lines(vec![3, 4, 5])
                .theme("Solarized (dark)"),
        )
        // JavaScript code example with git diff markers
        .child(
            label("JavaScript Example (with git diff markers):")
                .fg(Color::Magenta)
                .bold(),
        )
        .child(
            code_block(&state.javascript_code)
                .language("javascript")
                .show_line_numbers(true)
                .line_added(2)
                .line_added(6)
                .line_deleted(4)
                .theme("Solarized (dark)"),
        )
        // Code snippet example (starting from line 48)
        .child(
            label("Code Snippet Example (lines 48-52 with highlight):")
                .fg(Color::Blue)
                .bold(),
        )
        .child(
            code_block(&state.snippet_code)
                .language("rust")
                .show_line_numbers(true)
                .start_line(48)
                .highlight_line(49)
                .theme("Solarized (dark)"),
        )
}

fn main() -> Result<()> {
    let rust_code = r#"fn main() {
    let message = "Hello, Rust!";
    // Calculate fibonacci
    let fib = (0..10)
        .fold((0, 1), |(a, b), _| (b, a + b))
        .0;
    println!("Fibonacci: {}", fib);
}"#;

    let javascript_code = r#"function greet(name) {
    return `Hello, ${name}!`;
}

// Main execution
const message = greet("JavaScript");
console.log(message);

const numbers = [1, 2, 3, 4, 5];
const squared = numbers.map(x => x ** 2);
console.log("Squared:", squared);"#;

    let snippet_code = r#"    if config.enabled {
        process_data(&input);
    }
    Ok(result)
}"#;

    let app = App::new(State {
        rust_code: rust_code.to_string(),
        javascript_code: javascript_code.to_string(),
        snippet_code: snippet_code.to_string(),
    });

    app.run_inline(update, view)?;
    Ok(())
}
