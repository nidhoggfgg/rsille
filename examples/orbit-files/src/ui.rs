use super::*;
use rsille::render::chunk::Chunk;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub const BG: Color = Color::Rgb(16, 22, 34);
pub const CARD: Color = Color::Rgb(22, 31, 46);
pub const LINE: Color = Color::Rgb(46, 61, 80);
pub const TEXT: Color = Color::Rgb(223, 232, 245);
pub const MUTED: Color = Color::Rgb(122, 143, 168);
pub const ACCENT: Color = Color::Rgb(131, 230, 206);
pub const VIOLET: Color = Color::Rgb(181, 164, 245);
pub const GOLD: Color = Color::Rgb(244, 206, 138);
pub const SELECTED: Color = Color::Rgb(37, 65, 82);

pub fn theme() -> Theme {
    let mut t = Theme::dark();
    let s = &mut t.styles;
    s.surface = Style::default().bg(BG).fg(TEXT);
    s.surface_modal = Style::default().bg(CARD).fg(TEXT);
    s.surface_elevated = s.surface_modal;
    s.text = Style::default().fg(TEXT);
    s.text_muted = Style::default().fg(MUTED);
    s.text_placeholder = s.text_muted;
    s.border = Style::default().fg(LINE);
    s.border_focused = Style::default().fg(ACCENT);
    s.interactive = Style::default().bg(CARD).fg(TEXT);
    s.interactive_focused = s.interactive.fg(ACCENT);
    s.primary_action = Style::default().bg(ACCENT).fg(BG).bold();
    s.primary_action_focused = s.primary_action.bg(GOLD);
    s.primary_action_hover = s.primary_action_focused;
    s.secondary_action = Style::default().bg(Color::Rgb(45, 63, 82)).fg(TEXT);
    s.secondary_action_focused = s.secondary_action.bg(Color::Rgb(65, 91, 114)).fg(ACCENT);
    s.secondary_action_hover = s.interactive_focused;
    s.selected = Style::default().bg(SELECTED).fg(ACCENT);
    s.selected_focused = s.selected.bold();
    s.cursor = Style::default().bg(ACCENT).fg(BG);
    t
}
pub fn truncate(text: &str, width: usize) -> String {
    let clean: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if clean.width() <= width {
        return clean;
    }
    if width == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut count = 0;
    for g in clean.graphemes(true) {
        if count + g.width() >= width {
            break;
        }
        out.push_str(g);
        count += g.width();
    }
    out.push('…');
    out
}
pub fn size(n: u64) -> String {
    if n >= 1_048_576 {
        format!("{:.1} MB", n as f64 / 1_048_576.0)
    } else if n >= 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
}
pub fn put(chunk: &mut Chunk, x: i32, y: i32, text: &str, fg: Color, bg: Color) {
    if x < 0
        || y < 0
        || x >= i32::from(chunk.area().width())
        || y >= i32::from(chunk.area().height())
    {
        return;
    }
    let text = truncate(text, (i32::from(chunk.area().width()) - x) as usize);
    let _ = chunk.set_string(
        x as u16,
        y as u16,
        &text,
        Style::default().fg(fg).bg(bg).to_render_style(),
    );
}
pub fn paint(chunk: &mut Chunk, x: i32, y: i32, w: i32, h: i32, color: Color) {
    let x1 = x.max(0).min(i32::from(chunk.area().width()));
    let y1 = y.max(0).min(i32::from(chunk.area().height()));
    let x2 = (x + w).max(x1).min(i32::from(chunk.area().width()));
    let y2 = (y + h).max(y1).min(i32::from(chunk.area().height()));
    let _ = chunk.fill(
        x1 as u16,
        y1 as u16,
        (x2 - x1) as u16,
        (y2 - y1) as u16,
        ' ',
        Style::default().bg(color).to_render_style(),
    );
}
fn action(label: &str, key: &str, f: impl Fn() -> Msg + 'static) -> Button<Msg> {
    button(label)
        .key(key)
        .variant(ButtonVariant::Ghost)
        .on_click(f)
}
// Dialog actions share a solid style and allocation regardless of label length.
struct DialogButton {
    child: [Box<dyn Widget<Msg>>; 1],
    width: u16,
}
impl Widget<Msg> for DialogButton {
    fn constraints(&self) -> Constraints {
        Constraints::fixed(self.width, 3)
    }
    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.child
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.render_children(chunk, self);
    }
}
fn dialog_actions(cancel: &str, confirm: &str) -> impl Widget<Msg> {
    let width = (cancel.width().max(confirm.width()) + 4).max(14) as u16;
    row()
        .gap(2)
        .justify_content(JustifyContent::End)
        .child(DialogButton {
            width,
            child: [Box::new(
                button(cancel)
                    .key("cancel")
                    .variant(ButtonVariant::Secondary)
                    .on_click(|| Msg::Close),
            )],
        })
        .child(DialogButton {
            width,
            child: [Box::new(
                button(confirm)
                    .key("confirm")
                    .variant(ButtonVariant::Secondary)
                    .on_click(|| Msg::Submit),
            )],
        })
}
pub fn view(s: &State) -> impl Widget<Msg> {
    let compact = s.width < 78;
    let location = s
        .cwd
        .strip_prefix(&s.home)
        .ok()
        .map(|p| {
            if p.as_os_str().is_empty() {
                "Home".to_owned()
            } else {
                format!("Home / {}", p.display())
            }
        })
        .unwrap_or_else(|| s.cwd.display().to_string());
    let mut navigation = grid()
        .columns(if compact { "5 5 1fr 5" } else { "5 5 1fr 24 5" })
        .rows("3")
        .gap(1)
        .child(action("←", "back", || Msg::Back).disabled(s.history_index == 0))
        .child(
            action("↑", "up", || Msg::Up)
                .disabled(s.cwd.parent().is_none() || (s.demo && s.cwd == s.home)),
        )
        .child(action(
            &truncate(
                &location,
                s.width.saturating_sub(if compact { 22 } else { 47 }) as usize,
            ),
            "address",
            || Msg::Modal(Modal::Address),
        ));
    if !compact {
        navigation = navigation.child(
            text_input()
                .key("search")
                .value(&s.filter)
                .placeholder("Search…")
                .on_change(Msg::Filter),
        );
    }
    navigation = navigation.child(action("…", "more", || Msg::Modal(Modal::Actions)));
    let board = board::Board {
        entries: s.visible(),
        busy: s.busy.is_some(),
        selected: s.selected.clone(),
        active: s.active.clone(),
        list: s.list,
        cut: s
            .clipboard
            .as_ref()
            .filter(|(cut, _)| *cut)
            .map(|(_, p)| p.clone())
            .unwrap_or_default(),
        identity: format!(
            "{:?}|{}|{}|{}|{:?}|{}",
            s.cwd, s.filter, s.list, s.hidden, s.sort, s.descending
        ),
    };
    let total_bytes: u64 = s
        .entries
        .iter()
        .filter(|e| s.selected.contains(&e.path) && !e.dir)
        .map(|e| e.size)
        .sum();
    let info = format!(
        "  {} items{}",
        s.visible().len(),
        if s.selected.is_empty() {
            String::new()
        } else {
            format!(" · {} selected · {}", s.selected.len(), size(total_bytes))
        }
    );
    let status = if let Some((n, total)) = s.busy {
        format!("{} {n}/{total}", ["◐", "◓", "◑", "◒"][s.pulse])
    } else {
        s.status.clone()
    };
    let footer =
        row()
            .child(label(info).fg(MUTED))
            .child(spacer().fill())
            .child(
                label(truncate(&status, s.width.saturating_sub(36) as usize))
                    .fg(if s.busy.is_some() { ACCENT } else { MUTED }),
            );
    let mut root = portal(
        grid()
            .columns("1fr")
            .rows("3 1fr 1")
            .gap(0)
            .child(navigation)
            .child(board)
            .child(footer),
    );
    if let Some(modal) = s.modal {
        let (title, height) = match modal {
            Modal::NewFolder => ("New folder", 16),
            Modal::Rename => ("Rename", 16),
            Modal::Address => ("Go to folder", 16),
            Modal::Trash => ("Move to trash", 17),
            Modal::Help => ("Help", 29),
            Modal::Quit => ("Quit?", 14),
            Modal::Actions => ("Actions", 22),
            Modal::Preview => ("Preview", 28),
            Modal::Search => ("Search", 16),
        };
        let mut form = dialog()
            .key("form")
            .title(title)
            .border(BorderStyle::Rounded)
            .padding(Padding::new(1, 2, 1, 2))
            .gap(1);
        match modal {
            Modal::Actions => {
                form = form.gap(0).child(
                    menu()
                        .key("file-actions")
                        .borderless()
                        .height(15)
                        .items([
                            MenuItem::new("open", "Open / quick look             Enter")
                                .disabled(s.active.is_none()),
                            MenuItem::new("copy", "Copy                         Ctrl+C")
                                .disabled(s.selected.is_empty()),
                            MenuItem::new("cut", "Cut                          Ctrl+X")
                                .disabled(s.selected.is_empty()),
                            MenuItem::new("paste", "Paste                        Ctrl+V")
                                .disabled(s.clipboard.is_none()),
                            MenuItem::new("rename", "Rename                           F2")
                                .disabled(s.selected.len() != 1),
                            MenuItem::new("trash", "Move to trash                Delete")
                                .disabled(s.selected.is_empty()),
                            MenuItem::new("folder", "New folder                   Ctrl+N"),
                            MenuItem::new("undo", "Undo move / rename / trash   Ctrl+Z")
                                .disabled(s.undo.is_empty()),
                            MenuItem::new("view", if s.list { "Grid view" } else { "List view" }),
                            MenuItem::new(
                                "sort",
                                match s.sort {
                                    Sort::Name => "Sort: name",
                                    Sort::Size => "Sort: size",
                                    Sort::Modified => "Sort: modified",
                                },
                            ),
                            MenuItem::new(
                                "hidden",
                                if s.hidden {
                                    "Hide hidden files"
                                } else {
                                    "Show hidden files"
                                },
                            ),
                            MenuItem::new("home", "Home"),
                            MenuItem::new("forward", "Forward                      Alt+→")
                                .disabled(s.history_index + 1 >= s.history.len()),
                            MenuItem::new("help", "Help                            F1"),
                            MenuItem::new("quit", "Quit                        Ctrl+Q"),
                        ])
                        .on_select(Msg::Action)
                        .on_close(|| Msg::Close),
                );
            }
            Modal::Preview => {
                let title = s
                    .active
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .unwrap_or_default()
                    .to_string_lossy();
                form = form
                    .gap(1)
                    .child(label(truncate(&title, 56)).fg(TEXT))
                    .child(
                        grid().columns("1fr").rows("17").child(
                            scroll_view(label(s.preview.clone()).wrap_char())
                                .key("preview-content")
                                .focusable(true),
                        ),
                    )
                    .child(action("Close", "close-preview", || Msg::Close));
            }
            Modal::Help => {
                form = form.gap(0);
                for text in [
                    "MOUSE",
                    "Click · select     Ctrl+click · toggle",
                    "Shift+click · range    Double-click · open",
                    "Drag empty space · rectangular selection",
                    "Drag selected files onto a folder · move",
                    "Hold Ctrl while dropping · copy",
                    "Scroll wheel · browse   Right-click · actions",
                    "",
                    "KEYBOARD · in the file pane",
                    "Arrows / Home / End / PgUp / PgDn · navigate",
                    "Shift+arrows · range    Space · toggle",
                    "Ctrl+A · all    Ctrl+C / X / V · clipboard",
                    "Ctrl+Z · undo move / rename / trash",
                    "Ctrl+N · folder    Ctrl+H · hidden files",
                    "Enter · open    Backspace · parent folder",
                    "Alt+← / → · history    Ctrl+L · address",
                    "F2 · rename    F5 · refresh    Ctrl+F · search",
                    "Esc · cancel / clear    Ctrl+Q · quit",
                ] {
                    form = form.child(label(text).fg(
                        if text == "MOUSE" || text.starts_with("KEYBOARD") {
                            ACCENT
                        } else {
                            TEXT
                        },
                    ));
                }
                form = form.child(action("Got it", "close-help", || Msg::Close));
            }
            Modal::Trash => {
                form = form
                    .child(
                        label(format!(
                            "Move {} selected items to local trash?",
                            s.selected.len()
                        ))
                        .fg(TEXT),
                    )
                    .child(label("Ctrl+Z restores them during this session.").fg(MUTED))
                    .child(label("Trash stays on disk when using --path.").fg(MUTED));
                for p in s.selected.iter().take(2) {
                    form = form.child(
                        label(truncate(
                            &p.file_name().unwrap_or_default().to_string_lossy(),
                            45,
                        ))
                        .fg(GOLD),
                    );
                }
                form = form.child(dialog_actions("Cancel", "Move to trash"));
            }
            Modal::Quit => {
                form = form
                    .child(
                        label(if s.demo {
                            "This demo directory will be removed on exit."
                        } else {
                            "Your changes and local trash remain on disk."
                        })
                        .fg(TEXT),
                    )
                    .child(label("Undo history is kept only for this session.").fg(MUTED))
                    .child(dialog_actions("Stay", "Leave orbit"));
            }
            _ => {
                form = form
                    .child(
                        label(if modal == Modal::Search {
                            "Filter file names in the current folder"
                        } else if modal == Modal::Address {
                            "Absolute path or path relative to this folder"
                        } else {
                            if modal == Modal::NewFolder {
                                "Folder name"
                            } else {
                                "File name"
                            }
                        })
                        .fg(MUTED),
                    )
                    .child(
                        text_input()
                            .key("name-input")
                            .value(&s.draft)
                            .placeholder("Name…")
                            .on_change(Msg::Draft)
                            .on_submit(|_| Msg::Submit),
                    )
                    .child(label(truncate(&s.error, 46)).fg(GOLD))
                    .child(dialog_actions(
                        "Cancel",
                        if modal == Modal::Search {
                            "Search"
                        } else if modal == Modal::Address {
                            "Open folder"
                        } else {
                            "Save"
                        },
                    ));
            }
        }
        root = root.layer(
            PortalLayer::new(form)
                .key(format!("modal-{modal:?}"))
                .modal(true)
                .size(if modal == Modal::Preview { 66 } else { 54 }, height)
                .dismiss_on_outside_click(false)
                .on_dismiss(|| Msg::Close),
        );
    }
    drag::Layer {
        child: [Box::new(root)],
        preview: if s.modal.is_none() {
            s.drag_preview.clone()
        } else {
            None
        },
    }
}
