//! List widget example
//!
//! Demonstrates the List widget with various features:
//! - Single and multiple selection modes
//! - Keyboard navigation (arrows, PageUp/Down, Home/End)
//! - Mouse interaction and scroll
//! - Scrollbar for large lists
//! - Disabled items
//! - Dividers between items
//! - Selection callbacks
//!
//! Controls:
//! - Up/Down Arrow: Navigate list
//! - Home/End: Jump to first/last item
//! - PageUp/PageDown: Jump by page
//! - Enter/Space: Toggle selection
//! - Mouse wheel: Scroll
//! - M: Toggle between single and multiple selection modes
//! - Q/Esc: Quit
//!
//! Run with: cargo run --example list

use tui::prelude::*;

#[derive(Clone, Debug, PartialEq)]
enum Fruit {
    Apple,
    Banana,
    Cherry,
    Date,
    Elderberry,
    Fig,
    Grape,
    Honeydew,
    Kiwi,
    Lemon,
    Mango,
    Orange,
    Papaya,
    Raspberry,
    Strawberry,
}

impl std::fmt::Display for Fruit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
struct State {
    // Selection mode toggle
    show_single: bool,

    // Single selection list state
    single_focused_index: Option<usize>,
    single_scroll_offset: usize,
    single_selected: Vec<usize>,
    single_selection_text: String,

    // Multiple selection list state
    multi_focused_index: Option<usize>,
    multi_scroll_offset: usize,
    multi_selected: Vec<usize>,
    multi_selection_text: String,
}

#[derive(Clone, Debug)]
enum Message {
    SingleSelected(SelectionEvent<Fruit>),
    MultipleSelected(SelectionEvent<Fruit>),
    ToggleMode,
    Quit,
}

fn update(state: &mut State, msg: Message) {
    match msg {
        Message::SingleSelected(event) => {
            // Save list state
            state.single_focused_index = event.focused_index;
            state.single_scroll_offset = event.scroll_offset;

            if event.selected_values.is_empty() {
                state.single_selection_text = "No selection".to_string();
                state.single_selected.clear();
            } else {
                state.single_selection_text = format!("Selected: {}", event.selected_values[0]);
                // Update selected indices based on the fruit values
                state.single_selected = get_single_items()
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| event.selected_values.contains(&item.value))
                    .map(|(idx, _)| idx)
                    .collect();
            }
        }
        Message::MultipleSelected(event) => {
            // Save list state
            state.multi_focused_index = event.focused_index;
            state.multi_scroll_offset = event.scroll_offset;

            if event.selected_values.is_empty() {
                state.multi_selection_text = "No selection".to_string();
                state.multi_selected.clear();
            } else {
                let fruit_names: Vec<String> = event
                    .selected_values
                    .iter()
                    .map(|f| f.to_string())
                    .collect();
                state.multi_selection_text = format!("Selected: {}", fruit_names.join(", "));
                // Update selected indices based on the fruit values
                state.multi_selected = get_multi_items()
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| event.selected_values.contains(&item.value))
                    .map(|(idx, _)| idx)
                    .collect();
            }
        }
        Message::ToggleMode => {
            state.show_single = !state.show_single;
        }
        Message::Quit => {
            std::process::exit(0);
        }
    }
}

// Helper functions to get list items
fn get_single_items() -> Vec<ListItem<Fruit>> {
    vec![
        ListItem::new(Fruit::Apple, "🍎 Apple"),
        ListItem::new(Fruit::Banana, "🍌 Banana"),
        ListItem::new(Fruit::Cherry, "🍒 Cherry").with_divider(),
        ListItem::new(Fruit::Date, "Date (Out of season)").disabled(true),
        ListItem::new(Fruit::Elderberry, "Elderberry"),
        ListItem::new(Fruit::Fig, "🍈 Fig").with_divider(),
        ListItem::new(Fruit::Grape, "🍇 Grape"),
        ListItem::new(Fruit::Honeydew, "🍈 Honeydew"),
    ]
}

