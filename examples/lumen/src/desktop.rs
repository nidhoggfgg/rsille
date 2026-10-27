//! Lumen's window configuration and content; interaction lives in the framework.
use super::{art, ui, Msg, State};
use rsille::tui::prelude::*;

#[derive(Debug)]
pub(super) enum Action {
    Toggle,
    Show(usize),
    Close(usize),
    Arrange,
    Rect(usize, WindowRect),
    Visibility(usize, WindowVisibility),
    Active(Option<String>),
    Pause,
    Scene,
    Pointer(CanvasPointer),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WindowConfig {
    pub rect: WindowRect,
    pub visibility: WindowVisibility,
    pub open: bool,
}
pub(super) struct Desktop {
    pub enabled: bool,
    pub windows: [WindowConfig; 2],
    pub active: Option<String>,
    pub paused: bool,
    pub scene: usize,
    pub morph: f64,
    pub time: f64,
    pub pointer: Option<(f64, f64)>,
    pub pulse: f64,
}
impl Default for Desktop {
    fn default() -> Self {
        Self {
            enabled: false,
            windows: [
                WindowConfig {
                    rect: WindowRect::new(30, 11, 43, 19),
                    visibility: WindowVisibility::Expanded,
                    open: true,
                },
                WindowConfig {
                    rect: WindowRect::new(68, 21, 38, 14),
                    visibility: WindowVisibility::Expanded,
                    open: true,
                },
            ],
            active: Some("floating-1".into()),
            paused: false,
            scene: 0,
            morph: 0.0,
            time: 0.0,
            pointer: None,
            pulse: -10.0,
        }
    }
}
impl Desktop {
    pub fn open(&mut self, width: u16, height: u16) {
        self.enabled = true;
        self.fit(width, height);
    }
    pub fn fit(&mut self, width: u16, height: u16) {
        for window in &mut self.windows {
            window.rect =
                window
                    .rect
                    .constrained((width, height.saturating_sub(7)), (28, 11), (90, 32));
        }
    }
    pub fn update(&mut self, action: Action, width: u16, height: u16) {
        match action {
            Action::Toggle => {
                if self.enabled {
                    self.enabled = false;
                } else {
                    self.open(width, height);
                }
            }
            Action::Show(id) => {
                self.enabled = true;
                self.windows[id].open = true;
                self.windows[id].visibility = WindowVisibility::Expanded;
                self.active = Some(format!("floating-{id}"));
            }
            Action::Close(id) => self.windows[id].open = false,
            Action::Rect(id, rect) => self.windows[id].rect = rect,
            Action::Visibility(id, visibility) => self.windows[id].visibility = visibility,
            Action::Active(active) => self.active = active,
            Action::Arrange => {
                let defaults = Self::default();
                self.windows = defaults.windows;
                self.active = defaults.active;
                if width < 108 {
                    self.windows[0].rect.x = 3;
                    self.windows[0].rect.y = 8;
                    self.windows[1].rect.x = i32::from(width.saturating_sub(41));
                    self.windows[1].rect.y = 15;
                }
                self.fit(width, height);
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

pub(super) fn layers(base: impl IntoWidget<Msg>, s: &State, p: ui::Palette) -> Overlay<Msg> {
    let mut host = window_host(base)
        .key("studio-windows")
        .padding(Padding::new(0, 0, 7, 0))
        .snap_distance(3)
        .active_window(s.desk.active.clone())
        .on_active_window_change(|id| Msg::Desk(Action::Active(id)));
    for id in 0..2 {
        let window = s.desk.windows[id];
        if !window.open {
            continue;
        }
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
        host = host.window(
            floating_window(body)
                .key(format!("floating-{id}"))
                .title(if id == 0 {
                    "◈  Spatial lens"
                } else {
                    "≋  Live signal"
                })
                .title_key(if id == 0 {
                    "lens-title"
                } else {
                    "signal-title"
                })
                .resize_key(if id == 0 {
                    "lens-resize"
                } else {
                    "signal-resize"
                })
                .rect(window.rect)
                .visibility(if s.desk.enabled {
                    window.visibility
                } else {
                    WindowVisibility::Hidden
                })
                .on_rect_change(move |rect| Msg::Desk(Action::Rect(id, rect)))
                .on_visibility_change(move |visibility| {
                    Msg::Desk(Action::Visibility(id, visibility))
                })
                .on_close(move || Msg::Desk(Action::Close(id)))
                .hideable(true)
                .min_size(28, 11)
                .max_size(90, 32)
                .style(p.surface())
                .active_style(Style::default().bg(p.card).fg(p.mint))
                .layout_transition(if s.quiet {
                    LayoutTransition::none()
                } else {
                    LayoutTransition::size_and_position(AnimationSpec::new(
                        std::time::Duration::from_millis(240),
                        Easing::EaseOut,
                    ))
                }),
        );
    }
    let root = overlay(host);
    if !s.desk.enabled {
        return root;
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
