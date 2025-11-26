//! Table widget example
//!
//! Demonstrates the Table widget with various features:
//! - Column definitions with different width strategies
//! - Single and multiple selection modes
//! - Keyboard navigation (arrows, PageUp/Down, Home/End)
//! - Mouse interaction and scroll
//! - Scrollbar for large datasets
//! - Custom styling
//! - Selection callbacks
//!
//! Controls:
//! - Up/Down Arrow: Navigate rows
//! - Home/End: Jump to first/last row
//! - PageUp/PageDown: Jump by page
//! - Enter/Space: Toggle selection
//! - Mouse wheel: Scroll
//! - M: Toggle between single and multiple selection modes
//! - Q/Esc: Quit
//!
//! Run with: cargo run --example table

use tui::prelude::*;

#[derive(Clone, Debug, PartialEq)]
struct User {
    id: u32,
    name: String,
    email: String,
    role: String,
    status: String,
}

impl User {
    fn new(id: u32, name: &str, email: &str, role: &str, status: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            status: status.to_string(),
        }
    }
}

#[derive(Debug)]
struct State {
    // Selection mode toggle
    use_multiple_selection: bool,

    // Table state
    focused_index: Option<usize>,
    scroll_offset: usize,
    selected: Vec<usize>,
    selection_text: String,
}

#[derive(Clone, Debug)]
enum Message {
    UserSelected(TableSelectionEvent<User>),
    ToggleMode,
    Quit,
}

fn update(state: &mut State, msg: Message) {
    match msg {
        Message::UserSelected(event) => {
            // Save table state
            state.focused_index = event.focused_index;
            state.scroll_offset = event.scroll_offset;

            if event.selected_rows.is_empty() {
                state.selection_text = "No selection".to_string();
                state.selected.clear();
            } else {
                let user_names: Vec<String> = event
                    .selected_rows
                    .iter()
                    .map(|u| format!("{} ({})", u.name, u.email))
                    .collect();
                state.selection_text = format!("Selected:\n{}", user_names.join("\n"));

                // Update selected indices based on user IDs
                state.selected = get_users()
                    .iter()
                    .enumerate()
                    .filter(|(_, user)| event.selected_rows.iter().any(|u| u.id == user.id))
                    .map(|(idx, _)| idx)
                    .collect();
            }
        }
        Message::ToggleMode => {
            state.use_multiple_selection = !state.use_multiple_selection;
            // Clear selection when switching modes
            state.selected.clear();
            state.selection_text = "No selection".to_string();
        }
        Message::Quit => {
            std::process::exit(0);
        }
    }
}

fn get_users() -> Vec<User> {
    vec![
        User::new(1, "Alice Johnson", "alice@example.com", "Admin", "Active"),
        User::new(2, "Bob Smith", "bob@example.com", "Developer", "Active"),
        User::new(
            3,
            "Charlie Brown",
            "charlie@example.com",
            "Designer",
            "Away",
        ),
        User::new(4, "Diana Prince", "diana@example.com", "Manager", "Active"),
        User::new(5, "Eve Martinez", "eve@example.com", "Developer", "Active"),
        User::new(
            6,
            "Frank Wilson",
            "frank@example.com",
            "Developer",
            "Inactive",
        ),
        User::new(7, "Grace Lee", "grace@example.com", "QA Engineer", "Active"),
        User::new(8, "Henry Davis", "henry@example.com", "DevOps", "Active"),
        User::new(9, "Iris Chen", "iris@example.com", "Data Scientist", "Away"),
        User::new(
            10,
            "Jack Thompson",
            "jack@example.com",
            "Developer",
            "Active",
        ),
        User::new(
            11,
            "Karen White",
            "karen@example.com",
            "Product Manager",
            "Active",
        ),
        User::new(12, "Leo Garcia", "leo@example.com", "Developer", "Active"),
        User::new(
            13,
            "Mia Robinson",
            "mia@example.com",
            "UX Designer",
            "Active",
        ),
        User::new(
            14,
            "Noah Taylor",
            "noah@example.com",
            "Backend Engineer",
            "Inactive",
        ),
        User::new(
            15,
            "Olivia Anderson",
            "olivia@example.com",
            "Frontend Engineer",
            "Active",
        ),
    ]
}

fn get_columns() -> Vec<Column<User>> {
    vec![
        Column::new("ID", |user: &User| user.id.to_string()).width(ColumnWidth::Fixed(6)),
        Column::new("Name", |user: &User| user.name.clone()).width(ColumnWidth::Flex(2)),
        Column::new("Email", |user: &User| user.email.clone()).width(ColumnWidth::Flex(3)),
        Column::new("Role", |user: &User| user.role.clone()).width(ColumnWidth::Flex(2)),
        Column::new("Status", |user: &User| match user.status.as_str() {
            "Active" => format!("✓ {}", user.status),
            "Away" => format!("○ {}", user.status),
            "Inactive" => format!("✗ {}", user.status),
            _ => user.status.clone(),
        })
        .width(ColumnWidth::Fixed(12)),
    ]
}

fn view(state: &State) -> impl Layout<Message> {
    let selection_mode = if state.use_multiple_selection {
        SelectionMode::Multiple
    } else {
        SelectionMode::Single
    };

    let mode_text = if state.use_multiple_selection {
        "Multiple Selection Mode"
    } else {
        "Single Selection Mode"
    };

    col()
        .padding(Padding::new(2, 2, 1, 1))
        .gap(1)
        // Title
        .child(
            label(format!("Table Demo - {}", mode_text))
                .fg(Color::Cyan)
                .bold(),
        )
        .child(label("Press 'M' to toggle selection mode").fg(Color::Indexed(8)))
        .child(spacer().height(1))
        // Instructions
        .child(
            label(
                "Navigation: ↑/↓, Home/End, PgUp/PgDn | Select: Enter/Space | Scroll: Mouse wheel",
            )
            .fg(Color::Yellow),
        )
        .child(spacer().height(1))
        // Table
        .child(
            table(get_columns())
                .rows(get_users())
                .selection_mode(selection_mode)
                .viewport_height(12)
                .focused_index(state.focused_index)
                .scroll_offset(state.scroll_offset)
                .selected(state.selected.clone())
                .show_header(true)
                .show_scrollbar(true)
                .on_select(|event| Message::UserSelected(event)),
        )
        .child(spacer().height(1))
        // Selection display
        .child(label(&state.selection_text).fg(Color::Green))
        .child(spacer().height(1))
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

fn main() -> Result<()> {
    let app = App::new(State {
        use_multiple_selection: false,
        focused_index: None,
        scroll_offset: 0,
        selected: vec![],
        selection_text: "No selection".to_string(),
    });

    app.run_inline(update, view)?;
    Ok(())
}
