//! Floating Live Shell HUD built on `rsille-overlay` and `rsille-tui`.
//!
//! Floats a draggable, interactive utility panel over your real shell without
//! requiring Kitty or Sixel graphics: live command timer & exit badge,
//! clickable directory tree with one-click `cd here`, session checklist, and
//! quick shell macros.

use clap::ValueEnum;
use rsille::tui::prelude::*;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};

#[cfg(any(unix, windows))]
use rsille_overlay::{Anchor, Session, ShellEvent, WidgetOverlay};

const HUD_WIDTH: u16 = 46;
const HUD_HEIGHT: u16 = 16;

const DEFAULT_TODOS: [&str; 3] = [
    "Review git status & diff",
    "Run test suite",
    "Verify release build",
];

const QUICK_MACROS: [(&str, &str, &str); 5] = [
    ("yes", "» yes, continue", "yes, continue\r"),
    ("status", "git status -sb", "git status -sb\r"),
    ("diff", "git diff --stat", "git diff --stat\r"),
    ("ls", "ls -la", "ls -la\r"),
    ("clear", "clear screen", "clear\r"),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum HudTab {
    #[default]
    Files,
    Tasks,
    Quick,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum HudAnchor {
    #[default]
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}

#[derive(Clone, clap::Parser)]
pub struct Options {
    /// Initial active tab in the floating HUD
    #[arg(long, value_enum, default_value = "files")]
    pub tab: HudTab,
    /// Corner where the HUD floats initially (drag the title bar to move it)
    #[arg(long, value_enum, default_value = "top-right")]
    pub anchor: HudAnchor,
    /// Custom checklist items for the Tasks tab (repeatable)
    #[arg(long, value_name = "TASK")]
    pub todo: Vec<String>,
    /// Run on the terminal's alternate screen
    #[arg(long)]
    pub alt_screen: bool,
    /// Skip caching and restoring shell text covered by the HUD
    #[arg(long)]
    pub no_restore: bool,
    /// Verify HUD layout, widgets, and shell hooks without a TTY
    #[arg(long)]
    pub headless: bool,
    /// Program and arguments to run in the PTY instead of an interactive shell
    #[arg(last = true, value_name = "COMMAND", num_args = 1..)]
    pub command: Vec<std::ffi::OsString>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            tab: HudTab::Files,
            anchor: HudAnchor::TopRight,
            todo: Vec::new(),
            alt_screen: false,
            no_restore: false,
            headless: false,
            command: Vec::new(),
        }
    }
}

struct FsEntry {
    path: PathBuf,
    directory: bool,
    children: Option<std::ops::Range<usize>>,
}

impl FsEntry {
    fn name(&self) -> String {
        self.path
            .file_name()
            .unwrap_or(self.path.as_os_str())
            .to_string_lossy()
            .replace(char::is_control, "")
    }
}

struct TaskItem {
    name: String,
    done: bool,
}

enum ShellStatus {
    Idle,
    Editing,
    Running {
        cmd: String,
        started: Instant,
    },
    Done {
        cmd: String,
        status: i32,
        elapsed: Duration,
    },
}

struct HudState {
    tab: HudTab,
    entries: Vec<FsEntry>,
    selected_entry: usize,
    tasks: Vec<TaskItem>,
    initial_tasks: Vec<String>,
    shell_status: ShellStatus,
    shell_busy: bool,
    notice: Option<String>,
    sent_flash: Option<String>,
    tick_count: u64,
}

#[derive(Debug, Clone)]
enum Msg {
    SwitchTab(HudTab),
    SelectEntry(String),
    ExpandEntry(String),
    CdHere,
    ToggleTask(usize, bool),
    ClearDone,
    ResetTasks,
    SendMacro(String, String),
    ClearFlash,
    ClearStatus,
    ShellPrompt,
    ShellEdit(bool),
    ShellExec(String),
    ShellDone(i32),
    Tick,
}

