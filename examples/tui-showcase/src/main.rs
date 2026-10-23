//! A component gallery built entirely with the public rsille API.
mod pages;
mod verify;

use std::time::Duration;

use rsille::tui::prelude::*;

const PAGES: [(&str, &str); 7] = [
    ("overview", "Overview"),
    ("controls", "Controls"),
    ("collections", "Data"),
    ("content", "Content"),
    ("layout", "Layout"),
    ("motion", "Motion"),
    ("themes", "Themes"),
];
const THEMES: [&str; 5] = ["Dark", "Light", "One Dark", "Dracula", "Tokyo Night"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Modal {
    Commands,
    Confirm,
}

enum Msg {
    Page(String),
    Theme,
    Tick,
    Pause(bool),
    Name(String),
    Notes(String),
    Save,
    Saved(u64, String),
    Remember(bool),
    Notifications(bool),
    Compact(bool),
    Density(String),
    Priority(String),
    Date(CalendarDate),
    DateSubmitted(CalendarDate),
    Count(i32),
    Filter(String),
    Sort,
    Owner(bool),
    Inspect(String),
    Selected(Vec<String>),
    Expanded(bool),
    Stretch,
    Open(Modal),
    Close,
    Confirm,
    Command(String),
    Escape,
    Quit,
}

struct State {
    page: String,
    theme: usize,
    ticks: usize,
    playing: bool,
    name: String,
    notes: String,
    revision: u64,
    saved: String,
    save_status: String,
    remember: bool,
    notifications: bool,
    compact: bool,
    density: String,
    priority: String,
    date: CalendarDate,
    count: i32,
    filter: String,
    sort_desc: bool,
    show_owner: bool,
    inspected: String,
    selected: Vec<String>,
    expanded: bool,
    stretched: bool,
    modal: Option<Modal>,
    logs: Vec<LogLine>,
    status: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            page: "overview".into(),
            theme: 4,
            ticks: 0,
            playing: true,
            name: "Terminal explorer".into(),
            notes: "Hello, rsille!\n中文 / e\u{301} / 👩🏽‍💻\nEdit me: changes are saved in memory."
                .into(),
            revision: 0,
            saved: String::new(),
            save_status: "Edit the notes to start autosave".into(),
            remember: true,
            notifications: true,
            compact: false,
            density: "comfortable".into(),
            priority: "Not selected".into(),
            date: CalendarDate::new(2026, 10, 23).expect("valid demo date"),
            count: 0,
            filter: String::new(),
            sort_desc: false,
            show_owner: true,
            inspected: "Use arrows to inspect an item".into(),
            selected: Vec::new(),
            expanded: true,
            stretched: false,
            modal: None,
            logs: vec![
                LogLine::new(LogLevel::Trace, "TRACE layout initialized"),
                LogLine::new(LogLevel::Debug, "DEBUG deterministic sample data loaded"),
                LogLine::new(LogLevel::Info, "INFO welcome to the component gallery"),
                LogLine::new(LogLevel::Warn, "WARN sample warning for style preview"),
                LogLine::new(LogLevel::Error, "ERROR sample error for style preview"),
            ],
            status: "F1: commands  F2: theme  F3: dialog  Ctrl+C: exit".into(),
        }
    }
}

fn theme(index: usize) -> Theme {
    match index % THEMES.len() {
        0 => Theme::dark(),
        1 => Theme::light(),
        2 => Theme::one_dark(),
        3 => Theme::dracula(),
        _ => Theme::tokyo_night(),
    }
}

fn app() -> App<State, Msg> {
    App::new(State::default())
        .with_theme_from(|state| theme(state.theme))
        .disable_quit_key()
        .on_key(KeyCode::F(1), || Msg::Open(Modal::Commands))
        .on_key(KeyCode::F(2), || Msg::Theme)
        .on_key(KeyCode::F(3), || Msg::Open(Modal::Confirm))
        .on_key(KeyCode::Esc, || Msg::Escape)
        .on_tick(Duration::from_millis(80), || Msg::Tick)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("TUI component showcase\n\n  --inline    use the normal terminal screen\n  --headless  verify the gallery without a terminal\n\nF1 commands / F2 theme / F3 dialog / Tab focus / Esc close or exit");
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--headless") {
        verify::run()?;
        println!("Showcase verified: 7 pages, 5 themes, Unicode editing, autosave, modal focus and resize.");
        return Ok(());
    }
    let mode = if args.iter().any(|arg| arg == "--inline") {
        Mode::Inline
    } else {
        Mode::Fullscreen
    };
    app().mode(mode).update(update).view(view).run()?;
    Ok(())
}

