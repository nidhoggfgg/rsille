//! Advanced Grid Layout Example
//!
//! Demonstrates:
//! - repeat() function for repeated columns/rows
//! - Responsive card grid layout
//! - Dashboard-style layout with sidebars
//! - Complex multi-row layouts
//!
//! Controls:
//! - q/Esc: Quit
//!
//! Run with: cargo run --example grid_advanced

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct State {
    selected_card: Option<usize>,
}

/// Messages from UI interactions
#[derive(Clone, Debug)]
enum Message {
    SelectCard(usize),
    Quit,
}

/// Update function - handles messages
fn update(state: &mut State, msg: Message) {
    match msg {
        Message::SelectCard(index) => {
            state.selected_card = Some(index);
        }
        Message::Quit => {
            std::process::exit(0);
        }
    }
}

/// Create a card widget
fn card(title: &str, content: &str, index: usize, is_selected: bool) -> impl Layout<Message> {
    let color = if is_selected {
        Color::Yellow
    } else {
        Color::Cyan
    };

    col()
        .border(BorderStyle::Rounded)
        .padding(Padding::new(1, 1, 1, 1))
        .child(label(title).fg(color).bold())
        .child(spacer().height(1))
        .child(label(content).fg(Color::Indexed(250)))
        .child(spacer().height(1))
        .child(button("Select").on_click(move || Message::SelectCard(index)))
}

/// View function - builds the UI
fn view(state: &State) -> impl Layout<Message> {
    col()
        .padding(Padding::new(2, 2, 1, 1))
        .gap(1)
        // Header
        .child(label("Advanced Grid Layouts").fg(Color::Cyan).bold())
        .child(spacer().height(1))
        // Example 1: repeat() function
        .child(
            label("Example 1: repeat(4, 1fr) - 4 equal columns")
                .fg(Color::Green)
                .bold(),
        )
        .child(
            grid()
                .columns("repeat(4, 1fr)")
                .rows("auto")
                .gap(1)
                .border(BorderStyle::Rounded)
                .child(label("Col 1").fg(Color::Yellow).bg(Color::Indexed(235)))
                .child(label("Col 2").fg(Color::Yellow).bg(Color::Indexed(235)))
                .child(label("Col 3").fg(Color::Yellow).bg(Color::Indexed(235)))
                .child(label("Col 4").fg(Color::Yellow).bg(Color::Indexed(235))),
        )
        .child(spacer().height(1))
        // Example 2: Dashboard layout
        .child(
            label("Example 2: Dashboard (sidebar-content-sidebar)")
                .fg(Color::Green)
                .bold(),
        )
        .child(
            grid()
                .columns("15 1fr 15")
                .rows("auto")
                .gap(1)
                .border(BorderStyle::Rounded)
                .child(
                    col()
                        .padding(Padding::new(1, 1, 0, 0))
                        .child(label("Left").fg(Color::Blue))
                        .child(label("Menu").fg(Color::Blue)),
                )
                .child(
                    col()
                        .padding(Padding::new(1, 1, 0, 0))
                        .child(label("Main Content Area").fg(Color::Magenta).bold())
                        .child(label("This is where the main content goes"))
                        .child(label("It takes up all available space")),
                )
                .child(
                    col()
                        .padding(Padding::new(1, 1, 0, 0))
                        .child(label("Right").fg(Color::Blue))
                        .child(label("Panel").fg(Color::Blue)),
                ),
        )
        .child(spacer().height(1))
        // Example 3: Card grid
        .child(
            label("Example 3: Card Grid - repeat(3, 1fr)")
                .fg(Color::Green)
                .bold(),
        )
        .child(
            grid()
                .columns("repeat(3, 1fr)")
                .rows("auto auto")
                .gap(1)
                .child(card(
                    "Card 1",
                    "First card",
                    0,
                    state.selected_card == Some(0),
                ))
                .child(card(
                    "Card 2",
                    "Second card",
                    1,
                    state.selected_card == Some(1),
                ))
                .child(card(
                    "Card 3",
                    "Third card",
                    2,
                    state.selected_card == Some(2),
                ))
                .child(card(
                    "Card 4",
                    "Fourth card",
                    3,
                    state.selected_card == Some(3),
                ))
                .child(card(
                    "Card 5",
                    "Fifth card",
                    4,
                    state.selected_card == Some(4),
                ))
                .child(card(
                    "Card 6",
                    "Sixth card",
                    5,
                    state.selected_card == Some(5),
                )),
        )
        .child(spacer().height(1))
        // Status
        .child(
            label(format!(
                "Selected card: {}",
                state
                    .selected_card
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "None".to_string())
            ))
            .fg(Color::Yellow),
        )
        .child(spacer().height(1))
        // Footer
        .child(label("Press 'q' or Esc to quit").fg(Color::Indexed(8)))
}

fn main() -> WidgetResult<()> {
    let app = App::new(State {
        selected_card: None,
    });
    app.on_key(KeyCode::Char('q'), || Message::Quit)
        .run(update, view)?;
    Ok(())
}