impl HudState {
    fn new(root: PathBuf, tab: HudTab, custom_todos: &[String]) -> Self {
        let initial_tasks: Vec<String> = if custom_todos.is_empty() {
            DEFAULT_TODOS.iter().map(|s| (*s).to_owned()).collect()
        } else {
            custom_todos.to_vec()
        };
        let tasks = initial_tasks
            .iter()
            .map(|name| TaskItem {
                name: name.clone(),
                done: false,
            })
            .collect();
        let mut state = Self {
            tab,
            entries: vec![FsEntry {
                path: root,
                directory: true,
                children: None,
            }],
            selected_entry: 0,
            tasks,
            initial_tasks,
            shell_status: ShellStatus::Idle,
            shell_busy: false,
            notice: None,
            sent_flash: None,
            tick_count: 0,
        };
        let _ = state.expand(0);
        state
    }

    fn expand(&mut self, index: usize) -> io::Result<()> {
        let Some(entry) = self.entries.get(index) else {
            return Ok(());
        };
        if entry.children.is_some() || !entry.directory {
            return Ok(());
        }
        let mut children = Vec::new();
        for result in fs::read_dir(&entry.path)? {
            let child = result?;
            let kind = child.file_type()?;
            let path = child.path();
            let directory = kind.is_dir() || (kind.is_symlink() && path.is_dir());
            children.push(FsEntry {
                path,
                directory,
                children: None,
            });
        }
        children.sort_by(|a, b| {
            b.directory
                .cmp(&a.directory)
                .then_with(|| a.path.cmp(&b.path))
        });
        let start = self.entries.len();
        self.entries.extend(children);
        self.entries[index].children = Some(start..self.entries.len());
        Ok(())
    }

    fn tree_item(&self, index: usize) -> FileExplorerItem {
        let entry = &self.entries[index];
        let name = entry.name();
        let id = index.to_string();
        match &entry.children {
            Some(children) => FileExplorerItem::directory(id, name)
                .children(children.clone().map(|i| self.tree_item(i))),
            None if entry.directory => FileExplorerItem::lazy_directory(id, name),
            None => FileExplorerItem::file(id, name),
        }
    }
}

fn shorten(line: &str, max: usize) -> String {
    let mut out: String = line.chars().take(max).collect();
    if line.chars().count() > max {
        out.pop();
        out.push('…');
    }
    out
}

fn cd_command(path: &Path) -> Result<String, &'static str> {
    let path = path
        .to_str()
        .filter(|p| !p.contains(char::is_control))
        .ok_or("cd needs a printable UTF-8 path")?;
    #[cfg(windows)]
    let shell = env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into());
    #[cfg(not(windows))]
    let shell = env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into());
    let shell = Path::new(&shell)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match shell.as_str() {
        "sh" | "dash" | "ash" | "bash" | "ksh" | "mksh" | "zsh" => {
            Ok(format!("cd '{}'\r", path.replace('\'', "'\\''")))
        }
        "fish" => Ok(format!(
            "cd '{}'\r",
            path.replace('\\', "\\\\").replace('\'', "\\'")
        )),
        "cmd" if !path.contains(['%', '!', '"']) => Ok(format!("cd /d \"{path}\"\r")),
        "cmd" => Err("cmd cannot quote this path literally"),
        "powershell" | "pwsh" if path.contains(['‘', '’', '‚', '‛']) => {
            Err("Unsupported quote in path")
        }
        "powershell" | "pwsh" => Ok(format!(
            "Set-Location -LiteralPath '{}'\r",
            path.replace('\'', "''")
        )),
        _ => Err("cd is unavailable for this shell"),
    }
}

fn base_app(state: HudState) -> App<HudState, Msg> {
    App::new(state)
        .with_theme(Theme::tokyo_night())
        .on_tick(Duration::from_millis(250), || Msg::Tick)
}

