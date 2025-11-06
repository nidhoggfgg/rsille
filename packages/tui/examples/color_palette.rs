//! Color Palette Example
//!
//! Demonstrates all available colors and text styles
//! (bold, italic, underline, etc.) in the TUI framework.
//!
//! Run with: cargo run --example color_palette

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct ColorState {
    selected_section: usize,
}

/// Messages that can update the state
#[derive(Clone, Debug)]
enum Message {
    NextSection,
    PrevSection,
}

/// Update function - handles state changes
fn update(state: &mut ColorState, msg: Message) {
    match msg {
        Message::NextSection => {
            state.selected_section = (state.selected_section + 1) % 3;
        }
        Message::PrevSection => {
            if state.selected_section == 0 {
                state.selected_section = 2;
            } else {
                state.selected_section -= 1;
            }
        }
    }
}

/// Create basic colors section
fn basic_colors_section() -> Vec<AnyWidget<Message>> {
    vec![
        Label::new("Basic Colors:")
            .style(Style::default().fg(Color::Yellow).bold())
            .into(),
        Label::new("").into(),
        Label::new("  Black   ")
            .style(Style::default().fg(Color::Black))
            .into(),
        Label::new("  Red     ")
            .style(Style::default().fg(Color::Red))
            .into(),
        Label::new("  Green   ")
            .style(Style::default().fg(Color::Green))
            .into(),
        Label::new("  Yellow  ")
            .style(Style::default().fg(Color::Yellow))
            .into(),
        Label::new("  Blue    ")
            .style(Style::default().fg(Color::Blue))
            .into(),
        Label::new("  Magenta ")
            .style(Style::default().fg(Color::Magenta))
            .into(),
        Label::new("  Cyan    ")
            .style(Style::default().fg(Color::Cyan))
            .into(),
        Label::new("  White   ")
            .style(Style::default().fg(Color::White))
            .into(),
    ]
}

/// Create indexed colors section
fn indexed_colors_section() -> Vec<AnyWidget<Message>> {
    let mut widgets = vec![
        Label::new("Indexed Colors (0-15):")
            .style(Style::default().fg(Color::Yellow).bold())
            .into(),
        Label::new("").into(),
    ];

    for i in 0..16 {
        widgets.push(
            Label::new(format!("  Color {:2}: ████████", i))
                .style(Style::default().fg(Color::Indexed(i)))
                .into(),
        );
    }

    widgets
}

/// Create text styles section
fn text_styles_section() -> Vec<AnyWidget<Message>> {
    vec![
        Label::new("Text Styles:")
            .style(Style::default().fg(Color::Yellow).bold())
            .into(),
        Label::new("").into(),
        Label::new("  Normal text")
            .style(Style::default().fg(Color::White))
            .into(),
        Label::new("  Bold text")
            .style(Style::default().fg(Color::White).bold())
            .into(),
        Label::new("  Italic text")
            .style(Style::default().fg(Color::White).italic())
            .into(),
        Label::new("  Underlined text")
            .style(Style::default().fg(Color::White).underlined())
            .into(),
        Label::new("").into(),
        Label::new("Combined Styles:")
            .style(Style::default().fg(Color::Yellow).bold())
            .into(),
        Label::new("").into(),
        Label::new("  Bold + Italic + Cyan")
            .style(Style::default().fg(Color::Cyan).bold().italic())
            .into(),
        Label::new("  Bold + Underlined + Green")
            .style(Style::default().fg(Color::Green).bold().underlined())
            .into(),
        Label::new("  Italic + Underlined + Magenta")
            .style(Style::default().fg(Color::Magenta).italic().underlined())
            .into(),
    ]
}

/// View function - renders the UI from current state
fn view(state: &ColorState) -> Container<Message> {
    let section_names = ["Basic Colors", "Indexed Colors", "Text Styles"];

    let mut content_widgets = vec![
        // Title
        Label::new("🎨 Color Palette & Styles")
            .style(Style::default().fg(Color::Cyan).bold())
            .into(),
        Label::new("").into(),
        Label::new(format!(
            "Section: {} ({}/{})",
            section_names[state.selected_section],
            state.selected_section + 1,
            section_names.len()
        ))
        .style(Style::default().fg(Color::Yellow))
        .into(),
        Label::new("").into(),
    ];

    // Add section-specific content
    let section_content = match state.selected_section {
        0 => basic_colors_section(),
        1 => indexed_colors_section(),
        2 => text_styles_section(),
        _ => vec![],
    };

    content_widgets.extend(section_content);

    content_widgets.extend(vec![
        Label::new("").into(),
        // Keyboard controls
        KeyboardController::new()
            .on_key(KeyCode::Right, || Message::NextSection)
            .on_key(KeyCode::Left, || Message::PrevSection)
            .into(),
        // Instructions
        Label::new("Controls:")
            .style(Style::default().fg(Color::Yellow))
            .into(),
        Label::new("  Left/Right arrows: switch section | Esc/q: quit")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    ]);

    Container::vertical(content_widgets)
        .gap(0)
        .padding(Padding::new(2, 2, 1, 1))
}

fn main() -> Result<()> {
    let app = App::new(ColorState {
        selected_section: 0,
    });
    app.run(update, view)?;
    Ok(())
}
