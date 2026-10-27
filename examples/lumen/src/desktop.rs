//! A tiny window manager built from public overlays, widgets and pointer capture.
use super::{art, ui, Msg, State};
use rsille::render::{area::Area, chunk::Chunk};
use rsille::tui::prelude::*;
use std::time::Duration;

#[derive(Debug)]
pub(super) enum Action {
    Toggle,
    Show(usize),
    Close(usize),
    Fold(usize),
    Raise(usize),
    Arrange,
    Start(usize, (u16, u16), bool),
    Drag((u16, u16)),
    End,
    Nudge(usize, i32, i32, bool),
    Pause,
    Scene,
    Pointer(CanvasPointer),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Window {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub visible: bool,
    pub folded: bool,
}
impl Window {
    fn fit(&mut self, width: u16, height: u16) {
        self.width = self.width.clamp(28, 90).min(width.max(1));
        self.height = self
            .height
            .clamp(11, 32)
            .min(height.saturating_sub(7).max(1));
        self.x = self.x.min(width.saturating_sub(self.width));
        self.y = self.y.min(height.saturating_sub(7 + self.height));
    }
}
#[derive(Clone, Copy)]
struct Gesture {
    id: usize,
    origin: (u16, u16),
    window: Window,
    resize: bool,
}

pub(super) struct Desktop {
    pub enabled: bool,
    pub windows: [Window; 2],
    pub front: usize,
    pub paused: bool,
    pub scene: usize,
    pub morph: f64,
    pub time: f64,
    pub pointer: Option<(f64, f64)>,
    pub pulse: f64,
    gesture: Option<Gesture>,
}
impl Default for Desktop {
    fn default() -> Self {
        Self {
            enabled: false,
            windows: [
                Window {
                    x: 30,
                    y: 11,
                    width: 43,
                    height: 19,
                    visible: true,
                    folded: false,
                },
                Window {
                    x: 68,
                    y: 21,
                    width: 38,
                    height: 14,
                    visible: true,
                    folded: false,
                },
            ],
            front: 1,
            paused: false,
            scene: 0,
            morph: 0.0,
            time: 0.0,
            pointer: None,
            pulse: -10.0,
            gesture: None,
        }
    }
}
impl Desktop {
    pub fn open(&mut self, width: u16, height: u16) {
        self.enabled = true;
        if self.windows.iter().all(|w| !w.visible) {
            self.windows.iter_mut().for_each(|w| w.visible = true);
        }
        self.fit(width, height);
    }
    pub fn fit(&mut self, width: u16, height: u16) {
        self.cancel_drag();
        for w in &mut self.windows {
            w.fit(width, height);
        }
    }
    pub fn cancel_drag(&mut self) {
        self.gesture = None;
    }
    pub fn update(&mut self, action: Action, width: u16, height: u16) {
        match action {
            Action::Toggle => {
                self.cancel_drag();
                if self.enabled {
                    self.enabled = false;
                } else {
                    self.open(width, height);
                }
            }
            Action::Show(id) => {
                self.enabled = true;
                self.windows[id].visible = true;
                self.windows[id].folded = false;
                self.front = id;
            }
            Action::Close(id) => {
                self.windows[id].visible = false;
                self.cancel_drag();
            }
            Action::Fold(id) => {
                self.windows[id].folded = !self.windows[id].folded;
                self.cancel_drag();
            }
            Action::Raise(id) => self.front = id,
            Action::Arrange => {
                let was_enabled = self.enabled;
                let time = self.time;
                let paused = self.paused;
                let scene = self.scene;
                let morph = self.morph;
                *self = Self {
                    enabled: was_enabled,
                    time,
                    paused,
                    scene,
                    morph,
                    ..Self::default()
                };
                if width < 108 {
                    self.windows[0].x = 3;
                    self.windows[0].y = 8;
                    self.windows[1].x = width.saturating_sub(41);
                    self.windows[1].y = 15;
                }
                self.fit(width, height);
            }
            Action::Start(id, origin, resize) => {
                self.front = id;
                self.gesture = Some(Gesture {
                    id,
                    origin,
                    window: self.windows[id],
                    resize,
                });
            }
            Action::Drag(position) => {
                if let Some(g) = self.gesture {
                    let dx = i32::from(position.0) - i32::from(g.origin.0);
                    let dy = i32::from(position.1) - i32::from(g.origin.1);
                    self.windows[g.id] = g.window;
                    self.nudge(g.id, dx, dy, g.resize, width, height);
                }
            }
            Action::End => {
                if let Some(g) = self.gesture.take() {
                    let w = &mut self.windows[g.id];
                    // Magnetic edges, applied only on release so dragging stays direct.
                    if !g.resize {
                        if w.x < 4 {
                            w.x = 0;
                        }
                        let right = width.saturating_sub(w.width);
                        if right.saturating_sub(w.x) < 4 {
                            w.x = right;
                        }
                        if w.y < 3 {
                            w.y = 0;
                        }
                    }
                }
            }
            Action::Nudge(id, dx, dy, resize) => {
                self.front = id;
                self.nudge(id, dx, dy, resize, width, height);
            }
            Action::Pause => self.paused = !self.paused,
            Action::Scene => self.scene = (self.scene + 1) % 2,
            Action::Pointer(pointer) => {
                self.pointer = Some((pointer.position.x, pointer.position.y));
                if matches!(pointer.kind, MouseEventKind::Down(MouseButton::Left)) {
                    self.pulse = self.time;
                }
            }
        }
    }
    fn nudge(&mut self, id: usize, dx: i32, dy: i32, resize: bool, width: u16, height: u16) {
        let w = &mut self.windows[id];
        let add = |v: u16, delta: i32| (i32::from(v) + delta).clamp(0, i32::from(u16::MAX)) as u16;
        if resize {
            w.width = add(w.width, dx);
            w.height = add(w.height, dy);
        } else {
            w.x = add(w.x, dx);
            w.y = add(w.y, dy);
        }
        w.fit(width, height);
    }
}

pub(super) fn page(s: &State, p: ui::Palette) -> impl Widget<Msg> {
    let launch: Box<dyn Widget<Msg>> = if s.desk.enabled {
        Box::new(spacer())
    } else {
        Box::new(
            row()
                .gap(2)
                .child(
                    button("Launch floating desk  ↗")
                        .key("launch-desk")
                        .on_click(|| Msg::Desk(Action::Toggle)),
                )
                .child(label("F5 anywhere · click the field to send a pulse").fg(p.muted)),
        )
    };
    grid()
        .columns("1fr")
        .rows("5 1fr 3")
        .gap(0)
        .child(
            col()
                .gap(1)
                .child(label("S P A T I A L   D E S K").fg(p.mint))
                .child(
                    label("A little freedom. Another dimension.")
                        .fg(p.text)
                        .bold(),
                )
                .child(
                    label("Drag a title. Stretch a corner. Make room for possibility.").fg(p.muted),
                ),
        )
        .child(art::space(s, p, true).key("desk-backdrop"))
        .child(launch)
}

pub(super) fn layers(mut root: Overlay<Msg>, s: &State, p: ui::Palette) -> Overlay<Msg> {
    if !s.desk.enabled {
        return root;
    }
    // Keep insertion order and keyed identities stable; only the paint order changes.
    for id in 0..2 {
        let w = s.desk.windows[id];
        if !w.visible {
            continue;
        }
        let folded = w.folded || w.height < 11;
        let active = s.desk.front == id;
        let accent = if active { p.mint } else { p.violet };
        let body: Box<dyn Widget<Msg>> = if id == 0 {
            Box::new(
                grid()
                    .columns("1fr")
                    .rows("1fr 1 3")
                    .gap(0)
                    .child(art::space(s, p, false).key("lens-canvas"))
                    .child(
                        label(if s.desk.scene == 0 {
                            "  01 / TORUS   ·   DEPTH + LIGHT"
                        } else {
                            "  02 / HELIX   ·   DEPTH + LIGHT"
                        })
                        .fg(p.muted),
                    )
                    .child(
                        row()
                            .gap(1)
                            .padding(Padding::new(0, 1, 0, 1))
                            .child(
                                button(if s.desk.paused { "Resume" } else { "Pause" })
                                    .key("desk-pause")
                                    .variant(ButtonVariant::Secondary)
                                    .on_click(|| Msg::Desk(Action::Pause)),
                            )
                            .child(
                                button("Morph ↻")
                                    .key("desk-scene")
                                    .variant(ButtonVariant::Ghost)
                                    .on_click(|| Msg::Desk(Action::Scene)),
                            ),
                    ),
            )
        } else {
            Box::new(
                grid()
                    .columns("1fr")
                    .rows("1 1fr 1 3")
                    .gap(0)
                    .child(label("  SIGNAL STREAM  /  SIMULATED").fg(p.muted))
                    .child(art::desk_signal(s, p))
                    .child(
                        label(format!(
                            "  {} session runs  ·  {}",
                            s.completed,
                            if s.running.is_some() {
                                "FLOW ACTIVE"
                            } else {
                                "READY"
                            }
                        ))
                        .fg(p.mint),
                    )
                    .child(
                        row().padding(Padding::new(0, 1, 0, 1)).child(
                            button(if s.running.is_some() {
                                "Running..."
                            } else {
                                "Run selected flow >"
                            })
                            .key("desk-run")
                            .disabled(s.running.is_some())
                            .on_click(|| Msg::Run),
                        ),
                    ),
            )
        };
        let chrome = WindowFrame {
            id,
            key: format!("floating-{id}"),
            accent,
            p,
            children: {
                let mut children: Vec<Box<dyn Widget<Msg>>> = vec![Box::new(TitleBar {
                    id,
                    folded,
                    active,
                    p,
                })];
                if !folded {
                    children.push(body);
                    children.push(Box::new(ResizeGrip { id, p }));
                }
                children
            },
            folded,
        };
        // Never ease a captured drag: the title must stay attached to the pointer.
        let spec = AnimationSpec::new(
            Duration::from_millis(if s.quiet || s.desk.gesture.is_some() {
                0
            } else {
                240
            }),
            Easing::EaseOut,
        );
        root = root.layer(
            OverlayLayer::new(
                visual(chrome)
                    .key(format!("floating-motion-{id}"))
                    .layout_transition(if s.quiet || s.desk.gesture.is_some() {
                        LayoutTransition::none()
                    } else {
                        LayoutTransition::size_and_position(spec)
                    })
                    .hit_test(HitTestMode::Display)
                    .animation(spec)
                    .enter(VisualEffect::fade_in()),
            )
            .floating(OverlayAnchor::TopLeft)
            .offset(w.x as i16, w.y as i16)
            .size(w.width, if folded { 3.min(w.height) } else { w.height })
            .z_index(if active { 2 } else { 1 }),
        );
    }
    root.layer(
        OverlayLayer::new(
            panel()
                .key("desk-dock")
                .padding(Padding::new(0, 1, 0, 1))
                .border(BorderStyle::Rounded)
                .style(p.surface())
                .gap(1)
                .child(
                    row()
                        .gap(1)
                        .child(
                            button("◈ Lens")
                                .key("show-lens")
                                .variant(ButtonVariant::Ghost)
                                .on_click(|| Msg::Desk(Action::Show(0))),
                        )
                        .child(
                            button("≋ Signal")
                                .key("show-signal")
                                .variant(ButtonVariant::Ghost)
                                .on_click(|| Msg::Desk(Action::Show(1))),
                        )
                        .child(
                            button("Arrange")
                                .key("arrange-desk")
                                .variant(ButtonVariant::Ghost)
                                .on_click(|| Msg::Desk(Action::Arrange)),
                        )
                        .child(
                            button("× Desk")
                                .key("hide-desk")
                                .variant(ButtonVariant::Ghost)
                                .on_click(|| Msg::Desk(Action::Toggle)),
                        ),
                ),
        )
        .floating(OverlayAnchor::Bottom)
        .offset(0, -1)
        .size(55.min(s.width), 5)
        .z_index(3),
    )
}

struct WindowFrame {
    id: usize,
    key: String,
    accent: Color,
    p: ui::Palette,
    children: Vec<Box<dyn Widget<Msg>>>,
    folded: bool,
}
impl Widget<Msg> for WindowFrame {
    fn key(&self) -> Option<&str> {
        Some(&self.key)
    }
    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }
    fn constraints(&self) -> Constraints {
        Constraints::min(1, 1)
    }
    fn layout(&self, area: Area, _: &MeasureCtx) -> Layout {
        let w = area.width().saturating_sub(2);
        let h = area.height().saturating_sub(2);
        let x = area.x() + 1;
        let y = area.y() + 1;
        let mut children = vec![ChildLayout::new(
            0,
            Area::new((x, y).into(), (w, 1.min(h)).into()),
        )];
        if !self.folded && h > 2 {
            children.push(ChildLayout::new(
                1,
                Area::new((x, y + 2).into(), (w, h - 3).into()),
            ));
            children.push(ChildLayout::new(
                2,
                Area::new((x, y + h - 1).into(), (w, 1).into()),
            ));
        }
        Layout::new(children)
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let (w, h) = (chunk.area().width(), chunk.area().height());
        let style = self.p.surface().to_render_style();
        let line = Style::default()
            .bg(self.p.card)
            .fg(self.accent)
            .to_render_style();
        let _ = chunk.fill(0, 0, w, h, ' ', style);
        if w > 1 && h > 1 {
            for x in 1..w - 1 {
                let _ = chunk.set_string(x, 0, "─", line);
                let _ = chunk.set_string(x, h - 1, "─", line);
            }
            for y in 1..h - 1 {
                let _ = chunk.set_string(0, y, "│", line);
                let _ = chunk.set_string(w - 1, y, "│", line);
            }
            for (x, y, c) in [
                (0, 0, "╭"),
                (w - 1, 0, "╮"),
                (0, h - 1, "╰"),
                (w - 1, h - 1, "╯"),
            ] {
                let _ = chunk.set_string(x, y, c, line);
            }
        }
        ctx.render_children(chunk, self);
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<Msg>) {
        if ctx.phase() == EventPhase::Capture
            && matches!(
                event,
                Event::Key(KeyEvent {
                    code: KeyCode::Enter
                        | KeyCode::Char(' ')
                        | KeyCode::Left
                        | KeyCode::Right
                        | KeyCode::Up
                        | KeyCode::Down
                        | KeyCode::Delete,
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    ..
                })
            )
        {
            ctx.emit(Msg::Desk(Action::Raise(self.id)));
        }
        if matches!(
            event,
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                ..
            })
        ) {
            if matches!(ctx.phase(), EventPhase::Capture | EventPhase::Target) {
                ctx.emit(Msg::Desk(Action::Raise(self.id)));
            }
            if ctx.phase() == EventPhase::Target {
                ctx.stop_propagation();
            }
        }
    }
}

