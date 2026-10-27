//! Lumen: a standalone, stateful rsille application using only public APIs.
mod art;
mod ui;
mod verify;

use rsille::tui::prelude::*;
use std::time::Duration;

const STEP: Duration = Duration::from_millis(33);
const PAGES: [(&str, &str); 4] = [
    ("overview", "Overview"),
    ("flows", "Flows"),
    ("motion", "Motion lab"),
    ("notes", "Notebook"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Modal {
    Commands,
    NewFlow,
}

#[derive(Debug)]
enum Msg {
    Tick,
    Resize(u16, u16),
    Page(String),
    Theme,
    Quiet(bool),
    Open(Modal),
    Close,
    Escape,
    Command(String),
    Run,
    Filter(String),
    Inspect(String),
    Sort,
    Name(String),
    Create,
    Notes(String),
    Save(u64),
    Saved(u64, String),
    Effect(usize),
    Replay,
    Expand,
    Pointer(CanvasPointer),
    Notify(String),
    Dismiss(u64),
}

#[derive(Clone)]
struct Flow {
    id: String,
    name: String,
    trigger: String,
    runs: u32,
}

struct State {
    page: String,
    theme: usize,
    quiet: bool,
    width: u16,
    height: u16,
    time: f64,
    age: f64,
    effect: usize,
    effect_age: f64,
    expanded: bool,
    pointer: Option<(f64, f64)>,
    ripple_age: f64,
    modal: Option<Modal>,
    toast: Option<(u64, String)>,
    serial: u64,
    flows: Vec<Flow>,
    filter: String,
    sort_desc: bool,
    inspected: String,
    running: Option<String>,
    progress: f64,
    completed: u32,
    name: String,
    notes: String,
    revision: u64,
    saved_revision: u64,
    saved_notes: String,
    logs: Vec<LogLine>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            page: "overview".into(), theme: 0, quiet: false,
            width: 120, height: 44, time: 0.0, age: 0.0,
            effect: 0, effect_age: 0.0, expanded: false, pointer: None,
            ripple_age: 10.0, modal: None, toast: None, serial: 0,
            flows: vec![
                Flow { id: "orbit".into(), name: "Orbit / daily digest".into(), trigger: "Every morning".into(), runs: 128 },
                Flow { id: "prism".into(), name: "Prism / design sync".into(), trigger: "On demand".into(), runs: 84 },
                Flow { id: "bloom".into(), name: "Bloom / release notes".into(), trigger: "On demand".into(), runs: 56 },
                Flow { id: "tide".into(), name: "Tide / inbox sweep".into(), trigger: "Every hour".into(), runs: 213 },
            ],
            filter: String::new(), sort_desc: false, inspected: "orbit".into(),
            running: None, progress: 0.0, completed: 0, name: String::new(),
            notes: "# A little room for ideas\n\nMake useful things feel delightful.\nKeep the motion soft.\nLet the details feel intentional.\n\n- Try the orbit with your mouse\n- Run a flow, then explore\n  while it works in the background.\n- Open the command palette with F1\n\n中文、e\u{301} 和 👩🏽‍💻 都可以在这里编辑。".into(),
            revision: 0, saved_revision: 0, saved_notes: String::new(),
            logs: vec![LogLine::new(LogLevel::Info, "Workspace ready. Everything here runs locally.")],
        }
    }
}

fn app(state: State) -> App<State, Msg> {
    App::new(state)
        .with_theme_from(|s| {
            let mut theme = ui::palette(s.theme).theme();
            theme.animations.fast = AnimationSpec::new(
                Duration::from_millis(if s.quiet { 0 } else { 160 }),
                Easing::EaseOut,
            );
            theme
        })
        .focus_events(true)
        .disable_quit_key()
        .on_key(KeyCode::F(1), || Msg::Open(Modal::Commands))
        .on_key(KeyCode::F(2), || Msg::Theme)
        .on_key(KeyCode::F(3), || Msg::Command("quiet".into()))
        .on_key(KeyCode::F(4), || Msg::Command("next".into()))
        .on_key(KeyCode::Esc, || Msg::Escape)
        .on_hotkey(
            Hotkey::new(
                "commands",
                KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
                "Commands",
            ),
            || Msg::Open(Modal::Commands),
        )
        .on_input(|input| match input {
            InputEvent::Terminal(Event::Resize(w, h)) => Some(Msg::Resize(w, h)),
            _ => None,
        })
        .on_tick(STEP, || Msg::Tick)
}

fn log(state: &mut State, text: impl Into<String>) {
    state.logs.push(LogLine::new(LogLevel::Info, text));
    if state.logs.len() > 40 {
        state.logs.remove(0);
    }
}

fn notify(state: &mut State, ctx: &mut UpdateCtx<Msg>, text: impl Into<String>) {
    state.serial += 1;
    let text = text.into();
    log(state, &text);
    state.toast = Some((state.serial, text));
    ctx.after(Duration::from_secs(4), Msg::Dismiss(state.serial));
}

