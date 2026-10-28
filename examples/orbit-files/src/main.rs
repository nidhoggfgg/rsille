//! Orbit Files — a working file manager built exclusively with rsille's public UI APIs.
mod board;
mod drag;
mod files;
mod icons;
mod ui;
mod verify;

use files::{Entry, Job, Operation, Outcome};
use rsille::tui::prelude::*;
use std::{collections::BTreeSet, path::PathBuf, time::Duration};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Modal {
    NewFolder,
    Rename,
    Address,
    Trash,
    Help,
    Quit,
    Actions,
    Search,
    Preview,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sort {
    Name,
    Size,
    Modified,
}
#[derive(Debug)]
enum Msg {
    Resize(u16, u16),
    Tick,
    Navigate(PathBuf),
    Back,
    Forward,
    Up,
    Refresh,
    Filter(String),
    Hidden,
    View,
    Sort,
    Select(Vec<PathBuf>, Option<PathBuf>),
    Open(PathBuf),
    Context(Option<PathBuf>),
    Action(String),
    Clipboard(bool),
    Paste,
    Drop {
        destination: PathBuf,
        copy: bool,
        sources: Vec<PathBuf>,
    },
    DragPreview(Option<drag::Preview>),
    Undo,
    Modal(Modal),
    Close,
    Draft(String),
    Submit,
    Progress(usize, usize),
    Finished(Outcome, Operation),
    Undone(Outcome),
    Escape,
    Quit,
}
struct State {
    home: PathBuf,
    cwd: PathBuf,
    entries: Vec<Entry>,
    selected: BTreeSet<PathBuf>,
    active: Option<PathBuf>,
    filter: String,
    hidden: bool,
    list: bool,
    sort: Sort,
    descending: bool,
    history: Vec<PathBuf>,
    history_index: usize,
    clipboard: Option<(bool, Vec<PathBuf>)>,
    undo: Vec<Vec<(PathBuf, PathBuf)>>,
    modal: Option<Modal>,
    draft: String,
    error: String,
    status: String,
    busy: Option<(usize, usize)>,
    preview: String,
    drag_preview: Option<drag::Preview>,
    width: u16,
    height: u16,
    pulse: usize,
    demo: bool,
}
impl State {
    fn new(home: PathBuf, demo: bool) -> Self {
        let mut s = Self {
            cwd: home.clone(),
            history: vec![home.clone()],
            history_index: 0,
            home,
            entries: vec![],
            selected: BTreeSet::new(),
            active: None,
            filter: String::new(),
            hidden: false,
            list: false,
            sort: Sort::Name,
            descending: false,
            clipboard: None,
            undo: vec![],
            modal: None,
            draft: String::new(),
            error: String::new(),
            status: String::new(),
            busy: None,
            preview: String::new(),
            drag_preview: None,
            width: 120,
            height: 42,
            pulse: 0,
            demo,
        };
        s.refresh();
        s
    }
    fn visible(&self) -> Vec<Entry> {
        let filter = self.filter.to_lowercase();
        let mut entries: Vec<_> = self
            .entries
            .iter()
            .filter(|e| {
                (self.hidden || !e.name.starts_with('.')) && e.name.to_lowercase().contains(&filter)
            })
            .cloned()
            .collect();
        entries.sort_by(|a, b| {
            b.dir.cmp(&a.dir).then_with(|| {
                let order = match self.sort {
                    Sort::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    Sort::Size => a.size.cmp(&b.size),
                    Sort::Modified => a.modified.cmp(&b.modified),
                }
                .then_with(|| a.path.cmp(&b.path));
                if self.descending {
                    order.reverse()
                } else {
                    order
                }
            })
        });
        entries
    }
    fn refresh(&mut self) {
        match files::read_dir(&self.cwd) {
            Ok(entries) => {
                self.entries = entries;
                self.selected
                    .retain(|p| self.entries.iter().any(|e| &e.path == p));
                if self
                    .active
                    .as_ref()
                    .is_some_and(|p| !self.entries.iter().any(|e| &e.path == p))
                {
                    self.active = None;
                }
                self.update_preview();
            }
            Err(e) => self.status = format!("Cannot read directory: {e}"),
        }
    }
    fn update_preview(&mut self) {
        self.preview = self
            .active
            .as_ref()
            .map(|p| files::preview(p))
            .unwrap_or_default();
    }
    fn navigate(&mut self, path: PathBuf, record: bool) {
        self.drag_preview = None;
        match std::fs::canonicalize(&path).and_then(|p| files::read_dir(&p).map(|e| (p, e))) {
            Ok((path, entries)) => {
                if self.demo && !path.starts_with(&self.home) {
                    self.error =
                        "Demo stays inside its workspace. Use --path DIR to browse real files."
                            .into();
                    self.status = self.error.clone();
                    return;
                }
                if record && self.cwd != path {
                    self.history.truncate(self.history_index + 1);
                    self.history.push(path.clone());
                    self.history_index = self.history.len() - 1;
                }
                self.cwd = path;
                self.entries = entries;
                self.selected.clear();
                self.active = None;
                self.filter.clear();
                self.preview.clear();
                self.modal = None;
            }
            Err(e) => {
                self.error = format!("Cannot open {}: {e}", path.display());
                self.status = self.error.clone();
            }
        }
    }
    fn launch(&mut self, job: Job, ctx: &mut UpdateCtx<Msg>) {
        if self.busy.is_some() || job.sources.is_empty() {
            return;
        }
        self.busy = Some((0, job.sources.len()));
        self.status = format!("{:?} · {} items", job.operation, job.sources.len());
        ctx.spawn(
            Task::new(move |task| {
                let outcome = job.run(|n, total| task.emit(Msg::Progress(n, total)));
                task.emit(Msg::Finished(outcome, job.operation));
                TaskOutcome::Complete
            })
            .key("file-operation"),
        );
    }
}
fn app(state: State) -> App<State, Msg> {
    App::new(state)
        .with_theme(ui::theme())
        .disable_quit_key()
        .focus_events(true)
        .on_key(KeyCode::Esc, || Msg::Escape)
        .on_key(KeyCode::F(1), || Msg::Modal(Modal::Help))
        .on_key(KeyCode::F(2), || Msg::Modal(Modal::Rename))
        .on_key(KeyCode::F(5), || Msg::Refresh)
        .on_hotkey(
            Hotkey::new(
                "quit",
                KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
                "Quit",
            ),
            || Msg::Quit,
        )
        .on_hotkey(
            Hotkey::new(
                "search",
                KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
                "Search folder",
            ),
            || Msg::Modal(Modal::Search),
        )
        .on_hotkey(
            Hotkey::new(
                "address",
                KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
                "Go to folder",
            ),
            || Msg::Modal(Modal::Address),
        )
        .on_input(|input| {
            if let InputEvent::Terminal(Event::Resize(w, h)) = input {
                Some(Msg::Resize(w, h))
            } else {
                None
            }
        })
        .on_tick(Duration::from_millis(120), || Msg::Tick)
}
fn update(s: &mut State, msg: Msg, ctx: &mut UpdateCtx<Msg>) {
    match msg {
        Msg::Tick => {
            if s.busy.is_some() {
                s.pulse = (s.pulse + 1) % 4;
            }
        }
        Msg::DragPreview(preview) => {
            s.drag_preview = if s.modal.is_none() { preview } else { None };
        }
        Msg::Resize(w, h) => {
            s.drag_preview = None;
            s.width = w;
            s.height = h;
        }
        Msg::Navigate(path) => s.navigate(path, true),
        Msg::Back => {
            if s.history_index > 0 {
                let i = s.history_index - 1;
                let p = s.history[i].clone();
                s.navigate(p.clone(), false);
                if s.cwd == p {
                    s.history_index = i;
                }
            }
        }
        Msg::Forward => {
            if s.history_index + 1 < s.history.len() {
                let i = s.history_index + 1;
                let p = s.history[i].clone();
                s.navigate(p.clone(), false);
                if s.cwd == p {
                    s.history_index = i;
                }
            }
        }
        Msg::Up => {
            if let Some(parent) = s.cwd.parent() {
                s.navigate(parent.to_owned(), true);
            }
        }
        Msg::Refresh => {
            s.drag_preview = None;
            s.refresh();
        }
        Msg::Filter(filter) => {
            s.drag_preview = None;
            s.filter = filter;
            s.selected.clear();
            s.active = None;
            s.preview.clear();
        }
        Msg::Hidden => {
            s.drag_preview = None;
            s.hidden = !s.hidden;
            s.selected.clear();
            s.active = None;
            s.preview.clear();
        }
        Msg::View => {
            s.drag_preview = None;
            s.list = !s.list;
        }
        Msg::Sort => {
            s.drag_preview = None;
            s.sort = match s.sort {
                Sort::Name => Sort::Size,
                Sort::Size => Sort::Modified,
                Sort::Modified => {
                    s.descending = !s.descending;
                    Sort::Name
                }
            };
        }
        Msg::Select(paths, active) => {
            s.selected = paths.into_iter().collect();
            if s.active != active {
                s.active = active;
            }
        }
        Msg::Context(path) => {
            s.drag_preview = None;
            if s.busy.is_some() {
                return;
            }
            if let Some(path) = path {
                if !s.selected.contains(&path) {
                    s.selected = BTreeSet::from([path.clone()]);
                }
                s.active = Some(path);
            } else {
                s.selected.clear();
                s.active = None;
            }
            s.modal = Some(Modal::Actions);
        }
        Msg::Action(action) => {
            s.modal = None;
            let msg = match action.as_str() {
                "open" => s.active.clone().map(Msg::Open),
                "copy" => Some(Msg::Clipboard(false)),
                "cut" => Some(Msg::Clipboard(true)),
                "paste" => Some(Msg::Paste),
                "rename" => Some(Msg::Modal(Modal::Rename)),
                "trash" => Some(Msg::Modal(Modal::Trash)),
                "folder" => Some(Msg::Modal(Modal::NewFolder)),
                "undo" => Some(Msg::Undo),
                "view" => Some(Msg::View),
                "sort" => Some(Msg::Sort),
                "hidden" => Some(Msg::Hidden),
                "home" => Some(Msg::Navigate(s.home.clone())),
                "forward" => Some(Msg::Forward),
                "help" => Some(Msg::Modal(Modal::Help)),
                "quit" => Some(Msg::Quit),
                _ => None,
            };
            if let Some(msg) = msg {
                update(s, msg, ctx);
            }
        }
        Msg::Open(path) => {
            s.drag_preview = None;
            if path.is_dir() {
                s.navigate(path, true);
            } else {
                s.active = Some(path);
                s.update_preview();
                s.modal = Some(Modal::Preview);
            }
        }
        Msg::Clipboard(cut) => {
            if !s.selected.is_empty() {
                s.clipboard = Some((cut, s.selected.iter().cloned().collect()));
                s.status = format!(
                    "{} {} items · navigate then Ctrl+V",
                    if cut { "Cut" } else { "Copied" },
                    s.selected.len()
                );
            }
        }
        Msg::Paste => {
            if let Some((cut, sources)) = &s.clipboard {
                s.launch(
                    Job {
                        sources: sources.clone(),
                        destination: s.cwd.clone(),
                        operation: if *cut {
                            Operation::Move
                        } else {
                            Operation::Copy
                        },
                    },
                    ctx,
                );
            }
        }
        Msg::Drop {
            destination,
            copy,
            sources,
        } => {
            s.launch(
                Job {
                    sources,
                    destination,
                    operation: if copy {
                        Operation::Copy
                    } else {
                        Operation::Move
                    },
                },
                ctx,
            );
        }
        Msg::Modal(modal) => {
            s.drag_preview = None;
            if s.busy.is_some() && modal != Modal::Help {
                return;
            }
            if (modal == Modal::Rename && s.selected.len() != 1)
                || (modal == Modal::Trash && s.selected.is_empty())
            {
                return;
            }
            s.draft = match modal {
                Modal::Address => s.cwd.to_string_lossy().into_owned(),
                Modal::Search => s.filter.clone(),
                Modal::Rename => s
                    .selected
                    .first()
                    .and_then(|p| p.file_name())
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                _ => String::new(),
            };
            s.error.clear();
            s.modal = Some(modal);
        }
        Msg::Close => s.modal = None,
        Msg::Draft(value) => s.draft = value,
        Msg::Submit => {
            let result = match s.modal {
                Some(Modal::NewFolder) => files::valid_name(&s.draft)
                    .and_then(|name| std::fs::create_dir(s.cwd.join(name))),
                Some(Modal::Rename) => files::valid_name(&s.draft).and_then(|name| {
                    let from = s
                        .selected
                        .first()
                        .ok_or_else(|| std::io::Error::other("Select one item"))?
                        .clone();
                    let to = s.cwd.join(name);
                    files::move_new(&from, &to)?;
                    s.undo.push(vec![(to, from)]);
                    Ok(())
                }),
                Some(Modal::Search) => {
                    s.modal = None;
                    update(s, Msg::Filter(s.draft.clone()), ctx);
                    return;
                }
                Some(Modal::Address) => {
                    let p = PathBuf::from(s.draft.trim());
                    s.error.clear();
                    s.navigate(if p.is_absolute() { p } else { s.cwd.join(p) }, true);
                    return;
                }
                Some(Modal::Trash) => {
                    s.modal = None;
                    s.launch(
                        Job {
                            sources: s.selected.iter().cloned().collect(),
                            destination: s.cwd.clone(),
                            operation: Operation::Trash,
                        },
                        ctx,
                    );
                    return;
                }
                Some(Modal::Quit) => {
                    if s.busy.is_none() {
                        ctx.exit();
                    }
                    return;
                }
                _ => {
                    s.modal = None;
                    return;
                }
            };
            match result {
                Ok(()) => {
                    s.modal = None;
                    s.status = "Done · F5 refresh".into();
                    s.refresh();
                }
                Err(e) => s.error = e.to_string(),
            }
        }
        Msg::Progress(n, total) => s.busy = Some((n, total)),
        Msg::Finished(outcome, operation) => {
            s.busy = None;
            s.status = format!(
                "{} items complete{}",
                outcome.completed,
                if operation == Operation::Copy {
                    ""
                } else {
                    " · Ctrl+Z to undo"
                }
            );
            if !outcome.errors.is_empty() {
                s.status = format!(
                    "{} complete · {} failed · {}",
                    outcome.completed,
                    outcome.errors.len(),
                    outcome.errors.join(" | ")
                );
            }
            if !outcome.undo.is_empty() {
                s.undo.push(outcome.undo);
            }
            if operation == Operation::Move {
                if let Some((true, paths)) = &mut s.clipboard {
                    paths.retain(|p| files::exists(p));
                    if paths.is_empty() {
                        s.clipboard = None;
                    }
                }
            }
            s.refresh();
        }
        Msg::Undo => {
            if s.busy.is_some() {
                return;
            }
            if let Some(batch) = s.undo.pop() {
                s.busy = Some((0, batch.len()));
                ctx.spawn(
                    Task::new(move |task| {
                        let mut result = Outcome::default();
                        for (from, to) in batch.iter().rev() {
                            match files::move_new(from, to) {
                                Ok(()) => {
                                    result.completed += 1;
                                    if let Some(p) = from.parent().filter(|p| {
                                        p.file_name().is_some_and(|n| {
                                            n.to_string_lossy().starts_with(".orbit-trash-")
                                        })
                                    }) {
                                        let _ = std::fs::remove_dir(p);
                                    }
                                }
                                Err(e) => {
                                    result.errors.push(e.to_string());
                                    result.undo.push((from.clone(), to.clone()));
                                }
                            }
                        }
                        task.emit(Msg::Undone(result));
                        TaskOutcome::Complete
                    })
                    .key("file-operation"),
                );
            }
        }
        Msg::Undone(result) => {
            s.busy = None;
            s.status = format!(
                "Restored {} items{}",
                result.completed,
                if result.errors.is_empty() {
                    String::new()
                } else {
                    format!(" · {}", result.errors.join(" | "))
                }
            );
            if !result.undo.is_empty() {
                s.undo.push(result.undo);
            }
            s.refresh();
        }
        Msg::Escape => {
            s.drag_preview = None;
            if s.modal.take().is_none() {
                s.selected.clear();
                s.active = None;
                s.preview.clear();
            }
        }
        Msg::Quit => {
            if s.busy.is_some() {
                s.status = "Wait for the current file operation before quitting".into();
            } else {
                s.modal = Some(Modal::Quit);
            }
        }
    }
}
fn main() -> Result {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Orbit Files · a file manager built with rsille\n\n  --path DIR       open a real directory (operations change real files)\n  --headless       verify interactions and filesystem operations\n  --snapshot FILE  export an actual 120x42 terminal frame as SVG\n  --snapshot-drag FILE  export a captured multi-file drag as SVG\n  --inline         render in the normal screen\n\nDefault: disposable demo directory, removed on clean exit.\nCtrl+Q quit · F1 help · F2 rename · F5 refresh\nCopy / cut / paste: Ctrl+C / X / V in the file pane.");
        return Ok(());
    }
    if args.iter().any(|a| a == "--headless") {
        verify::run()?;
        return Ok(());
    }
    let demo = if args.iter().any(|a| a == "--path") {
        None
    } else {
        Some(files::Demo::new()?)
    };
    let home = if let Some(i) = args.iter().position(|a| a == "--path") {
        std::fs::canonicalize(args.get(i + 1).ok_or("--path needs a directory")?)?
    } else {
        demo.as_ref().unwrap().0.clone()
    };
    if !home.is_dir() {
        return Err("--path must be a directory".into());
    }
    let mut state = State::new(home, demo.is_some());
    if let Some(i) = args.iter().position(|a| a == "--snapshot-drag") {
        if !state.demo {
            return Err("--snapshot-drag uses the demo directory; omit --path".into());
        }
        return verify::snapshot_drag(
            state,
            args.get(i + 1)
                .ok_or("--snapshot-drag needs an output file")?,
        );
    }
    if let Some(i) = args.iter().position(|a| a == "--snapshot") {
        return verify::snapshot(
            state,
            args.get(i + 1).ok_or("--snapshot needs an output file")?,
        );
    }
    if let Ok((w, h)) = crossterm::terminal::size() {
        state.width = w;
        state.height = h;
    }
    app(state)
        .mode(if args.iter().any(|a| a == "--inline") {
            Mode::Inline
        } else {
            Mode::Fullscreen
        })
        .update(update)
        .view(ui::view)
        .run()?;
    drop(demo);
    Ok(())
}