fn make_update(
    input: mpsc::Sender<Vec<u8>>,
) -> impl Fn(&mut HudState, Msg, &mut UpdateCtx<Msg>) + 'static {
    move |hud, msg, ctx| match msg {
        Msg::SwitchTab(tab) => {
            hud.tab = tab;
            hud.notice = None;
        }
        Msg::SelectEntry(id) => {
            if let Ok(idx) = id.parse::<usize>() {
                if idx < hud.entries.len() {
                    hud.selected_entry = idx;
                    hud.notice = None;
                }
            }
        }
        Msg::ExpandEntry(id) => {
            if let Ok(idx) = id.parse::<usize>() {
                hud.notice = hud.expand(idx).err().map(|err| err.to_string());
            }
        }
        Msg::CdHere => {
            if !hud.shell_busy {
                if let Some(entry) = hud.entries.get(hud.selected_entry) {
                    if entry.directory {
                        match cd_command(&entry.path) {
                            Ok(command) => {
                                let _ = input.send(command.into_bytes());
                                hud.notice = Some(format!("cd → {}", entry.name()));
                            }
                            Err(err) => hud.notice = Some(err.to_owned()),
                        }
                    }
                }
            }
        }
        Msg::ToggleTask(index, done) => {
            if let Some(task) = hud.tasks.get_mut(index) {
                task.done = done;
            }
        }
        Msg::ClearDone => hud.tasks.retain(|task| !task.done),
        Msg::ResetTasks => {
            hud.tasks = hud
                .initial_tasks
                .iter()
                .map(|name| TaskItem {
                    name: name.clone(),
                    done: false,
                })
                .collect();
        }
        Msg::SendMacro(id, payload) => {
            let _ = input.send(payload.into_bytes());
            hud.sent_flash = Some(id);
            ctx.debounce("hud-flash", Duration::from_millis(700), Msg::ClearFlash);
        }
        Msg::ClearFlash => hud.sent_flash = None,
        Msg::ClearStatus => hud.shell_status = ShellStatus::Idle,
        Msg::ShellPrompt => {
            hud.shell_busy = false;
            if matches!(hud.shell_status, ShellStatus::Editing) {
                hud.shell_status = ShellStatus::Idle;
            }
        }
        Msg::ShellEdit(editing) => {
            hud.shell_busy = editing;
            if editing && !matches!(hud.shell_status, ShellStatus::Running { .. }) {
                hud.shell_status = ShellStatus::Editing;
            } else if !editing && matches!(hud.shell_status, ShellStatus::Editing) {
                hud.shell_status = ShellStatus::Idle;
            }
        }
        Msg::ShellExec(line) => {
            hud.shell_busy = true;
            hud.shell_status = ShellStatus::Running {
                cmd: line,
                started: Instant::now(),
            };
        }
        Msg::ShellDone(status) => {
            hud.shell_busy = false;
            let (cmd, elapsed) = match &hud.shell_status {
                ShellStatus::Running { cmd, started } => (cmd.clone(), started.elapsed()),
                _ => ("cmd".to_owned(), Duration::ZERO),
            };
            hud.shell_status = ShellStatus::Done {
                cmd,
                status,
                elapsed,
            };
        }
        Msg::Tick => {
            hud.tick_count = hud.tick_count.wrapping_add(1);
        }
    }
}

