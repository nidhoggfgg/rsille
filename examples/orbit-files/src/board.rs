//! Captured pointer gestures and a virtualized, controlled file surface.
use super::*;
use rsille::render::chunk::Chunk;
use std::time::Instant;
use ui::{paint, put, ACCENT, BG, CARD, LINE, MUTED, SELECTED, TEXT};

pub struct Board {
    pub entries: Vec<Entry>,
    pub busy: bool,
    pub selected: BTreeSet<PathBuf>,
    pub active: Option<PathBuf>,
    pub cut: Vec<PathBuf>,
    pub list: bool,
    pub identity: String,
}
#[derive(Clone)]
struct Gesture {
    origin: (i32, i32),
    current: (i32, i32),
    item: Option<usize>,
    base: BTreeSet<PathBuf>,
    restore: BTreeSet<PathBuf>,
    prior_active: Option<PathBuf>,
    moved: bool,
    modifiers: KeyModifiers,
    files: BTreeSet<PathBuf>,
}
#[derive(Default)]
struct Local {
    identity: String,
    paths: Vec<PathBuf>,
    scroll: i32,
    hover: Option<usize>,
    gesture: Option<Gesture>,
    last_click: Option<(PathBuf, Instant)>,
    anchor: Option<usize>,
    selected: BTreeSet<PathBuf>,
    active: Option<PathBuf>,
}
#[derive(Clone, Copy, Debug)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}
impl Rect {
    fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
    fn intersects(self, other: Self) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }
}
fn emit_selection(ctx: &mut EventCtx<Msg>, paths: Vec<PathBuf>, active: Option<PathBuf>) {
    let local = ctx.state_mut::<Local>();
    local.selected = paths.iter().cloned().collect();
    local.active = active.clone();
    ctx.emit(Msg::Select(paths, active));
}
impl Board {
    fn columns(&self, width: i32) -> usize {
        if self.list {
            1
        } else {
            ((width - 4) / 18).max(1) as usize
        }
    }
    fn row_height(&self) -> i32 {
        if self.list {
            2
        } else {
            6
        }
    }
    fn rect(&self, index: usize, width: i32) -> Rect {
        let cols = self.columns(width);
        let step = (width - 4).max(1) / cols as i32;
        Rect {
            x: 2 + (index % cols) as i32 * step,
            y: 2 + (index / cols) as i32 * self.row_height(),
            w: (step - if self.list { 0 } else { 2 }).max(1),
            h: if self.list { 1 } else { 5 },
        }
    }
    fn hit(&self, x: i32, y: i32, width: i32) -> Option<usize> {
        if x < 2 || y < 2 {
            return None;
        }
        let cols = self.columns(width);
        let step = (width - 4).max(1) / cols as i32;
        let col = (x - 2) / step;
        if col >= cols as i32 {
            return None;
        }
        let index = ((y - 2) / self.row_height()) as usize * cols + col as usize;
        (index < self.entries.len() && self.rect(index, width).contains(x, y)).then_some(index)
    }
    fn max_scroll(&self, width: i32, height: i32) -> i32 {
        (self.entries.len().div_ceil(self.columns(width)) as i32 * self.row_height() + 3 - height)
            .max(0)
    }
    fn drop_target(&self, g: &Gesture, w: i32, h: i32, scroll: i32) -> drag::Target {
        if self.busy {
            return drag::Target::Blocked("Processing files");
        }
        if !(0..w).contains(&g.current.0) || !(1..h).contains(&(g.current.1 - scroll)) {
            return drag::Target::Blocked("Outside file area");
        }
        match self
            .hit(g.current.0, g.current.1, w)
            .and_then(|i| self.entries.get(i))
        {
            Some(entry) if g.files.contains(&entry.path) => drag::Target::Blocked("Same selection"),
            Some(entry) if entry.dir => {
                drag::Target::Folder(entry.path.clone(), entry.name.clone())
            }
            Some(_) => drag::Target::Blocked("Not a folder"),
            None => drag::Target::Blocked("Choose a folder"),
        }
    }
    fn preview(
        &self,
        g: &Gesture,
        bounds: rsille::render::area::Area,
        scroll: i32,
    ) -> drag::Preview {
        let lead = g.item.and_then(|i| self.entries.get(i));
        let entries = lead
            .into_iter()
            .chain(self.entries.iter().filter(|e| {
                g.files.contains(&e.path) && lead.is_none_or(|first| first.path != e.path)
            }))
            .take(3)
            .cloned()
            .collect();
        drag::Preview {
            pointer: CellPoint::new(
                i32::from(bounds.x()) + g.current.0,
                i32::from(bounds.y()) + g.current.1 - scroll,
            ),
            entries,
            total: g.files.len(),
            copy: g.modifiers.contains(KeyModifiers::CONTROL),
            target: self.drop_target(g, bounds.width().into(), bounds.height().into(), scroll),
        }
    }
    fn select(
        &self,
        indices: impl IntoIterator<Item = usize>,
        active: Option<usize>,
        ctx: &mut EventCtx<Msg>,
    ) {
        emit_selection(
            ctx,
            indices
                .into_iter()
                .filter_map(|i| self.entries.get(i).map(|e| e.path.clone()))
                .collect(),
            active
                .and_then(|i| self.entries.get(i))
                .map(|e| e.path.clone()),
        );
    }
    fn marquee(&self, gesture: &Gesture, width: i32, ctx: &mut EventCtx<Msg>) {
        let (x, y) = gesture.origin;
        let (xx, yy) = gesture.current;
        let rect = Rect {
            x: x.min(xx),
            y: y.min(yy),
            w: (x - xx).abs() + 1,
            h: (y - yy).abs() + 1,
        };
        let mut selection = gesture.base.clone();
        for (i, entry) in self.entries.iter().enumerate() {
            if self.rect(i, width).intersects(rect) {
                selection.insert(entry.path.clone());
            }
        }
        let active = selection.first().cloned();
        emit_selection(ctx, selection.into_iter().collect(), active);
    }
    fn render_contents(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let local = ctx.state_or_default::<Local>();
        let (w, h) = (
            i32::from(chunk.area().width()),
            i32::from(chunk.area().height()),
        );
        paint(chunk, 0, 0, w, h, BG);
        if w < 1 || h < 2 {
            return;
        }
        let scroll = local.scroll.min(self.max_scroll(w, h));
        let dragging = local
            .gesture
            .as_ref()
            .filter(|g| g.moved && g.item.is_some());
        let target = dragging.map(|g| self.drop_target(g, w, h, scroll));
        for (i, entry) in self.entries.iter().enumerate() {
            let r = self.rect(i, w);
            let y = r.y - scroll;
            if y + r.h < 1 || y >= h {
                continue;
            }
            let selected = self.selected.contains(&entry.path);
            let lifted = dragging.is_some_and(|g| g.files.contains(&entry.path));
            let drop_target = target.as_ref().and_then(drag::Target::path) == Some(&entry.path);
            let bg = if drop_target {
                Color::Rgb(40, 89, 85)
            } else if lifted {
                Color::Rgb(24, 35, 48)
            } else if selected {
                SELECTED
            } else if local.hover == Some(i) {
                CARD
            } else {
                BG
            };
            let dim = lifted || self.cut.contains(&entry.path);
            let color = if dim { MUTED } else { icons::color(entry) };
            if self.list {
                paint(chunk, r.x, y, r.w, 1, bg);
                let marker = if drop_target {
                    "→"
                } else if selected {
                    "●"
                } else {
                    " "
                };
                put(
                    chunk,
                    r.x,
                    y,
                    &format!("{marker} {}", icons::symbol(entry)),
                    color,
                    bg,
                );
                let name_width = if r.w > 36 { r.w - 24 } else { r.w - 4 };
                put(
                    chunk,
                    r.x + 4,
                    y,
                    &ui::truncate(&entry.name, name_width.max(0) as usize),
                    if dim { MUTED } else { TEXT },
                    bg,
                );
                if r.w > 36 {
                    put(chunk, r.x + r.w - 20, y, entry.kind(), MUTED, bg);
                    put(chunk, r.x + r.w - 10, y, &ui::size(entry.size), MUTED, bg);
                }
            } else {
                paint(chunk, r.x, y, r.w, r.h, bg);
                icons::draw(
                    chunk,
                    r.x + (r.w - icons::WIDTH).max(0) / 2,
                    y,
                    entry,
                    bg,
                    dim,
                );
                let name = ui::truncate(&entry.name, (r.w - 2).max(1) as usize);
                let name_w = unicode_width::UnicodeWidthStr::width(name.as_str()) as i32;
                put(
                    chunk,
                    r.x + ((r.w - name_w) / 2).max(0),
                    y + icons::HEIGHT,
                    &name,
                    if dim { MUTED } else { TEXT },
                    bg,
                );
                if drop_target {
                    let label =
                        if dragging.is_some_and(|g| g.modifiers.contains(KeyModifiers::CONTROL)) {
                            "+ Copy here"
                        } else {
                            "↳ Move here"
                        };
                    drag::outline(chunk, (r.x, y, r.w, r.h), ACCENT, bg);
                    put(chunk, r.x + 2, y, label, ACCENT, bg);
                }
                if selected {
                    put(
                        chunk,
                        r.x + r.w - 2,
                        y,
                        "●",
                        if lifted { MUTED } else { ACCENT },
                        bg,
                    );
                }
                if self.active.as_ref() == Some(&entry.path) && ctx.is_focused() {
                    put(chunk, r.x, y, "▎", ACCENT, bg);
                }
            }
        }
        paint(chunk, 0, 0, w, 1, BG);
        if self.list {
            put(chunk, 2, 0, "Name", MUTED, BG);
            if w > 40 {
                put(chunk, w - 22, 0, "Type", MUTED, BG);
                put(chunk, w - 12, 0, "Size", MUTED, BG);
            }
        }
        if self.entries.is_empty() {
            put(chunk, 3, 4, "No files", MUTED, BG);
        }
        if self.max_scroll(w, h) > 0 && w > 0 && h > 3 {
            let y = 2 + scroll * (h - 4) / self.max_scroll(w, h);
            for row in 2..h - 1 {
                put(chunk, w - 1, row, "│", LINE, BG);
            }
            put(chunk, w - 1, y, "┃", ACCENT, BG);
        }
    }
}
impl Widget<Msg> for Board {
    fn key(&self) -> Option<&str> {
        Some("files")
    }
    fn focus_config(&self) -> FocusConfig {
        FocusConfig::Leaf
    }
    fn constraints(&self) -> Constraints {
        Constraints::fill()
    }
    fn lifecycle(&self, ctx: &mut StateCtx) {
        let local = ctx.state_mut::<Local>();
        if local.identity != self.identity {
            *local = Local {
                identity: self.identity.clone(),
                ..Local::default()
            };
            ctx.cancel_pointer(DragCancelReason::ControlledValueChanged);
        }
        let paths: Vec<_> = self.entries.iter().map(|e| e.path.clone()).collect();
        let local = ctx.state_mut::<Local>();
        if local.paths != paths {
            local.paths = paths;
            local.gesture = None;
            local.hover = None;
            local.anchor = None;
            ctx.cancel_pointer(DragCancelReason::ControlledValueChanged);
        }
        let local = ctx.state_mut::<Local>();
        local.selected = self.selected.clone();
        local.active = self.active.clone();
    }
    fn pointer_cancel(&self, reason: DragCancelReason, ctx: &mut EventCtx<Msg>) {
        ctx.emit(Msg::DragPreview(None));
        let gesture = ctx.state_mut::<Local>().gesture.take();
        if let Some(g) = gesture.filter(|g| g.item.is_none()) {
            emit_selection(ctx, g.restore.into_iter().collect(), g.prior_active);
        }
        if reason == DragCancelReason::Escape {
            ctx.set_handled();
        }
        ctx.request_paint();
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let local = ctx.state_or_default::<Local>();
        let Some(g) = local
            .gesture
            .as_ref()
            .filter(|g| g.item.is_none() && g.moved)
        else {
            self.render_contents(chunk, ctx);
            return;
        };
        // Composite only the background so the mask preserves text and wide glyphs.
        let size = chunk.area().size();
        let mut buffer = rsille::render::buffer::Buffer::new(size);
        buffer.set_text_metrics(chunk.text_metrics());
        let area = rsille::render::area::Area::new((0, 0).into(), size);
        self.render_contents(&mut Chunk::new(&mut buffer, area).unwrap(), ctx);
        let (w, h) = (i32::from(size.width), i32::from(size.height));
        let scroll = local.scroll.min(self.max_scroll(w, h));
        let right = w - i32::from(self.max_scroll(w, h) > 0);
        for (x, y, cell) in buffer.all_cells() {
            let mut content = cell.content.clone();
            let (px, py) = (i32::from(x), i32::from(y));
            if px < right
                && py >= 1
                && px >= g.origin.0.min(g.current.0)
                && px <= g.origin.0.max(g.current.0)
                && py + scroll >= g.origin.1.min(g.current.1)
                && py + scroll <= g.origin.1.max(g.current.1)
            {
                let colors = content
                    .style
                    .colors
                    .get_or_insert(crossterm::style::Colors {
                        foreground: None,
                        background: None,
                    });
                let bg = match colors.background {
                    Some(crossterm::style::Color::Rgb { r, g, b }) => Color::Rgb(r, g, b),
                    _ => BG,
                };
                colors.background = Style::default()
                    .bg(bg.interpolate(ACCENT, 0.18))
                    .to_render_style()
                    .colors
                    .and_then(|c| c.background);
            }
            let _ = chunk.set_forced(x, y, content);
        }
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<Msg>) {
        if ctx.phase() != EventPhase::Target {
            return;
        }
        let selected = ctx.state_mut::<Local>().selected.clone();
        let active_path = ctx.state_mut::<Local>().active.clone();
        let Some(bounds) = ctx.bounds() else {
            return;
        };
        let (w, h) = (i32::from(bounds.width()), i32::from(bounds.height()));
        if w < 1 || h < 2 {
            return;
        }
        if let Event::Mouse(mouse) = event {
            let Some(p) = ctx.cell_mouse_position(event) else {
                return;
            };
            let (x, screen_y) = (p.x - i32::from(bounds.x()), p.y - i32::from(bounds.y()));
            let scroll = ctx.state_mut::<Local>().scroll.min(self.max_scroll(w, h));
            ctx.state_mut::<Local>().scroll = scroll;
            let y = screen_y + scroll;
            let hit = if screen_y >= 1 {
                self.hit(x, y, w)
            } else {
                None
            };
            match mouse.kind {
                MouseEventKind::Moved => {
                    ctx.state_mut::<Local>().hover = hit;
                    ctx.request_paint();
                }
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                    let delta = if mouse.kind == MouseEventKind::ScrollDown {
                        self.row_height() * 2
                    } else {
                        -self.row_height() * 2
                    };
                    ctx.state_mut::<Local>().scroll =
                        (scroll + delta).clamp(0, self.max_scroll(w, h));
                    ctx.request_paint();
                    ctx.set_handled();
                }
                MouseEventKind::Down(MouseButton::Right) => {
                    ctx.emit(Msg::Context(hit.map(|i| self.entries[i].path.clone())));
                    ctx.set_handled();
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    ctx.request_focus_self();
                    ctx.capture_pointer();
                    let additive = mouse
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::SHIFT);
                    let base = if additive {
                        selected.clone()
                    } else {
                        BTreeSet::new()
                    };
                    if let Some(i) = hit {
                        let entry = &self.entries[i];
                        let mut next = selected.clone();
                        if mouse.modifiers.contains(KeyModifiers::SHIFT) {
                            let anchor = ctx
                                .state_mut::<Local>()
                                .anchor
                                .unwrap_or(i)
                                .min(self.entries.len() - 1);
                            for e in &self.entries[anchor.min(i)..=anchor.max(i)] {
                                next.insert(e.path.clone());
                            }
                        } else if mouse.modifiers.contains(KeyModifiers::CONTROL) {
                            if !next.remove(&entry.path) {
                                next.insert(entry.path.clone());
                            }
                            ctx.state_mut::<Local>().anchor = Some(i);
                        } else if !next.contains(&entry.path) {
                            next = BTreeSet::from([entry.path.clone()]);
                            ctx.state_mut::<Local>().anchor = Some(i);
                        }
                        emit_selection(ctx, next.into_iter().collect(), Some(entry.path.clone()));
                    } else {
                        emit_selection(ctx, base.iter().cloned().collect(), None);
                    }
                    let files = ctx.state_mut::<Local>().selected.clone();
                    ctx.state_mut::<Local>().gesture = Some(Gesture {
                        origin: (x, y),
                        current: (x, y),
                        item: hit,
                        base,
                        restore: selected.clone(),
                        prior_active: active_path.clone(),
                        moved: false,
                        modifiers: mouse.modifiers,
                        files,
                    });
                    ctx.set_handled();
                }
                MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left) => {
                    let Some(mut g) = ctx.state_mut::<Local>().gesture.take() else {
                        return;
                    };
                    let next_scroll = if screen_y >= h - 1 {
                        scroll + self.row_height()
                    } else if screen_y < 1 {
                        scroll - self.row_height()
                    } else {
                        scroll
                    };
                    let next_scroll = next_scroll.clamp(0, self.max_scroll(w, h));
                    ctx.state_mut::<Local>().scroll = next_scroll;
                    g.current = (x, screen_y + next_scroll);
                    g.modifiers = mouse.modifiers;
                    let was_moved = g.moved;
                    g.moved |= (g.origin.0 - x).abs().max((g.origin.1 - g.current.1).abs()) >= 2;
                    if g.moved && !was_moved {
                        if let Some(entry) = g.item.and_then(|i| self.entries.get(i)) {
                            if !g.files.contains(&entry.path) {
                                if g.restore.contains(&entry.path) {
                                    g.files = g.restore.clone();
                                }
                                g.files.insert(entry.path.clone());
                            }
                            emit_selection(
                                ctx,
                                g.files.iter().cloned().collect(),
                                Some(entry.path.clone()),
                            );
                        }
                    }
                    if g.item.is_none() && g.moved {
                        self.marquee(&g, w, ctx);
                    }
                    if matches!(mouse.kind, MouseEventKind::Up(_)) {
                        ctx.release_pointer();
                        ctx.emit(Msg::DragPreview(None));
                        if g.moved && g.item.is_some() {
                            if let drag::Target::Folder(path, _) =
                                self.drop_target(&g, w, h, next_scroll)
                            {
                                ctx.emit(Msg::Drop {
                                    destination: path,
                                    copy: g.modifiers.contains(KeyModifiers::CONTROL),
                                    sources: g.files.iter().cloned().collect(),
                                });
                            }
                        } else if !g.moved {
                            if let Some(i) = g.item {
                                let path = self.entries[i].path.clone();
                                if !mouse
                                    .modifiers
                                    .intersects(KeyModifiers::CONTROL | KeyModifiers::SHIFT)
                                {
                                    let now = Instant::now();
                                    let double =
                                        ctx.state_mut::<Local>().last_click.as_ref().is_some_and(
                                            |(p, t)| {
                                                p == &path
                                                    && now.duration_since(*t)
                                                        < Duration::from_millis(400)
                                            },
                                        );
                                    emit_selection(ctx, vec![path.clone()], Some(path.clone()));
                                    if double {
                                        ctx.emit(Msg::Open(path));
                                        ctx.state_mut::<Local>().last_click = None;
                                    } else {
                                        ctx.state_mut::<Local>().last_click = Some((path, now));
                                    }
                                }
                            }
                        }
                    } else {
                        if g.moved && g.item.is_some() {
                            ctx.emit(Msg::DragPreview(Some(self.preview(
                                &g,
                                bounds,
                                next_scroll,
                            ))));
                        }
                        ctx.state_mut::<Local>().gesture = Some(g);
                    }
                    ctx.request_paint();
                    ctx.set_handled();
                }
                _ => {}
            }
        } else if let Event::Key(key) = event {
            use rsille::tui::event::ModifierKeyCode;
            if matches!(
                key.code,
                KeyCode::Modifier(ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl)
            ) {
                let local = ctx.state_mut::<Local>();
                if let Some(mut g) = local.gesture.take() {
                    g.modifiers
                        .set(KeyModifiers::CONTROL, key.kind != KeyEventKind::Release);
                    if g.moved && g.item.is_some() {
                        let scroll = ctx.state_mut::<Local>().scroll;
                        ctx.emit(Msg::DragPreview(Some(self.preview(&g, bounds, scroll))));
                    }
                    ctx.state_mut::<Local>().gesture = Some(g);
                    ctx.request_paint();
                    ctx.set_handled();
                    return;
                }
            }
            if key.kind == KeyEventKind::Release {
                return;
            }
            if key.code == KeyCode::F(5) {
                ctx.cancel_pointer();
                return;
            }
            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            let shift = key.modifiers.contains(KeyModifiers::SHIFT);
            let active = active_path
                .as_ref()
                .and_then(|p| self.entries.iter().position(|e| &e.path == p));
            let message = match key.code {
                KeyCode::Char('a') if ctrl => {
                    self.select(0..self.entries.len(), active.or(Some(0)), ctx);
                    None
                }
                KeyCode::Char('c') if ctrl => Some(Msg::Clipboard(false)),
                KeyCode::Char('x') if ctrl => Some(Msg::Clipboard(true)),
                KeyCode::Char('v') if ctrl => Some(Msg::Paste),
                KeyCode::Char('z') if ctrl => Some(Msg::Undo),
                KeyCode::Char('n') if ctrl => Some(Msg::Modal(Modal::NewFolder)),
                KeyCode::Char('h') if ctrl => Some(Msg::Hidden),
                KeyCode::Char(' ') => {
                    if let Some(i) = active {
                        let mut selected = selected.clone();
                        let p = self.entries[i].path.clone();
                        if !selected.remove(&p) {
                            selected.insert(p.clone());
                        }
                        emit_selection(ctx, selected.into_iter().collect(), Some(p));
                    }
                    None
                }
                KeyCode::Enter => active.map(|i| Msg::Open(self.entries[i].path.clone())),
                KeyCode::Backspace => Some(Msg::Up),
                KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => Some(Msg::Back),
                KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => Some(Msg::Forward),
                KeyCode::Delete => Some(Msg::Modal(Modal::Trash)),
                KeyCode::Esc => {
                    ctx.cancel_pointer();
                    Some(Msg::Escape)
                }
                KeyCode::Left
                | KeyCode::Right
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::PageUp
                | KeyCode::PageDown => {
                    if self.entries.is_empty() {
                        return;
                    }
                    let cols = self.columns(w);
                    let page = ((h / self.row_height()).max(1) as usize) * cols;
                    let index = active.unwrap_or(0);
                    let target = if active.is_none() {
                        0
                    } else {
                        match key.code {
                            KeyCode::Left => index.saturating_sub(1),
                            KeyCode::Right => index + 1,
                            KeyCode::Up => index.saturating_sub(cols),
                            KeyCode::Down => index + cols,
                            KeyCode::Home => 0,
                            KeyCode::End => self.entries.len() - 1,
                            KeyCode::PageUp => index.saturating_sub(page),
                            _ => index + page,
                        }
                    }
                    .min(self.entries.len() - 1);
                    let anchor = ctx
                        .state_mut::<Local>()
                        .anchor
                        .unwrap_or(index)
                        .min(self.entries.len() - 1);
                    if shift {
                        self.select(anchor.min(target)..=anchor.max(target), Some(target), ctx);
                    } else if ctrl {
                        emit_selection(
                            ctx,
                            selected.iter().cloned().collect(),
                            Some(self.entries[target].path.clone()),
                        );
                    } else {
                        self.select([target], Some(target), ctx);
                        ctx.state_mut::<Local>().anchor = Some(target);
                    }
                    let rect = self.rect(target, w);
                    let scroll = ctx.state_mut::<Local>().scroll;
                    ctx.state_mut::<Local>().scroll = if rect.y < scroll + 1 {
                        (rect.y - 1).max(0)
                    } else if rect.y + rect.h >= scroll + h {
                        (rect.y + rect.h - h + 1).min(self.max_scroll(w, h))
                    } else {
                        scroll
                    };
                    ctx.request_paint();
                    None
                }
                _ => return,
            };
            if let Some(msg) = message {
                ctx.emit(msg);
            }
            ctx.set_handled();
        }
    }
}
