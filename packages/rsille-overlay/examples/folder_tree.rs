//! Browse from the launch directory. Drag the title to move the panel, click a
//! directory to expand it, and click "cd here" at a shell prompt to enter it.
use rsille_overlay::{Anchor, Session, ShellEvent, WidgetOverlay};
use rsille_tui::prelude::*;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    sync::mpsc,
};

struct Entry {
    path: PathBuf,
    directory: bool,
    children: Option<std::ops::Range<usize>>,
}

impl Entry {
    fn name(&self) -> String {
        self.path
            .file_name()
            .unwrap_or(self.path.as_os_str())
            .to_string_lossy()
            .replace(char::is_control, "�")
    }
}

struct Browser {
    entries: Vec<Entry>,
    selected: usize,
    error: Option<String>,
    busy: bool,
}

enum Msg {
    Select(String),
    Expand(String),
    Cd,
    Busy(bool),
}

impl Browser {
    fn new(root: PathBuf) -> Self {
        Self {
            entries: vec![Entry {
                path: root,
                directory: true,
                children: None,
            }],
            selected: 0,
            error: None,
            busy: false,
        }
    }

    fn expand(&mut self, index: usize) -> io::Result<()> {
        let entry = &self.entries[index];
        if entry.children.is_some() {
            return Ok(());
        }
        let mut children = Vec::new();
        for result in fs::read_dir(&entry.path)? {
            let child = result?;
            let kind = child.file_type()?;
            let path = child.path();
            let directory = kind.is_dir() || (kind.is_symlink() && path.is_dir());
            children.push(Entry {
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

    fn item(&self, index: usize) -> FileExplorerItem {
        let entry = &self.entries[index];
        let name = entry.name();
        let id = index.to_string();
        match &entry.children {
            Some(children) => FileExplorerItem::directory(id, name)
                .children(children.clone().map(|i| self.item(i))),
            None if entry.directory => FileExplorerItem::lazy_directory(id, name),
            None => FileExplorerItem::file(id, name),
        }
    }
}

// The session starts $SHELL on Unix and %COMSPEC% on Windows. Always send an
// absolute path so the target stays correct after manual shell navigation.
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

fn view(browser: &Browser) -> impl Widget<Msg> {
    let entry = &browser.entries[browser.selected];
    let detail = match &browser.error {
        Some(error) => label(error.clone()).fg(Color::Red),
        None => label(entry.name()),
    };
    panel()
        .title("Folders  ·  drag to move")
        .border(BorderStyle::Rounded)
        .padding(Padding::uniform(0))
        .child(
            file_explorer()
                .key("files")
                .borderless()
                .height(10)
                .item(browser.item(0))
                .on_change(Msg::Select)
                .on_load_children(Msg::Expand),
        )
        .child(
            row()
                .gap(1)
                .child(
                    button("cd here")
                        .disabled(!entry.directory || browser.busy)
                        .on_click(|| Msg::Cd),
                )
                .child(detail),
        )
}

fn main() -> io::Result<()> {
    let (input, receiver) = mpsc::channel();
    let app = App::new(Browser::new(env::current_dir()?))
        .update(move |browser, msg, _| match msg {
            Msg::Select(id) => {
                browser.selected = id.parse().expect("numeric entry ID");
                browser.error = None;
            }
            Msg::Expand(id) => {
                browser.error = browser
                    .expand(id.parse().expect("numeric entry ID"))
                    .err()
                    .map(|err| err.to_string());
            }
            Msg::Cd if !browser.busy && browser.entries[browser.selected].directory => {
                match cd_command(&browser.entries[browser.selected].path) {
                    Ok(command) => input
                        .send(command.into_bytes())
                        .expect("session is running"),
                    Err(error) => browser.error = Some(error.into()),
                }
            }
            Msg::Cd => {}
            Msg::Busy(busy) => browser.busy = busy,
        })
        .view(view)
        .build();
    let overlay = WidgetOverlay::new(app, (44, 13))
        .primary_screen_only()
        .anchor(Anchor::TopRight)
        .draggable_title()
        .on_shell(|event| match event {
            ShellEvent::Prompt => Some(Msg::Busy(false)),
            ShellEvent::Edit(line) => Some(Msg::Busy(!line.is_empty())),
            ShellEvent::Exec(_) | ShellEvent::Read => Some(Msg::Busy(true)),
            ShellEvent::Done(_) => None,
        });
    Session::new()
        .mouse(true)
        .run_with_input(overlay, receiver)?;
    Ok(())
}