struct TitleBar {
    id: usize,
    folded: bool,
    active: bool,
    p: ui::Palette,
}
impl Widget<Msg> for TitleBar {
    fn constraints(&self) -> Constraints {
        Constraints::min(1, 1)
    }
    fn key(&self) -> Option<&str> {
        Some(if self.id == 0 {
            "lens-title"
        } else {
            "signal-title"
        })
    }
    fn focus_config(&self) -> FocusConfig {
        FocusConfig::Leaf
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let w = chunk.area().width();
        let style = Style::default()
            .bg(self.p.raised)
            .fg(if self.active || ctx.is_focused() {
                self.p.mint
            } else {
                self.p.muted
            })
            .bold()
            .to_render_style();
        let _ = chunk.fill(0, 0, w, 1, ' ', style);
        let name = match (self.id, w < 26) {
            (0, true) => "◈ Lens",
            (_, true) => "≋ Signal",
            (0, false) => "◈  Spatial lens",
            _ => "≋  Live signal",
        };
        let _ = chunk.set_string(1, 0, name, style);
        if w >= 7 {
            let _ = chunk.set_string(
                w - 7,
                0,
                if self.folded {
                    " +   × "
                } else {
                    " −   × "
                },
                style,
            );
        }
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<Msg>) {
        if ctx.phase() != EventPhase::Target {
            return;
        }
        if let Some(action) = gesture_event(event, ctx) {
            ctx.emit(Msg::Desk(action));
            return;
        }
        match event {
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                ..
            }) => {
                if let (Some((x, _)), Some(bounds), Some(position)) = (
                    ctx.local_mouse_position(event),
                    ctx.bounds(),
                    ctx.layout_mouse_position(event),
                ) {
                    let action = if x >= bounds.width().saturating_sub(3) {
                        Action::Close(self.id)
                    } else if x >= bounds.width().saturating_sub(7) {
                        Action::Fold(self.id)
                    } else {
                        Action::Start(self.id, position, false)
                    };
                    ctx.emit(Msg::Desk(action));
                }
            }
            Event::Key(k) if k.kind != KeyEventKind::Release => {
                let (dx, dy) = match k.code {
                    KeyCode::Left => (-2, 0),
                    KeyCode::Right => (2, 0),
                    KeyCode::Up => (0, -1),
                    KeyCode::Down => (0, 1),
                    KeyCode::Enter => {
                        ctx.emit(Msg::Desk(Action::Fold(self.id)));
                        return;
                    }
                    KeyCode::Delete => {
                        ctx.emit(Msg::Desk(Action::Close(self.id)));
                        return;
                    }
                    _ => return,
                };
                ctx.emit(Msg::Desk(Action::Nudge(
                    self.id,
                    dx,
                    dy,
                    k.modifiers.contains(KeyModifiers::SHIFT),
                )));
            }
            _ => {}
        }
    }
}
struct ResizeGrip {
    id: usize,
    p: ui::Palette,
}
impl Widget<Msg> for ResizeGrip {
    fn constraints(&self) -> Constraints {
        Constraints::min(1, 1)
    }
    fn key(&self) -> Option<&str> {
        Some(if self.id == 0 {
            "lens-resize"
        } else {
            "signal-resize"
        })
    }
    fn focus_config(&self) -> FocusConfig {
        FocusConfig::Leaf
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let w = chunk.area().width();
        let style = Style::default()
            .fg(if ctx.is_hovered() || ctx.is_focused() {
                self.p.mint
            } else {
                self.p.muted
            })
            .bg(self.p.card)
            .to_render_style();
        let _ = chunk.set_string(1, 0, "drag title · resize ↘", style);
        if w > 2 {
            let _ = chunk.set_string(w - 2, 0, "◢", style);
        }
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<Msg>) {
        if ctx.phase() != EventPhase::Target {
            return;
        }
        if let Some(action) = gesture_event(event, ctx) {
            ctx.emit(Msg::Desk(action));
            return;
        }
        if matches!(
            event,
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                ..
            })
        ) {
            if let Some(pos) = ctx.layout_mouse_position(event) {
                ctx.emit(Msg::Desk(Action::Start(self.id, pos, true)));
            }
        }
        if let Event::Key(k) = event {
            if k.kind == KeyEventKind::Release {
                return;
            }
            let (dx, dy) = match k.code {
                KeyCode::Left => (-2, 0),
                KeyCode::Right => (2, 0),
                KeyCode::Up => (0, -1),
                KeyCode::Down => (0, 1),
                _ => return,
            };
            ctx.emit(Msg::Desk(Action::Nudge(self.id, dx, dy, true)));
        }
    }
}
fn gesture_event(event: &Event, ctx: &EventCtx<Msg>) -> Option<Action> {
    match event {
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            ..
        }) => ctx.layout_mouse_position(event).map(Action::Drag),
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            ..
        }) => Some(Action::End),
        _ => None,
    }
}