fn get_multi_items() -> Vec<ListItem<Fruit>> {
    vec![
        ListItem::new(Fruit::Apple, "🍎 Apple"),
        ListItem::new(Fruit::Banana, "🍌 Banana"),
        ListItem::new(Fruit::Cherry, "🍒 Cherry"),
        ListItem::new(Fruit::Date, "Date").with_divider(),
        ListItem::new(Fruit::Elderberry, "Elderberry (Low stock)").disabled(true),
        ListItem::new(Fruit::Fig, "🍈 Fig"),
        ListItem::new(Fruit::Grape, "🍇 Grape"),
        ListItem::new(Fruit::Honeydew, "🍈 Honeydew"),
        ListItem::new(Fruit::Kiwi, "🥝 Kiwi").with_divider(),
        ListItem::new(Fruit::Lemon, "🍋 Lemon"),
        ListItem::new(Fruit::Mango, "🥭 Mango"),
        ListItem::new(Fruit::Orange, "🍊 Orange"),
        ListItem::new(Fruit::Papaya, "Papaya"),
        ListItem::new(Fruit::Raspberry, "Raspberry"),
        ListItem::new(Fruit::Strawberry, "🍓 Strawberry"),
    ]
}

fn view_single(state: &State) -> Container<Message> {
    // Single selection mode
    col()
        .padding(Padding::new(2, 2, 1, 1))
        .gap(1)
        // Title
        .child(
            label("List Demo - Single Selection Mode")
                .fg(Color::Cyan)
                .bold(),
        )
        .child(label("Press 'M' to switch to Multiple Selection mode").fg(Color::Indexed(8)))
        .child(label(""))
        // Instructions
        .child(
            label("Navigation: ↑/↓ arrows, Home/End, PgUp/PgDn | Select: Enter/Space")
                .fg(Color::Yellow),
        )
        .child(label(""))
        // List
        .child(
            list()
                .items(get_single_items())
                .show_scrollbar(false)
                .selection_mode(SelectionMode::Single)
                .viewport_height(8)
                .focused_index(state.single_focused_index)
                .scroll_offset(state.single_scroll_offset)
                .selected(state.single_selected.clone())
                .on_select(|event| Message::SingleSelected(event)),
        )
        .child(label(""))
        // Selection display
        .child(
            label(format!(
                "Current Selection:\n{}",
                state.single_selection_text
            ))
            .fg(Color::Green),
        )
        .child(label(""))
        // Footer
        .child(label("Q or Esc: Quit | M: Toggle mode").fg(Color::Indexed(8)))
        // Keyboard controller
        .child(
            keyboard_controller()
                .on('q', || Message::Quit)
                .on('m', || Message::ToggleMode)
                .on('M', || Message::ToggleMode)
                .on_key(KeyCode::Esc, || Message::Quit),
        )
}

fn view_multiple(state: &State) -> Container<Message> {
    // Multiple selection mode
    col()
        .padding(Padding::new(2, 2, 1, 1))
        .gap(1)
        // Title
        .child(
            label("List Demo - Multiple Selection Mode")
                .fg(Color::Cyan)
                .bold(),
        )
        .child(label("Press 'M' to switch to Single Selection mode").fg(Color::Indexed(8)))
        .child(label(""))
        // Instructions
        .child(
            label(
                "Navigation: ↑/↓, Home/End, PgUp/PgDn | Toggle: Enter/Space | Scroll: Mouse wheel",
            )
            .fg(Color::Yellow),
        )
        .child(label(""))
        // List with more items (scrollable)
        .child(
            list()
                .items(get_multi_items())
                .selection_mode(SelectionMode::Multiple)
                .viewport_height(10)
                .focused_index(state.multi_focused_index)
                .scroll_offset(state.multi_scroll_offset)
                .selected(state.multi_selected.clone())
                .on_select(|event| Message::MultipleSelected(event)),
        )
        .child(label(""))
        // Selection display
        .child(
            label(format!(
                "Current Selection:\n{}",
                state.multi_selection_text
            ))
            .fg(Color::Green),
        )
        .child(label(""))
        // Footer
        .child(label("Q or Esc: Quit | M: Toggle mode").fg(Color::Indexed(8)))
        // Keyboard controller
        .child(
            keyboard_controller()
                .on('q', || Message::Quit)
                .on('m', || Message::ToggleMode)
                .on('M', || Message::ToggleMode)
                .on_key(KeyCode::Esc, || Message::Quit),
        )
}

fn view(state: &State) -> Container<Message> {
    if state.show_single {
        view_single(state)
    } else {
        view_multiple(state)
    }
}

fn main() -> Result<()> {
    let app = App::new(State {
        show_single: true,
        single_focused_index: None,
        single_scroll_offset: 0,
        single_selected: vec![],
        single_selection_text: "No selection".to_string(),
        multi_focused_index: None,
        multi_scroll_offset: 0,
        multi_selected: vec![],
        multi_selection_text: "No selection".to_string(),
    });

    app.run_inline(update, view)?;
    Ok(())
}
