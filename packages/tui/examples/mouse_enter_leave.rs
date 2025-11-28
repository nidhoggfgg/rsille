//! Mouse Enter/Leave Events Demo
//!
//! Demonstrates the new on_mouse_enter and on_mouse_leave events.
//! These events fire only once when crossing the widget boundary.
//!
//! Features:
//! - on_mouse_enter: Fires when mouse enters widget area
//! - on_mouse_leave: Fires when mouse exits widget area
//! - State transition based: only fires on boundary crossing
//!
//! Controls:
//! - Move mouse over widgets to trigger enter/leave events
//! - q/Esc: Quit

use tui::prelude::*;

#[derive(Debug)]
struct State {
    widget1_status: String,
    widget2_status: String,
    widget3_status: String,
    enter_count: usize,
    leave_count: usize,
}

#[derive(Clone, Debug)]
enum Message {
    Widget1Enter,
    Widget1Leave,
    Widget2Enter,
    Widget2Leave,
    Widget3Enter,
    Widget3Leave,
    Quit,
}

fn update(state: &mut State, msg: Message) {
    match msg {
        Message::Widget1Enter => {
            state.widget1_status = "Mouse Inside".to_string();
            state.enter_count += 1;
        }
        Message::Widget1Leave => {
            state.widget1_status = "Mouse Outside".to_string();
            state.leave_count += 1;
        }
        Message::Widget2Enter => {
            state.widget2_status = "Hovering!".to_string();
            state.enter_count += 1;
        }
        Message::Widget2Leave => {
            state.widget2_status = "Not hovering".to_string();
            state.leave_count += 1;
        }
        Message::Widget3Enter => {
            state.widget3_status = "🎯 Entered".to_string();
            state.enter_count += 1;
        }
        Message::Widget3Leave => {
            state.widget3_status = "💤 Left".to_string();
            state.leave_count += 1;
        }
        Message::Quit => {}
    }
}

fn view(state: &State) -> impl Layout<Message> {
    col()
        .padding(Padding::new(2, 2, 1, 1))
        .gap(2)
        .child(
            label("Mouse Enter/Leave Events Demo")
                .bold()
                .fg(Color::Cyan),
        )
        .child(label("Move your mouse over the boxes below to see enter/leave events"))
        .child(
            row()
                .gap(3)
                .child(
                    enhanced(
                        col()
                            .child(label("Widget 1"))
                            .child(label(&state.widget1_status)),
                    )
                    .hoverable()
                    .bordered(BorderStyle::Single)
                    .padding(Padding::uniform(2))
                    .hover_bg(Color::Blue)
                    .on_mouse_enter(|| Message::Widget1Enter)
                    .on_mouse_leave(|| Message::Widget1Leave),
                )
                .child(
                    enhanced(
                        col()
                            .child(label("Widget 2"))
                            .child(label(&state.widget2_status)),
                    )
                    .hoverable()
                    .bordered(BorderStyle::Rounded)
                    .padding(Padding::uniform(2))
                    .hover_fg(Color::Yellow)
                    .on_mouse_enter(|| Message::Widget2Enter)
                    .on_mouse_leave(|| Message::Widget2Leave),
                )
                .child(
                    enhanced(
                        col()
                            .child(label("Widget 3"))
                            .child(label(&state.widget3_status)),
                    )
                    .hoverable()
                    .bordered(BorderStyle::Double)
                    .padding(Padding::uniform(2))
                    .hover_bg(Color::Green)
                    .on_mouse_enter(|| Message::Widget3Enter)
                    .on_mouse_leave(|| Message::Widget3Leave),
                ),
        )
        .child(spacer().height(1))
        .child(label("=== Event Counters ===").bold())
        .child(label(&format!("Total Enter Events: {}", state.enter_count)))
        .child(label(&format!("Total Leave Events: {}", state.leave_count)))
        .child(spacer().height(1))
        .child(label("Press 'q' or Esc to quit").fg(Color::Indexed(8)))
}

fn main() -> Result<()> {
    let app = App::new(State {
        widget1_status: "Mouse Outside".to_string(),
        widget2_status: "Not hovering".to_string(),
        widget3_status: "💤 Left".to_string(),
        enter_count: 0,
        leave_count: 0,
    });

    app.on_key(KeyCode::Char('q'), || Message::Quit)
        .on_key(KeyCode::Esc, || Message::Quit)
        .run(update, view)?;

    Ok(())
}
