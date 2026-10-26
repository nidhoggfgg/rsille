//! Virtual logs, anchored modal palette and interactive world canvas.
//! cargo run -p tui --example extensions [-- --headless]
use std::{collections::HashSet, sync::Arc};
use tui::prelude::*;

#[derive(Clone, Default)]
struct Logs {
    recovered: HashSet<usize>,
}
impl DataSource for Logs {
    type Item = String;
    fn len(&self) -> usize {
        100_000
    }
    fn key(&self, index: usize) -> String {
        index.to_string()
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        key.parse().ok().filter(|&i| i < self.len())
    }
    fn get(&self, index: usize) -> RowData<String> {
        if index == 3 && !self.recovered.contains(&index) {
            RowData::Failed("page unavailable".into())
        } else {
            RowData::Ready(format!("log {index:06}: generated on demand"))
        }
    }
}
struct State {
    logs: Arc<Logs>,
    list: VirtualListState,
    palette: bool,
    viewport: CanvasViewport,
    drag: Option<CanvasPoint>,
    hit: Option<CanvasPoint>,
}
enum Msg {
    Window(VirtualListState),
    Palette,
    Close,
    Reset,
    Retry(usize),
    Command(String),
    Pointer(CanvasPointer),
}
fn commands(state: &State) -> CommandRouter<Msg> {
    let mut commands = CommandRouter::new();
    commands
        .register("reset", "Reset selection and viewport", || Msg::Reset)
        .enabled(state.list.selected.is_some())
        .hotkey(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
    commands
        .register("palette", "Open command palette", || Msg::Palette)
        .hotkey(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    commands
}
fn update(state: &mut State, msg: Msg, ctx: &mut UpdateCtx<Msg>) {
    match msg {
        Msg::Window(window) => state.list = window,
        Msg::Palette => state.palette = true,
        Msg::Close => state.palette = false,
        Msg::Reset => {
            state.list = VirtualListState::default();
            state.viewport = CanvasViewport::new(-10.0, -5.0, 20.0, 10.0);
            state.palette = false;
        }
        Msg::Retry(index) => {
            Arc::make_mut(&mut state.logs).recovered.insert(index);
        }
        Msg::Command(id) => {
            if let Some(msg) = commands(state).dispatch(&id) {
                update(state, msg, ctx);
            }
        }
        Msg::Pointer(pointer) => match pointer.kind {
            MouseEventKind::ScrollUp => {
                state.viewport = state.viewport.zoom_at(1.2, pointer.position)
            }
            MouseEventKind::ScrollDown => {
                state.viewport = state.viewport.zoom_at(1.0 / 1.2, pointer.position)
            }
            MouseEventKind::Down(MouseButton::Left) => {
                state.drag = Some(pointer.position);
                state.hit = Some(pointer.position);
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(previous) = state.drag {
                    state.viewport = state.viewport.pan(
                        previous.x - pointer.position.x,
                        previous.y - pointer.position.y,
                    );
                }
            }
            MouseEventKind::Up(_) => state.drag = None,
            _ => {}
        },
    }
}
fn view(state: &State) -> impl Widget<Msg> {
    let mut root = portal(
        col()
            .gap(1)
            .child(
                row()
                    .child(
                        button("Commands Ctrl+P")
                            .key("commands")
                            .on_click(|| Msg::Palette),
                    )
                    .child(label("100,000 logs · arrows / PgUp / PgDown / End")),
            )
            .child(
                VirtualList::new(
                    state.logs.clone(),
                    &state.list,
                    VirtualListConfig {
                        height: 8,
                        row_height: 1,
                        overscan: 2,
                    },
                    |index, row, selected| -> Box<dyn Widget<Msg>> {
                        match row {
                            RowData::Ready(text) => Box::new(label(format!(
                                "{} {text}",
                                if selected { ">" } else { " " }
                            ))),
                            RowData::Loading => Box::new(label("loading…")),
                            RowData::Failed(error) => Box::new(
                                button(format!("{error}; retry"))
                                    .key("retry")
                                    .on_click(move || Msg::Retry(index)),
                            ),
                        }
                    },
                )
                .key("logs")
                .on_change(Msg::Window),
            )
            .child(label(format!(
                "Canvas: wheel to zoom, drag to pan, click to hit test. {:?}",
                state.hit
            )))
            .child(
                canvas(|surface, ctx| {
                    ctx.world(surface)
                        .line(CanvasPoint::new(-100.0, 0.0), CanvasPoint::new(100.0, 0.0))
                        .line(CanvasPoint::new(0.0, -100.0), CanvasPoint::new(0.0, 100.0))
                        .rect(CanvasViewport::new(-3.0, -2.0, 6.0, 4.0))
                        .circle(CanvasPoint::new(0.0, 0.0), 2.0);
                })
                .viewport(state.viewport)
                .height(8)
                .key("plot")
                .on_pointer(Msg::Pointer),
            ),
    );
    if state.palette {
        root = root.layer(
            PortalLayer::new(
                command_palette()
                    .key("palette")
                    .items(commands(state).palette_items(None))
                    .on_submit(Msg::Command)
                    .on_close(|| Msg::Close),
            )
            .key("commands-popup")
            .anchor(WidgetId::root().child("commands"))
            .modal(true)
            .size(42, 9)
            .on_dismiss(|| Msg::Close),
        );
    }
    root
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(State {
        logs: Arc::new(Logs::default()),
        list: VirtualListState::default(),
        palette: false,
        viewport: CanvasViewport::new(-10.0, -5.0, 20.0, 10.0),
        drag: None,
        hit: None,
    })
    .with_commands_from(commands)
    .update(update)
    .view(view)
    .build();
    if std::env::args().any(|arg| arg == "--headless") {
        app = app.size(64, 32);
        app.step(&[])?;
        // Header button → virtual list, then jump directly to the final log.
        app.step(&[Event::Key(KeyEvent::new(
            KeyCode::Tab,
            KeyModifiers::empty(),
        ))])?;
        app.step(&[Event::Key(KeyEvent::new(
            KeyCode::End,
            KeyModifiers::empty(),
        ))])?;
        assert_eq!(app.state().list.selected.as_deref(), Some("99999"));
        app.step(&[Event::Key(KeyEvent::new(
            KeyCode::Char('p'),
            KeyModifiers::CONTROL,
        ))])?;
        assert_eq!(app.focused_key(), Some("palette"));
        app.step(&[Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::empty(),
        ))])?;
        assert!(!app.state().palette);
        assert_eq!(app.state().list.selected, None);
        println!("extensions: virtual logs, commands, Portal focus and reset passed");
        Ok(())
    } else {
        app.run().map(|_| ()).map_err(Into::into)
    }
}