fn view(hud: &HudState) -> impl Widget<Msg> {
    let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"][(hud.tick_count as usize) % 8];
    let status_widget = match &hud.shell_status {
        ShellStatus::Idle => label("● Shell ready · Ctrl+] exit").fg(Color::Rgb(120, 210, 180)),
        ShellStatus::Editing => label("✎ Editing command…").fg(Color::Rgb(140, 190, 255)),
        ShellStatus::Running { cmd, started } => label(format!(
            "{spinner} {}  {:.1}s",
            shorten(cmd, 24),
            started.elapsed().as_secs_f32()
        ))
        .fg(Color::Rgb(245, 200, 95)),
        ShellStatus::Done {
            cmd,
            status: 0,
            elapsed,
        } => label(format!(
            "✔ {}  {:.1}s",
            shorten(cmd, 24),
            elapsed.as_secs_f32()
        ))
        .fg(Color::Rgb(110, 232, 165)),
        ShellStatus::Done {
            cmd,
            status,
            elapsed,
        } => label(format!(
            "✘ {status} {}  {:.1}s",
            shorten(cmd, 20),
            elapsed.as_secs_f32()
        ))
        .fg(Color::Rgb(255, 118, 128)),
    };

    let status_bar = row::<Msg>().child(status_widget.width(38)).child(
        button("×")
            .variant(ButtonVariant::Ghost)
            .focusable(false)
            .on_click(|| Msg::ClearStatus),
    );

    let tab_btn = |tab: HudTab, title: &str| {
        let variant = if hud.tab == tab {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Ghost
        };
        button(title)
            .variant(variant)
            .focusable(false)
            .on_click(move || Msg::SwitchTab(tab))
    };

    let done_count = hud.tasks.iter().filter(|t| t.done).count();
    let tasks_label = format!("Tasks {done_count}/{}", hud.tasks.len());
    let tabs = row::<Msg>()
        .gap(1)
        .child(tab_btn(HudTab::Files, "Files"))
        .child(tab_btn(HudTab::Tasks, &tasks_label))
        .child(tab_btn(HudTab::Quick, "Quick"));

    let mut body = col::<Msg>().gap(0);
    match hud.tab {
        HudTab::Files => {
            let entry = &hud.entries[hud.selected_entry];
            let detail = match &hud.notice {
                Some(msg) => label(shorten(msg, 22)).fg(Color::Rgb(150, 215, 255)),
                None => label(shorten(&entry.name(), 22)).fg(Color::Rgb(165, 178, 205)),
            };
            body = body
                .child(
                    file_explorer()
                        .key("hud-files")
                        .borderless()
                        .height(8)
                        .item(hud.tree_item(0))
                        .on_change(Msg::SelectEntry)
                        .on_load_children(Msg::ExpandEntry),
                )
                .child(
                    row::<Msg>()
                        .gap(1)
                        .child(
                            button("cd here")
                                .disabled(!entry.directory || hud.shell_busy)
                                .focusable(false)
                                .on_click(|| Msg::CdHere),
                        )
                        .child(detail),
                );
        }
        HudTab::Tasks => {
            if hud.tasks.is_empty() {
                body = body.child(label("All clear! Nice work.").fg(Color::Rgb(110, 232, 165)));
            } else {
                for (i, task) in hud.tasks.iter().take(7).enumerate() {
                    body = body.child(
                        checkbox(shorten(&task.name, 34))
                            .checked(task.done)
                            .on_change(move |v| Msg::ToggleTask(i, v)),
                    );
                }
            }
            body = body.child(
                row::<Msg>()
                    .gap(1)
                    .child(
                        button("Clear done")
                            .disabled(done_count == 0)
                            .focusable(false)
                            .on_click(|| Msg::ClearDone),
                    )
                    .child(
                        button("Reset")
                            .variant(ButtonVariant::Ghost)
                            .focusable(false)
                            .on_click(|| Msg::ResetTasks),
                    ),
            );
        }
        HudTab::Quick => {
            body = body.child(
                label("One-click commands injected into shell:").fg(Color::Rgb(145, 160, 190)),
            );
            for (id, caption, payload) in QUICK_MACROS {
                let flashed = hud.sent_flash.as_deref() == Some(id);
                let text = if flashed {
                    format!("✓ Sent: {caption}")
                } else {
                    caption.to_owned()
                };
                let id_owned = id.to_owned();
                let payload_owned = payload.to_owned();
                body = body.child(
                    button(text)
                        .variant(if flashed {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Ghost
                        })
                        .focusable(false)
                        .on_click(move || Msg::SendMacro(id_owned.clone(), payload_owned.clone())),
                );
            }
        }
    }

    panel()
        .title("rsille HUD  ·  drag title")
        .border(BorderStyle::Rounded)
        .padding(Padding::horizontal(1))
        .child(status_bar)
        .child(tabs)
        .child(body)
}

