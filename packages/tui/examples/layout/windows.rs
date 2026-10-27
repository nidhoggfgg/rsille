//! Floating windows and an independent moving drag handle, using the public API.
//! `cargo run -p tui --example windows` (add `-- --inline` for inline mode).
use tui::prelude::*;

#[derive(Default)]
struct State {
    note: String,
    visibility: WindowVisibility,
    closed: bool,
    origin: CellPoint,
    offset: CellPoint,
}
#[derive(Debug)]
enum Msg {
    Note(String),
    Visibility(WindowVisibility),
    Close,
    Restore,
    Drag(DragEvent),
}
fn main() -> WidgetResult<()> {
    let mode = if std::env::args().any(|a| a == "--inline") {
        Mode::Inline
    } else {
        Mode::Fullscreen
    };
    App::new(State::default())
        .mode(mode)
        .focus_events(true)
        .update(update)
        .view(view)
        .run()
        .map(|_| ())
}
fn update(s: &mut State, message: Msg, _: &mut UpdateCtx<Msg>) {
    match message {
        Msg::Note(value) => s.note = value,
        Msg::Visibility(value) => s.visibility = value,
        Msg::Close => s.closed = true,
        Msg::Restore => {
            s.closed = false;
            s.visibility = WindowVisibility::Expanded;
        }
        Msg::Drag(DragEvent::Begin(_)) => s.origin = s.offset,
        Msg::Drag(DragEvent::Update(update) | DragEvent::End(update)) => {
            s.offset = CellPoint::new(
                s.origin.x.saturating_add(update.total_delta.x),
                s.origin.y.saturating_add(update.total_delta.y),
            );
        }
        Msg::Drag(DragEvent::Cancel(DragCancelReason::Escape)) => s.offset = s.origin,
        Msg::Drag(DragEvent::Cancel(_)) => {}
    }
}
fn view(s: &State) -> impl Widget<Msg> {
    let tools = col()
        .padding(Padding::uniform(1))
        .gap(1)
        .child(label(
            "Drag a title or corner. Title: arrows move, Shift+arrows resize, Enter folds.",
        ))
        .child(button("Restore notes").on_click(|| Msg::Restore));
    let handle = drag_region(label(format!(
        "Move this content: ({}, {})",
        s.offset.x, s.offset.y
    )))
    .key("ordinary-drag")
    .on_drag(Msg::Drag);
    let offset = |base: i32, value: i32| {
        base.saturating_add(value)
            .clamp(i16::MIN.into(), i16::MAX.into()) as i16
    };
    let workspace = overlay(tools).layer(
        OverlayLayer::new(handle)
            .floating(OverlayAnchor::TopLeft)
            .offset(offset(2, s.offset.x), offset(5, s.offset.y))
            .size(36, 2),
    );
    let mut host = window_host(workspace).key("windows").snap_distance(2);
    if !s.closed {
        host = host.window(
            floating_window(
                textarea()
                    .key("note-editor")
                    .value(&s.note)
                    .on_change(Msg::Note),
            )
            .key("notes")
            .title("Notes")
            .default_rect(WindowRect::new(4, 8, 42, 14))
            .visibility(s.visibility)
            .on_visibility_change(Msg::Visibility)
            .hideable(true)
            .on_close(|| Msg::Close),
        );
    }
    host.window(
        floating_window(label(
            "Any widget can live in a window.\nClosing notes unmounts its editor; hiding retains it.",
        ))
        .key("help")
        .title("Help")
        .default_rect(WindowRect::new(48, 5, 34, 10)),
    )
}