fn record(state: &mut State, message: impl Into<String>) {
    state.status = message.into();
    state
        .logs
        .push(LogLine::new(LogLevel::Info, state.status.clone()));
    if state.logs.len() > 60 {
        state.logs.remove(0);
    }
}

fn update(state: &mut State, message: Msg, ctx: &mut UpdateCtx<Msg>) {
    match message {
        Msg::Page(page) => {
            if PAGES.iter().any(|(id, _)| *id == page) {
                record(state, format!("Viewing {page}"));
                state.page = page;
            }
        }
        Msg::Theme => {
            state.theme = (state.theme + 1) % THEMES.len();
            record(state, format!("Theme: {}", THEMES[state.theme]));
        }
        Msg::Tick => {
            if state.playing {
                state.ticks = state.ticks.wrapping_add(1);
            }
        }
        Msg::Pause(value) => state.playing = value,
        Msg::Name(value) => state.name = value,
        Msg::Notes(value) => {
            state.notes = value;
            state.revision += 1;
            state.save_status = "Waiting for 350 ms of idle time...".into();
            ctx.debounce("notes-save", Duration::from_millis(350), Msg::Save);
        }
        Msg::Save => {
            let notes = state.notes.clone();
            let revision = state.revision;
            state.save_status = "Saving on a background worker...".into();
            ctx.spawn(
                Task::new(move |task| {
                    task.emit(Msg::Saved(revision, notes.clone()));
                    TaskOutcome::Complete
                })
                .key("notes-worker"),
            );
        }
        Msg::Saved(revision, value) => {
            if revision == state.revision {
                state.saved = value;
                state.save_status = format!("Saved revision {revision} in memory");
                record(state, state.save_status.clone());
            }
        }
        Msg::Remember(value) => state.remember = value,
        Msg::Notifications(value) => state.notifications = value,
        Msg::Compact(value) => state.compact = value,
        Msg::Density(value) => state.density = value,
        Msg::Priority(value) => {
            state.priority = value;
            record(state, format!("Priority: {}", state.priority));
        }
        Msg::Date(value) => state.date = value,
        Msg::DateSubmitted(value) => record(
            state,
            format!(
                "Date: {:04}-{:02}-{:02}",
                value.year, value.month, value.day
            ),
        ),
        Msg::Count(delta) => {
            state.count += delta;
            record(state, format!("Button counter: {}", state.count));
        }
        Msg::Filter(value) => state.filter = value,
        Msg::Sort => state.sort_desc = !state.sort_desc,
        Msg::Owner(value) => state.show_owner = value,
        Msg::Inspect(value) => {
            state.inspected = value;
            record(state, format!("Inspect: {}", state.inspected));
        }
        Msg::Selected(values) => {
            state.selected = values;
            record(
                state,
                format!("Selected {} table rows", state.selected.len()),
            );
        }
        Msg::Expanded(value) => state.expanded = value,
        Msg::Stretch => state.stretched = !state.stretched,
        Msg::Open(modal) => state.modal = Some(modal),
        Msg::Close => state.modal = None,
        Msg::Confirm => {
            state.modal = None;
            record(state, "Dialog confirmed; focus returned to the gallery");
        }
        Msg::Command(id) => {
            state.modal = None;
            match id.as_str() {
                "theme" => update(state, Msg::Theme, ctx),
                "pause" => state.playing = !state.playing,
                "dialog" => state.modal = Some(Modal::Confirm),
                "quit" => ctx.exit(),
                _ => update(state, Msg::Page(id), ctx),
            }
        }
        Msg::Escape => {
            if state.modal.take().is_none() {
                ctx.exit();
            }
        }
        Msg::Quit => ctx.exit(),
    }
}

