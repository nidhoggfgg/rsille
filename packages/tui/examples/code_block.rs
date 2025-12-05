//! CodeBlock Example
//!
//! Demonstrates syntax highlighting for various programming languages,
//! line highlighting, git diff-style markers, and border styles.
//!
//! Run with: cargo run --example code_block

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct State {
    rust_code: String,
    javascript_code: String,
    snippet_code: String,
    no_border_code: String,
    theme_bg_code: String,
}

/// Update function - no changes for this example
fn update(_state: &mut State, _msg: ()) {}

/// View function - displays code blocks with syntax highlighting
fn view(state: &State) -> impl Layout<()> {
    col()
        .gap(1)
        .padding(Padding::new(0, 2, 1, 1))
        .child(label("Code Highlighting Demo").fg(Color::Cyan).bold())
        .child(label("Press Esc to quit").fg(Color::Indexed(8)))
        .child(
            row()
                .gap(2)
                .child(
                    // Left column
                    col()
                        .gap(1)
                        // Rust code example with line numbers and highlighted lines
                        .child(
                            label("Rust Example (Rounded border):")
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
                            label("JavaScript (Single border, git diff):")
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
                                .border(BorderStyle::Single)
                                .theme("Solarized (dark)"),
                        ),
                )
                .child(
                    // Right column
                    col()
                        .gap(1)
                        // Code snippet example
                        .child(
                            label("Code Snippet (Double border):")
                                .fg(Color::Blue)
                                .bold(),
                        )
                        .child(
                            code_block(&state.snippet_code)
                                .language("rust")
                                .show_line_numbers(true)
                                .start_line(48)
                                .highlight_line(49)
                                .border(BorderStyle::Double)
                                .theme("Solarized (dark)"),
                        )
                        // No border example
                        .child(label("No Border Example:").fg(Color::Yellow).bold())
                        .child(
                            code_block(&state.no_border_code)
                                .language("rust")
                                .show_line_numbers(true)
                                .border(BorderStyle::None)
                                .theme("Solarized (dark)"),
                        )
                        // TUI theme background example
                        .child(label("TUI Theme Background:").fg(Color::Cyan).bold())
                        .child(
                            code_block(&state.theme_bg_code)
                                .language("rust")
                                .show_line_numbers(true)
                                .use_theme_background(true)
                                .theme("Solarized (dark)"),
                        ),
                ),
        )
}

fn main() -> WidgetResult<()> {
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

    let no_border_code = r#"fn add(a: i32, b: i32) -> i32 {
    a + b
}"#;

    let theme_bg_code = r#"struct User {
    name: String,
    email: String,
    active: bool,
}"#;

    let app = App::new(State {
        rust_code: rust_code.to_string(),
        javascript_code: javascript_code.to_string(),
        snippet_code: snippet_code.to_string(),
        no_border_code: no_border_code.to_string(),
        theme_bg_code: theme_bg_code.to_string(),
    });

    app.run_inline(update, view)?;
    Ok(())
}
