//! Folded Harbor: perspective is a game mechanic, not just a camera effect.
use rsille::{
    render::{area::Area, chunk::Chunk},
    tui::prelude::*,
};

mod raster;
mod scene;
mod verify;
mod world;

use scene::{colors, BG, MUTED, TEXT};
use world::{Phase, World};

#[derive(Clone, Copy)]
enum Msg {
    Frame(FrameInfo),
    Orbit(f64, f64),
    Zoom(f64),
    Fold,
    Assist,
    Demo,
    Help,
    Pause,
    Escape,
    Restart,
    Quit,
}

fn app(world: World, inline: bool) -> App<World, Msg> {
    App::new(world)
        .mode(if inline {
            Mode::Inline
        } else {
            Mode::Fullscreen
        })
        .disable_quit_key()
        .with_theme_from(|world| {
            let mut theme = Theme::tokyo_night();
            let (a, _) = colors(world.chapter);
            let base = Style::default().fg(TEXT).bg(BG);
            theme.styles.surface = base;
            theme.styles.surface_elevated = base;
            theme.styles.surface_modal = base.bg(Color::Rgb(12, 23, 40));
            theme.styles.text = base;
            theme.styles.border = base.fg(Color::Rgb(31, 58, 77));
            theme.styles.border_focused = base.fg(a);
            theme.styles.primary_action = base.fg(BG).bg(a).bold();
            theme.styles.primary_action_focused = theme.styles.primary_action;
            theme.styles.secondary_action = base.fg(MUTED);
            theme.styles.secondary_action_focused = base.fg(a).bold();
            theme.styles.scrollbar_track = base.fg(Color::Rgb(25, 42, 61));
            theme.styles.scrollbar_thumb = base.fg(a);
            theme
        })
        .on_frame(Msg::Frame)
        .on_key(KeyCode::Left, || Msg::Orbit(-0.045, 0.0))
        .on_key(KeyCode::Right, || Msg::Orbit(0.045, 0.0))
        .on_key(KeyCode::Up, || Msg::Orbit(0.0, 0.035))
        .on_key(KeyCode::Down, || Msg::Orbit(0.0, -0.035))
        .on_key(KeyCode::Char('a'), || Msg::Orbit(-0.045, 0.0))
        .on_key(KeyCode::Char('d'), || Msg::Orbit(0.045, 0.0))
        .on_key(KeyCode::Char('w'), || Msg::Orbit(0.0, 0.035))
        .on_key(KeyCode::Char('s'), || Msg::Orbit(0.0, -0.035))
        .on_key(KeyCode::Char('+'), || Msg::Zoom(0.1))
        .on_key(KeyCode::Char('='), || Msg::Zoom(0.1))
        .on_key(KeyCode::Char('-'), || Msg::Zoom(-0.1))
        .on_key(KeyCode::Char(' '), || Msg::Fold)
        .on_key(KeyCode::Enter, || Msg::Fold)
        .on_key(KeyCode::Char('h'), || Msg::Assist)
        .on_key(KeyCode::Char('t'), || Msg::Demo)
        .on_key(KeyCode::Char('?'), || Msg::Help)
        .on_key(KeyCode::Char('p'), || Msg::Pause)
        .on_key(KeyCode::Esc, || Msg::Escape)
        .on_key(KeyCode::Char('r'), || Msg::Restart)
        .on_key(KeyCode::Char('q'), || Msg::Quit)
}

fn update(world: &mut World, msg: Msg, ctx: &mut UpdateCtx<Msg>) {
    match msg {
        Msg::Frame(frame) => world.advance(frame.delta.as_secs_f64()),
        Msg::Quit => ctx.exit(),
        Msg::Help => world.help = !world.help,
        Msg::Escape if world.help => world.help = false,
        Msg::Pause | Msg::Escape => {
            if !world.help {
                world.paused = !world.paused;
            }
        }
        Msg::Fold => world.action(),
        _ if world.help => {}
        Msg::Orbit(yaw, pitch) => world.rotate(yaw, pitch),
        Msg::Zoom(delta) => world.zoom(delta),
        Msg::Assist => world.assist(),
        Msg::Demo => world.demo = !world.demo,
        Msg::Restart => world.restart(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::default();
    let mut snapshot_time = 1.8_f64;
    let (mut inline, mut headless, mut snapshot, mut benchmark) = (false, false, None, false);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--demo" => world.demo = true,
            "--inline" => inline = true,
            "--headless" => headless = true,
            "--benchmark" => benchmark = true,
            "--snapshot" => snapshot = Some(args.next().ok_or("--snapshot needs an SVG path")?),
            "--at" => {
                snapshot_time = args.next().ok_or("--at needs seconds")?.parse()?;
                if !snapshot_time.is_finite() || !(0.0..=3600.0).contains(&snapshot_time) {
                    return Err("--at must be finite and between 0 and 3600 seconds".into());
                }
            }
            "--help" | "-h" => {
                println!("folded-harbor [--demo] [--inline] [--headless] [--benchmark] [--snapshot FILE.svg [--at SECONDS]]");
                return Ok(());
            }
            _ => return Err(format!("Unknown argument: {arg}; use --help").into()),
        }
    }
    if headless {
        verify::smoke()?;
    }
    if benchmark {
        verify::benchmark()?;
    }
    if let Some(path) = snapshot.as_ref() {
        verify::snapshot(path, snapshot_time)?;
    }
    if !headless && !benchmark && snapshot.is_none() {
        app(world, inline).update(update).view(view).run()?;
    }
    Ok(())
}

