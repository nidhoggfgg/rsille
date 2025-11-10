//! Smooth Transitions Example
//!
//! Demonstrates smooth state transitions in a mini UI application
//! showing how animations can enhance user experience.
//!
//! Run with: cargo run --example smooth_transitions

use std::time::Duration;
use tui::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Screen {
    Menu,
    Settings,
    About,
}

/// Application state
#[derive(Debug)]
struct AppState {
    current_screen: Screen,
    transition: Transition,
    menu_items: Vec<String>,
    selected_index: usize,
}

#[allow(unused)]
/// Messages
#[derive(Clone, Debug)]
enum Message {
    NavigateTo(Screen),
    SelectNext,
    SelectPrev,
    Tick,
}

/// Update function
fn update(state: &mut AppState, msg: Message) {
    match msg {
        Message::NavigateTo(screen) => {
            if state.current_screen != screen {
                // Fade out current screen
                state
                    .transition
                    .opacity
                    .fade_out(Duration::from_millis(150));
                state.current_screen = screen;

                // Set flag to fade in after short delay
                // Note: In a real implementation, you'd use a timer
                // For this demo, we simulate the transition directly
                std::thread::sleep(Duration::from_millis(150));
                state.transition.opacity.fade_in(Duration::from_millis(200));
            }
        }

        Message::SelectNext => {
            state.selected_index = (state.selected_index + 1) % state.menu_items.len();
        }

        Message::SelectPrev => {
            state.selected_index = if state.selected_index == 0 {
                state.menu_items.len() - 1
            } else {
                state.selected_index - 1
            };
        }

        Message::Tick => {
            // Fade in if opacity is low
            if state.transition.opacity.opacity() < 0.1 {
                state.transition.opacity.fade_in(Duration::from_millis(200));
            }
        }
    }
}

/// Render menu screen
fn render_menu(state: &AppState) -> Container<Message> {
    let mut items: Vec<AnyWidget<Message>> = vec![
        Label::new("📱 Main Menu")
            .style(Style::default().fg(Color::Cyan).bold())
            .into(),
        Label::new("").into(),
    ];

    for (i, item) in state.menu_items.iter().enumerate() {
        let style = if i == state.selected_index {
            Style::default().fg(Color::Green).bold()
        } else {
            Style::default().fg(Color::White)
        };

        let prefix = if i == state.selected_index {
            "► "
        } else {
            "  "
        };
        items.push(
            Label::new(format!("{}{}", prefix, item))
                .style(style)
                .into(),
        );
    }

    items.push(Label::new("").into());
    items.push(
        Label::new("Use ↑/↓ to navigate, Enter to select")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    );

    Container::vertical(items)
        .gap(0)
        .padding(Padding::new(2, 2, 1, 1))
}

/// Render settings screen
fn render_settings() -> Container<Message> {
    Container::vertical(vec![
        Label::new("⚙️  Settings")
            .style(Style::default().fg(Color::Yellow).bold())
            .into(),
        Label::new("").into(),
        Label::new("• Display Mode: Terminal")
            .style(Style::default().fg(Color::White))
            .into(),
        Label::new("• Animation Speed: Normal")
            .style(Style::default().fg(Color::White))
            .into(),
        Label::new("• Color Theme: Default")
            .style(Style::default().fg(Color::White))
            .into(),
        Label::new("").into(),
        Label::new("Press Esc to go back")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    ])
    .gap(0)
    .padding(Padding::new(2, 2, 1, 1))
}

/// Render about screen
fn render_about() -> Container<Message> {
    Container::vertical(vec![
        Label::new("ℹ️  About")
            .style(Style::default().fg(Color::Magenta).bold())
            .into(),
        Label::new("").into(),
        Label::new("TUI Animation Framework")
            .style(Style::default().fg(Color::White))
            .into(),
        Label::new("Version 0.1.0")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
        Label::new("").into(),
        Label::new("A modern terminal UI library with")
            .style(Style::default().fg(Color::White))
            .into(),
        Label::new("smooth animations and transitions.")
            .style(Style::default().fg(Color::White))
            .into(),
        Label::new("").into(),
        Label::new("Press Esc to go back")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    ])
    .gap(0)
    .padding(Padding::new(2, 2, 1, 1))
}

/// View function
fn view(state: &AppState) -> Container<Message> {
    // Get current opacity for fade effect
    let opacity = state.transition.opacity.opacity();

    // Choose color based on opacity to simulate fade
    let opacity_level = (opacity * 255.0) as u8;
    let dimmed = opacity > 0.5;

    let screen_content = match state.current_screen {
        Screen::Menu => render_menu(state),
        Screen::Settings => render_settings(),
        Screen::About => render_about(),
    };

    // Add keyboard controller based on screen
    let keyboard = match state.current_screen {
        Screen::Menu => KeyboardController::new()
            .on_key(KeyCode::Up, || Message::SelectPrev)
            .on_key(KeyCode::Down, || Message::SelectNext)
            .on_key(KeyCode::Enter, || {
                // Navigate based on selection - hardcoded for demo
                Message::NavigateTo(Screen::Settings)
            })
            .on_key(KeyCode::Char('1'), || Message::NavigateTo(Screen::Menu))
            .on_key(KeyCode::Char('2'), || Message::NavigateTo(Screen::Settings))
            .on_key(KeyCode::Char('3'), || Message::NavigateTo(Screen::About)),

        Screen::Settings | Screen::About => KeyboardController::new()
            .on_key(KeyCode::Esc, || Message::NavigateTo(Screen::Menu))
            .on_key(KeyCode::Char('1'), || Message::NavigateTo(Screen::Menu))
            .on_key(KeyCode::Char('2'), || Message::NavigateTo(Screen::Settings))
            .on_key(KeyCode::Char('3'), || Message::NavigateTo(Screen::About)),
    };

    // Wrapper container with opacity effect simulation
    let content_widgets = if dimmed {
        vec![screen_content.into(), keyboard.into()]
    } else {
        // When fading, show dimmed version
        vec![
            Label::new(format!("Transitioning... (opacity: {:.2})", opacity))
                .style(Style::default().fg(Color::Indexed(8)))
                .into(),
            keyboard.into(),
        ]
    };

    Container::vertical(vec![
        Container::vertical(content_widgets).gap(0).into(),
        Label::new("").into(),
        Label::new(format!(
            "Navigation: [1] Menu  [2] Settings  [3] About  |  Opacity: {:.0}%",
            opacity * 100.0
        ))
        .style(Style::default().fg(Color::Indexed(opacity_level)))
        .into(),
    ])
    .gap(0)
}

fn main() -> Result<()> {
    let app = App::new(AppState {
        current_screen: Screen::Menu,
        transition: Transition::new(),
        menu_items: vec![
            "Start New Game".to_string(),
            "Load Game".to_string(),
            "Settings".to_string(),
            "About".to_string(),
            "Exit".to_string(),
        ],
        selected_index: 0,
    });

    app.run(update, view)?;
    Ok(())
}