impl Options {
    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        if self.headless {
            return verify_headless(self);
        }
        crate::require_terminal()?;
        #[cfg(any(unix, windows))]
        {
            let direct = !self.command.is_empty();
            let root = env::current_dir()?;
            let (input, receiver) = mpsc::channel();
            let app = base_app(HudState::new(root, self.tab, &self.todo))
                .update(make_update(input))
                .view(view)
                .build();
            let anchor = match self.anchor {
                HudAnchor::TopRight => Anchor::TopRight,
                HudAnchor::TopLeft => Anchor::TopLeft,
                HudAnchor::BottomRight => Anchor::BottomRight,
                HudAnchor::BottomLeft => Anchor::BottomLeft,
            };
            let overlay = WidgetOverlay::new(app, (HUD_WIDTH, HUD_HEIGHT))
                .primary_screen_only()
                .anchor(anchor)
                .draggable_title()
                .on_shell(|event| match event {
                    ShellEvent::Prompt => Some(Msg::ShellPrompt),
                    ShellEvent::Edit(line) => Some(Msg::ShellEdit(!line.is_empty())),
                    ShellEvent::Exec(line) => Some(Msg::ShellExec(line.to_owned())),
                    ShellEvent::Done(status) => Some(Msg::ShellDone(status)),
                    ShellEvent::Read => Some(Msg::ShellEdit(true)),
                });
            let mut session = Session::new()
                .name("rsille hud")
                .alt_screen(self.alt_screen)
                .restore_text(!self.no_restore)
                .mouse(true)
                .shell_events(!direct);
            if direct {
                session = session.command(self.command);
            }
            let status = session.run_with_input(overlay, receiver)?;
            if direct && status != 0 {
                std::process::exit(status);
            }
            Ok(())
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err("rsille hud requires Unix PTYs or Windows ConPTY".into())
        }
    }
}

fn verify_headless(options: Options) -> Result<(), Box<dyn std::error::Error>> {
    use rsille::tui::runtime::ManualClock;
    let clock = ManualClock::default();
    let root = env::current_dir()?;
    let (tx, rx) = mpsc::channel();
    let mut rt = base_app(HudState::new(root, options.tab, &options.todo))
        .with_clock(clock.clone())
        .update(make_update(tx))
        .view(view)
        .build()
        .size(HUD_WIDTH, HUD_HEIGHT);
    rt.step(&[])?;

    // Exercise shell status lifecycle
    rt.handle()
        .send(Msg::ShellExec("cargo test".into()))
        .unwrap();
    rt.step(&[])?;
    clock.advance(Duration::from_millis(300));
    rt.step(&[])?;
    rt.handle().send(Msg::ShellDone(0)).unwrap();
    rt.step(&[])?;

    // Exercise cd injection
    rt.handle().send(Msg::CdHere).unwrap();
    rt.step(&[])?;
    let injected = rx.try_recv().expect("cd command must be sent");
    assert!(!injected.is_empty());

    // Exercise Tasks tab
    rt.handle().send(Msg::SwitchTab(HudTab::Tasks)).unwrap();
    rt.step(&[])?;
    rt.handle().send(Msg::ToggleTask(0, true)).unwrap();
    rt.step(&[])?;
    rt.handle().send(Msg::ClearDone).unwrap();
    rt.step(&[])?;

    // Exercise Quick tab macro injection
    rt.handle().send(Msg::SwitchTab(HudTab::Quick)).unwrap();
    rt.step(&[])?;
    rt.handle()
        .send(Msg::SendMacro("yes".into(), "yes, continue\r".into()))
        .unwrap();
    rt.step(&[])?;
    assert_eq!(rx.try_recv().unwrap(), b"yes, continue\r");

    let text: String = rt
        .frame()
        .content()
        .iter()
        .map(|c| c.content.symbol())
        .collect();
    assert!(text.contains("rsille HUD"));
    println!("HUD headless verified: files tree + cd injection, shell status timer, tasks checklist, and quick macros.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_hud_exercises_all_tabs_and_shell_events() {
        verify_headless(Options::default()).unwrap();
    }

    #[test]
    fn custom_todos_override_defaults() {
        let state = HudState::new(
            PathBuf::from("."),
            HudTab::Tasks,
            &["Ship v0.2".to_owned(), "Write changelog".to_owned()],
        );
        assert_eq!(state.tasks.len(), 2);
        assert_eq!(state.tasks[0].name, "Ship v0.2");
    }
}