fn view(world: &World) -> impl Widget<Msg> {
    let (a, b) = colors(world.chapter);
    let header = col()
        .gap(0)
        .child(
            visual(label(" F O L D E D   H A R B O R").bold())
                .progress(1.0)
                .effect(
                    VisualEffect::gradient(a, b, GradientDirection::Horizontal)
                        .phase(world.time * 0.025),
                ),
        )
        .child(
            label(format!(
                " IMPOSSIBLE PLACES / {:02}   {}    {}",
                world.chapter + 1,
                world.title(),
                if world.demo {
                    "◉ AUTOPLAY"
                } else {
                    "◌ EXPLORING"
                }
            ))
            .fg(MUTED),
        );
    let action = if world.paused {
        "Resume"
    } else {
        match world.phase {
            Phase::Align if world.ready() => "Fold reality",
            Phase::Align => "Align the rings",
            Phase::Transit => "Crossing...",
            Phase::Complete => "Next chapter",
            Phase::Won => "Begin again",
        }
    };
    let toolbar = row()
        .gap(1)
        .child(
            button(format!("{action} [Space]"))
                .key("fold-control")
                .focusable(false)
                .variant(if world.ready() {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Secondary
                })
                .on_click(|| Msg::Fold),
        )
        .child(
            button("Assist [h]")
                .key("assist")
                .focusable(false)
                .variant(ButtonVariant::Secondary)
                .on_click(|| Msg::Assist),
        )
        .child(
            button("Autoplay [t]")
                .key("demo")
                .focusable(false)
                .variant(ButtonVariant::Secondary)
                .on_click(|| Msg::Demo),
        )
        .child(
            button("Manual [?]")
                .key("manual")
                .focusable(false)
                .variant(ButtonVariant::Secondary)
                .on_click(|| Msg::Help),
        );
    let side=panel().borderless().padding(Padding::uniform(1)).gap(1)
        .child(label("THE OBSERVER").fg(a).bold())
        .child(label("Your viewpoint is the key.\nWhat overlaps can connect.").fg(MUTED))
        .child(divider())
        .child(label(format!("FOLD {:02} / 03",(world.gate+1).min(3))).fg(TEXT).bold())
        .child(label((0..4).map(|i|if i<=world.gate { "●" } else { "○" }).collect::<Vec<_>>().join(" ─ ")).fg(a))
        .child(label(if world.ready() { "PROJECTION LOCKED" } else { "PARALLAX ALIGNMENT" }).fg(if world.ready(){TEXT}else{MUTED}))
        .child(progress_bar(world.alignment()).width(22).variant(ProgressBarVariant::Line).fill_style(Style::default().fg(a)))
        .child(label(format!("{:3.0}%  /  LOCK {:3.0}%",world.alignment()*100.0,world.lock*100.0)).fg(a))
        .child(divider())
        .child(label("01  Rotate the world\n02  Overlap rings A + B\n03  Hold until white\n04  Space to cross").fg(TEXT))
        .child(label("← → / A D   azimuth\n↑ ↓ / W S   elevation\nDrag        orbit\nWheel / ±   zoom\nH           align assist\nP / Esc     pause\nR           restart\nQ           leave").fg(MUTED))
        .child(label(format!("ASSISTS  {:02}\nCHAPTER  {:02} / 03",world.assists,world.chapter+1)).fg(MUTED));
    let caption = if world.paused {
        "TIME IS STILL / Space to resume"
    } else if world.notice > 0.0 {
        "Not yet. Bring the two rings together, then hold still until they turn white."
    } else {
        match world.phase {
            Phase::Align if world.ready() => {
                "The gap has disappeared. Press Space to fold reality."
            }
            Phase::Align => {
                "Two distant doorways. One impossible connection. Rotate until their rings overlap."
            }
            Phase::Transit => "Distance is only a matter of perspective.",
            Phase::Complete => "Three impossible crossings. A new place is waiting.",
            Phase::Won => "You did not move the islands. You changed what was possible.",
        }
    };
    let footer=col().gap(0)
        .child(label(format!(" {caption}")).fg(if world.ready(){a}else{TEXT}))
        .child(label(format!(" drag / arrows orbit   wheel / ± zoom   Space fold   h assist   ? manual   q leave    {:.0}° / {:.0}°",
            world.camera.yaw.to_degrees(),world.camera.pitch.to_degrees())).fg(MUTED));
    let shell = Shell {
        children: vec![
            Box::new(header),
            Box::new(toolbar),
            Box::new(scene_widget(world)),
            Box::new(side),
            Box::new(footer),
        ],
    };
    let base = overlay(shell);
    if world.help {
        base.layer(OverlayLayer::new(visual(dialog().key("manual-dialog").title(" THE OBSERVER'S MANUAL ").gap(1)
            .child(label("These islands are separated in three dimensions.\nRotate your view until the rings marked A and B\noccupy the same place on your screen."))
            .child(label("Hold still for the white lock, then press Space.\nYour traveler crosses the fold. Repeat three times\nin each of three chapters."))
            .child(label("Arrows / WASD / drag: orbit    Wheel / +/-: zoom\nH: assisted alignment         T: auto play\nP / Esc: pause                R: restart\nSpace / Esc / ?: close        Q: leave").fg(MUTED))
            .child(button("Return to the impossible").key("close-manual").on_click(||Msg::Help)))
            .key("manual-enter").enter(VisualEffect::fade_in()))
            .floating(OverlayAnchor::Center).size(58,19).z_index(10)).trap_focus()
    } else if world.paused || matches!(world.phase, Phase::Complete | Phase::Won) {
        let (title, line) = if world.paused {
            (
                " A MOMENT BETWEEN WORLDS ",
                "Your perspective will be here when you return.",
            )
        } else if world.phase == Phase::Won {
            (
                " THE HARBOR REMEMBERS ",
                "Nine folds. Three worlds. One new way of seeing.",
            )
        } else {
            (
                " CHAPTER COMPLETE ",
                "The harbor unfolds another impossible place.",
            )
        };
        base.layer(
            OverlayLayer::new(
                visual(
                    dialog()
                        .key("chapter-dialog")
                        .title(title)
                        .gap(1)
                        .child(label(line).fg(TEXT))
                        .child(
                            label(format!(
                                "{}    /    {} assisted alignments",
                                world.title(),
                                world.assists
                            ))
                            .fg(MUTED),
                        )
                        .child(
                            button(format!("{action} [Space]"))
                                .key("continue")
                                .on_click(|| Msg::Fold),
                        ),
                )
                .key("chapter-enter")
                .enter(VisualEffect::fade_in()),
            )
            .floating(OverlayAnchor::Center)
            .size(58, 10)
            .z_index(5),
        )
        .trap_focus()
    } else {
        base
    }
}

