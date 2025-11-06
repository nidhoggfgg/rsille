//! Layout Demo Example
//!
//! Demonstrates different layout options: vertical, horizontal,
//! gaps, padding, and nested containers.
//!
//! Run with: cargo run --example layout_demo

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct LayoutState {
    selected_layout: usize,
    gap_size: u16,
}

/// Messages that can update the state
#[derive(Clone, Debug)]
enum Message {
    NextLayout,
    PrevLayout,
    IncreaseGap,
    DecreaseGap,
}

/// Update function - handles state changes
fn update(state: &mut LayoutState, msg: Message) {
    match msg {
        Message::NextLayout => {
            state.selected_layout = (state.selected_layout + 1) % 4;
        }
        Message::PrevLayout => {
            if state.selected_layout == 0 {
                state.selected_layout = 3;
            } else {
                state.selected_layout -= 1;
            }
        }
        Message::IncreaseGap => {
            state.gap_size = (state.gap_size + 1).min(10);
        }
        Message::DecreaseGap => {
            state.gap_size = state.gap_size.saturating_sub(1);
        }
    }
}

/// Create a colored box widget
fn create_box(label: &str, color: Color) -> AnyWidget<Message> {
    Label::new(format!("[ {} ]", label))
        .style(Style::default().fg(color).bold())
        .into()
}

/// View function - renders the UI from current state
fn view(state: &LayoutState) -> Container<Message> {
    let layout_names = [
        "Vertical Layout",
        "Horizontal Layout",
        "Nested Layout",
        "Grid-like Layout",
    ];

    let demo_content = match state.selected_layout {
        0 => {
            // Vertical layout
            Container::vertical(vec![
                create_box("Box 1", Color::Red),
                create_box("Box 2", Color::Green),
                create_box("Box 3", Color::Blue),
                create_box("Box 4", Color::Yellow),
            ])
            .gap(state.gap_size)
            .padding(Padding::new(2, 2, 1, 1))
        }
        1 => {
            // Horizontal layout
            Container::horizontal(vec![
                create_box("Box 1", Color::Red),
                create_box("Box 2", Color::Green),
                create_box("Box 3", Color::Blue),
                create_box("Box 4", Color::Yellow),
            ])
            .gap(state.gap_size)
            .padding(Padding::new(2, 2, 1, 1))
        }
        2 => {
            // Nested layout
            Container::vertical(vec![
                create_box("Header", Color::Cyan),
                Container::horizontal(vec![
                    create_box("Left Sidebar", Color::Magenta),
                    Container::vertical(vec![
                        create_box("Content Top", Color::Green),
                        create_box("Content Bottom", Color::Blue),
                    ])
                    .gap(state.gap_size)
                    .into(),
                    create_box("Right Sidebar", Color::Yellow),
                ])
                .gap(state.gap_size)
                .into(),
                create_box("Footer", Color::Red),
            ])
            .gap(state.gap_size)
            .padding(Padding::new(2, 2, 1, 1))
        }
        3 => {
            // Grid-like layout
            Container::vertical(vec![
                Container::horizontal(vec![
                    create_box("1", Color::Red),
                    create_box("2", Color::Green),
                    create_box("3", Color::Blue),
                ])
                .gap(state.gap_size)
                .into(),
                Container::horizontal(vec![
                    create_box("4", Color::Yellow),
                    create_box("5", Color::Magenta),
                    create_box("6", Color::Cyan),
                ])
                .gap(state.gap_size)
                .into(),
                Container::horizontal(vec![
                    create_box("7", Color::White),
                    create_box("8", Color::Indexed(8)),
                    create_box("9", Color::Red),
                ])
                .gap(state.gap_size)
                .into(),
            ])
            .gap(state.gap_size)
            .padding(Padding::new(2, 2, 1, 1))
        }
        _ => Container::vertical(vec![]).padding(Padding::new(2, 2, 1, 1)),
    };

    Container::vertical(vec![
        // Title
        Label::new("📐 Layout Demo")
            .style(Style::default().fg(Color::Cyan).bold())
            .into(),
        Label::new("")
            .into(),
        // Current layout info
        Label::new(format!(
            "Layout: {} ({}/ {})",
            layout_names[state.selected_layout],
            state.selected_layout + 1,
            layout_names.len()
        ))
        .style(Style::default().fg(Color::Yellow))
        .into(),
        Label::new(format!("Gap size: {}", state.gap_size))
            .style(Style::default().fg(Color::Yellow))
            .into(),
        Label::new("")
            .into(),
        // Demo container
        demo_content.into(),
        Label::new("")
            .into(),
        // Keyboard controls
        KeyboardController::new()
            .on_key(KeyCode::Right, || Message::NextLayout)
            .on_key(KeyCode::Left, || Message::PrevLayout)
            .on_up(|| Message::IncreaseGap)
            .on_down(|| Message::DecreaseGap)
            .into(),
        // Instructions
        Label::new("Controls:")
            .style(Style::default().fg(Color::Yellow))
            .into(),
        Label::new("  Left/Right arrows: switch layout | Up/Down arrows: adjust gap")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
        Label::new("  Esc/q: quit")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    ])
    .gap(0)
    .padding(Padding::new(2, 2, 1, 1))
}

fn main() -> Result<()> {
    let app = App::new(LayoutState {
        selected_layout: 0,
        gap_size: 1,
    });
    app.run(update, view)?;
    Ok(())
}
