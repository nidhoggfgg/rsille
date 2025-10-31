//! List Example
//!
//! Demonstrates a todo list with checkboxes and dynamic items.
//! Note: Full List widget implementation coming in Phase 7.
//!
//! Run with: cargo run --example list

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct TodoApp {
    items: Vec<TodoItem>,
}

#[derive(Debug, Clone)]
struct TodoItem {
    title: String,
    completed: bool,
}

/// Messages that can update the state
#[derive(Clone, Debug)]
enum Message {
    // Future: Add list interaction messages
}

/// Update function - handles state changes
fn update(_state: &mut TodoApp, _msg: Message) {
    // Future: Add update logic
}

/// View function - renders the UI from current state
fn view(state: &TodoApp) -> Container<Message> {
    // Create list items as labels
    let mut list_items: Vec<AnyWidget<Message>> = state
        .items
        .iter()
        .map(|item| {
            let checkbox = if item.completed { "[✓]" } else { "[ ]" };
            let style = if item.completed {
                Style::default().fg(Color::Indexed(8))
            } else {
                Style::default().fg(Color::White)
            };
            Label::new(format!("  {} {}", checkbox, item.title))
                .style(style)
                .into()
        })
        .collect();

    let mut view_items = vec![
        // Title
        Label::new("📋 Todo List Example")
            .style(Style::default().fg(Color::Cyan).bold())
            .into(),
        // Stats
        Label::new(format!(
            "Total: {} | Completed: {} | Remaining: {}",
            state.items.len(),
            state.items.iter().filter(|i| i.completed).count(),
            state.items.iter().filter(|i| !i.completed).count()
        ))
        .style(Style::default().fg(Color::Yellow))
        .into(),
        // Separator
        Label::new("─────────────────────────────")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    ];

    view_items.append(&mut list_items);

    view_items.push(
        // Instructions
        Label::new("Note: Full interactive list widget coming in Phase 7")
            .style(Style::default().fg(Color::Indexed(8)))
            .into()
    );

    Container::vertical(view_items)
        .gap(0)
        .padding(Padding::new(2, 2, 1, 1))
}

fn main() -> Result<()> {
    let app = App::new(TodoApp {
        items: vec![
            TodoItem {
                title: "Learn TUI framework".to_string(),
                completed: true,
            },
            TodoItem {
                title: "Build awesome apps".to_string(),
                completed: false,
            },
            TodoItem {
                title: "Share with community".to_string(),
                completed: false,
            },
        ],
    });
    app.run(update, view)?;
    Ok(())
}