/// Labels and the actionable gate are normal widgets, anchored to 3D coordinates.
fn scene_widget(world: &World) -> Scene {
    let (a, b) = colors(world.chapter);
    let snapshot = world.clone();
    let pixels = visual(
        canvas(move |surface, dimensions| scene::draw(surface, dimensions, &snapshot))
            .key("light-field"),
    )
    .key(format!("chapter-{}", world.chapter))
    .progress(world.reveal)
    .effect(VisualEffect::dissolve().with_seed(world.chapter as u64 + 19));
    let from_label = label(if world.phase == Phase::Align {
        "A / DEPARTURE"
    } else {
        ""
    })
    .fg(a)
    .bg(BG)
    .bold();
    let to_label = label(if world.phase == Phase::Align {
        "B / ARRIVAL"
    } else {
        ""
    })
    .fg(b)
    .bg(BG)
    .bold();
    let gate = button(if world.ready() { "FOLD [Space]" } else { "" })
        .key("gate-action")
        .focusable(false)
        .disabled(!world.ready())
        .on_click(|| Msg::Fold);
    Scene {
        world: world.clone(),
        children: vec![
            Box::new(pixels),
            Box::new(from_label),
            Box::new(to_label),
            Box::new(gate),
        ],
    }
}

#[derive(Default)]
struct Drag {
    last: Option<(u16, u16)>,
}
struct Scene {
    world: World,
    children: Vec<Box<dyn Widget<Msg>>>,
}
impl Widget<Msg> for Scene {
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.record_bounds(chunk.area());
        ctx.render_children(chunk, self);
    }
    fn layout(&self, area: Area, _: &MeasureCtx) -> Layout {
        let (from, to) = self.world.pair();
        let (w, h) = (area.width(), area.height());
        let project = |point| {
            self.world
                .camera
                .project(point, f64::from(w) * 2.0, f64::from(h) * 4.0)
        };
        let float = |point: (f64, f64), offset: i32, width: u16| {
            let x =
                ((point.0 / 2.0).round() as i32 + 2).clamp(0, i32::from(w.saturating_sub(width)));
            let y =
                ((point.1 / 4.0).round() as i32 + offset).clamp(0, i32::from(h.saturating_sub(1)));
            Area::new(
                (area.x() + x as u16, area.y() + y as u16).into(),
                (width.min(w), h.min(1)).into(),
            )
        };
        let from = project(from);
        let to = project(to);
        let label_visible = self.world.phase == Phase::Align && w >= 45 && h >= 15;
        let empty = Area::new(area.pos(), (0, 0).into());
        Layout::new([
            ChildLayout::new(0, area),
            ChildLayout::new(
                1,
                if label_visible {
                    float(from, -2, 14)
                } else {
                    empty
                },
            ),
            ChildLayout::new(
                2,
                if label_visible {
                    float(to, 2, 14)
                } else {
                    empty
                },
            ),
            ChildLayout::new(
                3,
                if self.world.ready() && label_visible {
                    float(from, 4, 17)
                } else {
                    empty
                },
            ),
        ])
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<Msg>) {
        if !matches!(ctx.phase(), EventPhase::Target | EventPhase::Bubble) || ctx.was_handled() {
            return;
        }
        let Event::Mouse(mouse) = event else {
            return;
        };
        let position = ctx.mouse_position(event);
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                ctx.state_mut::<Drag>().last = position;
                ctx.set_handled();
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                let previous = ctx
                    .state_mut::<Drag>()
                    .last
                    .replace((mouse.column, mouse.row));
                if let Some((x, y)) = previous {
                    ctx.emit(Msg::Orbit(
                        (i32::from(mouse.column) - i32::from(x)) as f64 * 0.022,
                        (i32::from(mouse.row) - i32::from(y)) as f64 * -0.045,
                    ));
                }
            }
            MouseEventKind::Up(MouseButton::Left) => ctx.state_mut::<Drag>().last = None,
            MouseEventKind::ScrollUp => ctx.emit(Msg::Zoom(0.1)),
            MouseEventKind::ScrollDown => ctx.emit(Msg::Zoom(-0.1)),
            _ => {}
        }
    }
    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }
    fn constraints(&self) -> Constraints {
        Constraints::fill()
    }
    fn key(&self) -> Option<&str> {
        Some("observatory")
    }
}