fn view(state: &State) -> impl Widget<Msg> {
    let styles = theme(state.theme).styles;
    let content = pages::view(state);
    let base = grid()
        .columns("1fr")
        .rows("3 3 1fr 1")
        .padding(Padding::uniform(1))
        .style(styles.surface)
        .child_at(
            row()
                .gap(1)
                .child(label("RSILLE / TUI GALLERY").bold())
                .child(spacer())
                .child(
                    button(format!("Theme: {}", THEMES[state.theme]))
                        .key("theme")
                        .variant(ButtonVariant::Secondary)
                        .on_click(|| Msg::Theme),
                )
                .child(
                    button("Commands [F1]")
                        .key("commands")
                        .on_click(|| Msg::Open(Modal::Commands)),
                )
                .child(
                    button("Exit")
                        .key("exit")
                        .variant(ButtonVariant::Ghost)
                        .on_click(|| Msg::Quit),
                ),
            GridPlacement::new().area(1, 1),
        )
        .child_at(
            tabs()
                .key("navigation")
                .tabs(PAGES.map(|(id, title)| TabItem::new(id, title)))
                .selected(&state.page)
                .on_change(Msg::Page),
            GridPlacement::new().area(1, 2),
        )
        .child_at(
            scroll_view(content)
                .key(format!("page-{}", state.page))
                .both()
                .content_width(100)
                .content_height(35)
                .scrollbars(ScrollbarVisibility::Auto),
            GridPlacement::new().area(1, 3),
        )
        .child_at(
            label(&state.status).style(styles.text_muted),
            GridPlacement::new().area(1, 4),
        );

    // Only the modal participates in input while open. Controlled values stay
    // in State; closing it returns to the gallery's first focusable control.
    let base: Box<dyn Widget<Msg>> = if state.modal.is_some() {
        Box::new(Backdrop(Box::new(base)))
    } else {
        Box::new(base)
    };
    let mut root = overlay(base);
    if let Some(modal) = state.modal {
        let (widget, height): (Box<dyn Widget<Msg>>, u16) = match modal {
            Modal::Commands => (
                Box::new(
                    command_palette()
                        .key("palette")
                        .height(16)
                        .title("Explore the showcase")
                        .placeholder("Type a page or action...")
                        .search_mode(SelectSearchMode::Fuzzy)
                        .items(
                            PAGES
                                .map(|(id, title)| CommandItem::new(id, format!("Page: {title}")))
                                .into_iter()
                                .chain([
                                    CommandItem::new("theme", "Cycle theme").keyword("color"),
                                    CommandItem::new("pause", "Pause / resume animation"),
                                    CommandItem::new("dialog", "Open confirmation dialog"),
                                    CommandItem::new("quit", "Exit showcase"),
                                ]),
                        )
                        .on_submit(Msg::Command)
                        .on_close(|| Msg::Close),
                ),
                16,
            ),
            Modal::Confirm => (
                Box::new(
                    dialog()
                        .key("confirmation")
                        .title("A focus-trapped dialog")
                        .padding(Padding::uniform(1))
                        .gap(1)
                        .child(label("Tab cycles between the two actions."))
                        .child(label("Esc closes this dialog. Try F2 to change its theme."))
                        .child(
                            row()
                                .gap(2)
                                .child(
                                    button("Cancel")
                                        .key("cancel")
                                        .variant(ButtonVariant::Secondary)
                                        .on_click(|| Msg::Close),
                                )
                                .child(button("Confirm").key("confirm").on_click(|| Msg::Confirm)),
                        ),
                ),
                11,
            ),
        };
        root = root
            .layer(
                OverlayLayer::new(widget)
                    .floating(OverlayAnchor::Center)
                    .size(60, height),
            )
            .trap_focus();
    }
    GalleryViewport([Box::new(root)])
}

// Inline mode measures the root before reserving terminal rows. Give the
// gallery its natural height; fullscreen still lays out to the actual window.
struct GalleryViewport([Box<dyn Widget<Msg>>; 1]);

impl Widget<Msg> for GalleryViewport {
    fn constraints(&self) -> Constraints {
        Constraints::min(1, 44)
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.0
    }

    fn render(&self, chunk: &mut rsille::render::chunk::Chunk, ctx: &RenderCtx) {
        ctx.render_children(chunk, self);
    }

    fn handle_event(&self, _event: &Event, _ctx: &mut EventCtx<Msg>) {}
}

// A paint-only preview: the background's controls are deliberately absent from
// the input tree while a modal is open. Application-controlled values survive.
struct Backdrop(Box<dyn Widget<Msg>>);

impl Widget<Msg> for Backdrop {
    fn constraints(&self) -> Constraints {
        self.0.constraints()
    }

    fn render(&self, chunk: &mut rsille::render::chunk::Chunk, ctx: &RenderCtx) {
        self.0.render(chunk, &ctx.child_ctx("preview"));
    }

    fn handle_event(&self, _event: &Event, _ctx: &mut EventCtx<Msg>) {}
}