fn update(s: &mut State, msg: Msg, ctx: &mut UpdateCtx<Msg>) {
    match msg {
        Msg::Tick => {
            s.age += STEP.as_secs_f64();
            if !s.quiet {
                s.time += STEP.as_secs_f64();
                s.effect_age += STEP.as_secs_f64();
                s.ripple_age += STEP.as_secs_f64();
            }
            if s.running.is_some() {
                s.progress = (s.progress + STEP.as_secs_f64() / 6.0).min(1.0);
                if s.progress >= 1.0 {
                    let id = s.running.take().expect("active flow");
                    if let Some(flow) = s.flows.iter_mut().find(|f| f.id == id) {
                        flow.runs += 1;
                    }
                    s.completed += 1;
                    notify(s, ctx, "Flow complete. A little more done.");
                }
            }
        }
        Msg::Resize(w, h) => {
            s.width = w;
            s.height = h;
        }
        Msg::Page(page) => {
            if PAGES.iter().any(|(id, _)| *id == page) {
                s.page = page;
            }
        }
        Msg::Theme => s.theme = (s.theme + 1) % 3,
        Msg::Quiet(value) => s.quiet = value,
        Msg::Open(modal) => {
            s.modal = Some(modal);
        }
        Msg::Close => s.modal = None,
        Msg::Escape => {
            if s.modal.take().is_none() {
                ctx.exit();
            }
        }
        Msg::Command(command) => {
            // A modal owns keyboard actions until dismissed.
            if s.modal == Some(Modal::NewFlow) {
                return;
            }
            s.modal = None;
            match command.as_str() {
                "theme" => update(s, Msg::Theme, ctx),
                "quiet" => s.quiet = !s.quiet,
                "new" => s.modal = Some(Modal::NewFlow),
                "run" => update(s, Msg::Run, ctx),
                "next" => {
                    let index = PAGES.iter().position(|(id, _)| *id == s.page).unwrap_or(0);
                    s.page = PAGES[(index + 1) % PAGES.len()].0.into();
                }
                "quit" => ctx.exit(),
                _ => update(s, Msg::Page(command), ctx),
            }
        }
        Msg::Run => {
            if s.running.is_none() {
                let flow = s
                    .flows
                    .iter()
                    .find(|f| f.id == s.inspected)
                    .unwrap_or(&s.flows[0]);
                let name = flow.name.clone();
                s.running = Some(flow.id.clone());
                s.progress = 0.0;
                log(s, format!("Started {name}"));
            }
        }
        Msg::Filter(value) => s.filter = value,
        Msg::Inspect(id) => s.inspected = id,
        Msg::Sort => s.sort_desc = !s.sort_desc,
        Msg::Name(value) => s.name = value,
        Msg::Create => {
            let name = s.name.trim().to_string();
            if name.is_empty() {
                return;
            }
            s.serial += 1;
            let id = format!("custom-{}", s.serial);
            s.flows.push(Flow {
                id: id.clone(),
                name: name.clone(),
                trigger: "On demand".into(),
                runs: 0,
            });
            s.inspected = id;
            s.name.clear();
            s.modal = None;
            s.page = "flows".into();
            s.filter.clear();
            notify(s, ctx, format!("Created {name}"));
        }
        Msg::Notes(value) => {
            s.notes = value;
            s.revision += 1;
            ctx.debounce("notes", Duration::from_millis(450), Msg::Save(s.revision));
        }
        Msg::Save(revision) => {
            if revision == s.revision {
                let text = s.notes.clone();
                ctx.spawn(
                    Task::new(move |task| {
                        task.emit(Msg::Saved(revision, text.clone()));
                        TaskOutcome::Complete
                    })
                    .key("save-notes"),
                );
            }
        }
        Msg::Saved(revision, text) => {
            if revision == s.revision {
                s.saved_revision = revision;
                s.saved_notes = text;
            }
        }
        Msg::Effect(index) => {
            s.effect = index % 4;
            s.effect_age = 0.0;
        }
        Msg::Replay => {
            s.effect_age = 0.0;
            s.ripple_age = 0.0;
        }
        Msg::Expand => s.expanded = !s.expanded,
        Msg::Pointer(pointer) => {
            if !s.quiet {
                s.pointer = Some((pointer.position.x, pointer.position.y));
                if matches!(pointer.kind, MouseEventKind::Down(MouseButton::Left)) {
                    s.ripple_age = 0.0;
                }
            }
        }
        Msg::Notify(text) => notify(s, ctx, text),
        Msg::Dismiss(id) => {
            if s.toast.as_ref().is_some_and(|(current, _)| *current == id) {
                s.toast = None;
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Lumen · an interactive rsille workspace\n\n  --inline           render in the normal terminal\n  --reduced-motion   start with decorative motion disabled\n  --headless         verify real interactions without a TTY\n  --snapshot FILE    export a deterministic 120x44 SVG frame\n  --page PAGE        overview / flows / motion / notes\n\nF1 / Ctrl+K commands · F2 palette · F3 motion · F4 next page\nTab / Shift+Tab focus · Enter activate · Esc close / exit · Ctrl+C exit");
        return Ok(());
    }
    if args.iter().any(|a| a == "--headless") {
        verify::run()?;
        println!("Lumen verified: navigation, mouse, flows, modals, autosave, effects, themes and resize.");
        return Ok(());
    }
    let mut state = State {
        quiet: args.iter().any(|a| a == "--reduced-motion"),
        ..State::default()
    };
    if let Some(index) = args.iter().position(|a| a == "--page") {
        let page = args.get(index + 1).ok_or("--page needs a page name")?;
        if !PAGES.iter().any(|(id, _)| id == page) {
            return Err("Unknown page; use overview, flows, motion or notes".into());
        }
        state.page = page.clone();
    }
    if let Some(index) = args.iter().position(|a| a == "--snapshot") {
        return verify::snapshot(
            args.get(index + 1).ok_or("--snapshot needs a file path")?,
            state,
        );
    }
    if let Ok((width, height)) = crossterm::terminal::size() {
        state.width = width;
        state.height = height;
    }
    let mode = if args.iter().any(|a| a == "--inline") {
        Mode::Inline
    } else {
        Mode::Fullscreen
    };
    app(state).mode(mode).update(update).view(ui::view).run()?;
    Ok(())
}