struct Shell {
    children: Vec<Box<dyn Widget<Msg>>>,
}
impl Widget<Msg> for Shell {
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        let _ = chunk.fill(
            0,
            0,
            area.width(),
            area.height(),
            ' ',
            Style::default().bg(BG).to_render_style(),
        );
        ctx.render_children(chunk, self);
    }
    fn layout(&self, area: Area, _: &MeasureCtx) -> Layout {
        let (w, h) = (area.width(), area.height());
        let footer = h.min(2);
        let header = h.saturating_sub(footer).min(3);
        let toolbar = h.saturating_sub(footer + header).min(2);
        let top = header + toolbar;
        let body = h.saturating_sub(top + footer);
        let side = if w >= 116 && h >= 34 { 30 } else { 0 };
        let rect = |x, y, width, height| {
            Area::new((area.x() + x, area.y() + y).into(), (width, height).into())
        };
        Layout::new([
            ChildLayout::new(0, rect(0, 0, w, header)),
            ChildLayout::new(1, rect(0, header, w, toolbar)),
            ChildLayout::new(2, rect(0, top, w - side, body)),
            ChildLayout::new(3, rect(w - side, top, side, body)),
            ChildLayout::new(4, rect(0, h - footer, w, footer)),
        ])
    }
    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }
    fn constraints(&self) -> Constraints {
        Constraints::fill()
    }
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::from_constraints(self.constraints()).preferred_height(42)
    }
}
