//! A terminal observatory made entirely with rsille's public API.
use std::fmt::Write as _;
use std::time::{Duration, Instant};

use rsille::render::{area::Area, buffer::Buffer, chunk::Chunk};
use rsille::tui::{prelude::*, runtime::ManualClock};

mod raster;
mod scenes;

use scenes::draw_scene;

const BG: Color = Color::Rgb(7, 12, 23);
const SURFACE: Color = Color::Rgb(11, 20, 34);
const TEXT: Color = Color::Rgb(214, 231, 243);
const MUTED: Color = Color::Rgb(109, 136, 158);
const INLINE_WIDTH: u16 = 140;
const INLINE_HEIGHT: u16 = 40;
const SCENES: [&str; 6] = [
    "ORBITAL / rings of a distant sun",
    "WARP / into the photon tunnel",
    "TERRAIN / neon valley flyover",
    "GALAXY / a billion distant suns",
    "TORUS / liquid chrome reactor",
    "AURORA / beneath the solar wind",
];

#[derive(Clone, Copy, Debug)]
enum Msg {
    Frame(FrameInfo),
    Scene(usize),
    Pause,
    Palette,
    Speed,
    Help,
    Quit,
}

#[derive(Default)]
struct State {
    time: f64,
    reveal: f64,
    scene: usize,
    palette: usize,
    speed: usize,
    paused: bool,
    help: bool,
}

impl State {
    fn colors(&self) -> (Color, Color) {
        match self.palette {
            1 => (Color::Rgb(255, 182, 94), Color::Rgb(255, 91, 142)),
            2 => (Color::Rgb(139, 245, 166), Color::Rgb(105, 172, 255)),
            _ => (Color::Rgb(78, 224, 242), Color::Rgb(180, 126, 255)),
        }
    }

    fn rate(&self) -> f64 {
        [1.0, 2.0, 0.5][self.speed]
    }
}

fn app(state: State) -> App<State, Msg> {
    App::new(state)
        .mode(Mode::Inline)
        .with_theme_from(|state| {
            let mut theme = Theme::tokyo_night();
            let (accent, _) = state.colors();
            theme.styles.surface = Style::default().fg(TEXT).bg(BG);
            theme.styles.surface_elevated = Style::default().fg(TEXT).bg(SURFACE);
            theme.styles.border = Style::default().fg(Color::Rgb(37, 69, 89));
            theme.styles.primary_action = Style::default().fg(BG).bg(accent).bold();
            theme
        })
        .on_frame(Msg::Frame)
        .on_key(KeyCode::Char('1'), || Msg::Scene(0))
        .on_key(KeyCode::Char('2'), || Msg::Scene(1))
        .on_key(KeyCode::Char('3'), || Msg::Scene(2))
        .on_key(KeyCode::Char('4'), || Msg::Scene(3))
        .on_key(KeyCode::Char('5'), || Msg::Scene(4))
        .on_key(KeyCode::Char('6'), || Msg::Scene(5))
        .on_key(KeyCode::Char('p'), || Msg::Pause)
        .on_key(KeyCode::Char('c'), || Msg::Palette)
        .on_key(KeyCode::Char('s'), || Msg::Speed)
        .on_key(KeyCode::Char('?'), || Msg::Help)
        .on_key(KeyCode::Char('q'), || Msg::Quit)
}

fn update(state: &mut State, message: Msg, ctx: &mut UpdateCtx<Msg>) {
    match message {
        Msg::Frame(frame) => {
            let delta = frame.delta.as_secs_f64().min(0.1);
            if !state.paused {
                state.time += delta * state.rate();
            }
            state.reveal = (state.reveal + delta / 0.65).min(1.0);
        }
        Msg::Scene(scene) => {
            state.scene = scene;
            state.reveal = 0.0;
        }
        Msg::Pause => state.paused = !state.paused,
        Msg::Palette => state.palette = (state.palette + 1) % 3,
        Msg::Speed => state.speed = (state.speed + 1) % 3,
        Msg::Help => state.help = !state.help,
        Msg::Quit => ctx.exit(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => {
            app(State::default()).update(update).view(view).run()?;
        }
        [flag] if flag == "--headless" => smoke()?,
        [flag] if flag == "--benchmark" => benchmark()?,
        [flag, path] if flag == "--snapshot" => snapshot(path, 0, 12.0)?,
        [flag, path, scene] if flag == "--snapshot" => {
            snapshot(path, parse_scene(scene)?, 12.0)?;
        }
        [flag, path, scene, time] if flag == "--snapshot" => {
            let time: f64 = time.parse()?;
            if !time.is_finite() || time < 0.0 {
                return Err("Time must be a finite, nonnegative number of seconds".into());
            }
            snapshot(path, parse_scene(scene)?, time)?;
        }
        _ => return Err("Usage: neon-observatory [--headless | --benchmark | --snapshot FILE.svg [1-6] [SECONDS]]".into()),
    }
    Ok(())
}

fn parse_scene(scene: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let scene: usize = scene.parse()?;
    if !(1..=SCENES.len()).contains(&scene) {
        return Err("Scene must be between 1 and 6".into());
    }
    Ok(scene - 1)
}

fn snapshot(path: &str, scene: usize, time: f64) -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = app(State {
        time,
        reveal: 1.0,
        scene,
        ..State::default()
    })
    .with_clock(ManualClock::default())
    .update(update)
    .view(view)
    .build()
    .size(INLINE_WIDTH, INLINE_HEIGHT);
    runtime.step(&[])?;
    std::fs::write(path, svg(runtime.frame()))?;
    println!("Saved actual rsille frame to {path}");
    Ok(())
}

/// Measure the real application frame path, excluding terminal I/O and SVG export.
fn benchmark() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{INLINE_WIDTH}x{INLINE_HEIGHT}, 90 frames per scene; CPU update + render, no terminal I/O"
    );
    for (scene, name) in SCENES.iter().enumerate() {
        let clock = ManualClock::default();
        let mut runtime = app(State {
            scene,
            reveal: 1.0,
            ..State::default()
        })
        .with_clock(clock.clone())
        .update(update)
        .view(view)
        .build()
        .size(INLINE_WIDTH, INLINE_HEIGHT);
        let mut times = Vec::with_capacity(90);
        for frame in 0..100 {
            clock.advance(Duration::from_secs_f64(1.0 / 60.0));
            let start = Instant::now();
            runtime.step(&[])?;
            if frame >= 10 {
                times.push(start.elapsed().as_secs_f64() * 1000.0);
            }
        }
        let mean = times.iter().sum::<f64>() / times.len() as f64;
        times.sort_by(f64::total_cmp);
        println!(
            "{:<8} mean {mean:6.2} ms / p95 {:6.2} ms",
            name.split(" /").next().unwrap_or(name),
            times[85]
        );
    }
    Ok(())
}

fn view(state: &State) -> impl Widget<Msg> {
    let (accent, secondary) = state.colors();
    let header = col()
        .gap(0)
        .child(
            visual(label(" N E O N   /   O B S E R V A T O R Y").bold())
                .progress(1.0)
                .effect(
                    VisualEffect::gradient(accent, secondary, GradientDirection::Horizontal)
                        .phase(state.time * 0.035),
                ),
        )
        .child(label(" A small window into an imagined universe.   /   RSILLE v3").fg(MUTED));
    let mut toolbar = row().gap(1);
    for (index, title) in [
        "1 Orbit",
        "2 Warp",
        "3 Terrain",
        "4 Galaxy",
        "5 Torus",
        "6 Aurora",
    ]
    .into_iter()
    .enumerate()
    {
        toolbar = toolbar.child(
            button(title)
                .key(format!("scene-{index}"))
                .focusable(false)
                .variant(if state.scene == index {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Secondary
                })
                .on_click(move || Msg::Scene(index)),
        );
    }
    let controls = row()
        .gap(1)
        .child(
            button(if state.paused { "Resume" } else { "Pause" })
                .key("pause")
                .variant(ButtonVariant::Secondary)
                .on_click(|| Msg::Pause),
        )
        .child(
            button("Color")
                .key("color")
                .variant(ButtonVariant::Secondary)
                .on_click(|| Msg::Palette),
        )
        .child(
            button(format!("{}x", state.rate()))
                .key("speed")
                .variant(ButtonVariant::Secondary)
                .on_click(|| Msg::Speed),
        );
    let toolbar = col().gap(1).child(toolbar).child(controls);
    let time = state.time;
    let scene = state.scene;
    let scene = panel()
        .title(format!(" {} ", SCENES[scene]))
        .border(BorderStyle::Rounded)
        .padding(Padding::uniform(0))
        .style(Style::default().bg(BG))
        .child(
            visual(
                canvas(move |surface, dimensions| {
                    draw_scene(surface, dimensions, scene, time, accent, secondary);
                })
                .key("universe"),
            )
            .key("scene-reveal")
            .progress(state.reveal)
            .effect(VisualEffect::reveal(WipeDirection::LeftToRight).softness(0.12)),
        );
    let footer = label(format!(
        " {}  /  {:.1}x    1-6 scene   p pause   c color   s speed   ? help   q quit",
        if state.paused { "PAUSED" } else { "LIVE" },
        state.rate()
    ))
    .fg(MUTED);
    let dashboard = Dashboard {
        children: vec![
            Box::new(header),
            Box::new(toolbar),
            Box::new(scene),
            Box::new(footer),
        ],
    };
    let base = overlay(dashboard);
    let content = if state.help {
        base.layer(
            OverlayLayer::new(
                visual(
                    dialog()
                        .key("help")
                        .title(" FLIGHT MANUAL ")
                        .gap(1)
                        .child(label("1 - 6         Switch the universe"))
                        .child(label("p             Pause / resume simulation"))
                        .child(label("c / s         Color palette / time scale"))
                        .child(label("Tab + Enter   Pause / color / speed controls"))
                        .child(label("?             Close this manual"))
                        .child(label("q / Esc       Exit the observatory"))
                        .child(
                            button("Return to flight")
                                .key("close-help")
                                .on_click(|| Msg::Help),
                        ),
                )
                .key("help-visual")
                .enter(VisualEffect::fade_in()),
            )
            .floating(OverlayAnchor::Center)
            .size(49, 19)
            .z_index(10),
        )
        .trap_focus()
    } else {
        base
    };
    Viewport {
        children: vec![Box::new(content)],
    }
}

/// Keep the entire UI, including floating dialogs, in a compact inline area.
struct Viewport {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl Widget<Msg> for Viewport {
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.render_children(chunk, self);
    }

    fn layout(&self, area: Area, _ctx: &MeasureCtx) -> Layout {
        Layout::new([ChildLayout::new(
            0,
            Area::new(
                area.pos(),
                (
                    area.width().min(INLINE_WIDTH),
                    area.height().min(INLINE_HEIGHT),
                )
                    .into(),
            ),
        )])
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn constraints(&self) -> Constraints {
        Constraints {
            max_width: Some(INLINE_WIDTH),
            max_height: Some(INLINE_HEIGHT),
            ..Constraints::fill()
        }
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::from_constraints(self.constraints())
            .preferred_width(INLINE_WIDTH)
            .preferred_height(INLINE_HEIGHT)
    }
}

/// A small application-specific responsive container; children remain normal widgets.
struct Dashboard {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl Widget<Msg> for Dashboard {
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

    fn layout(&self, area: Area, _ctx: &MeasureCtx) -> Layout {
        let width = area.width();
        let height = area.height();
        let rect = |x, y, w, h| Area::new((area.x() + x, area.y() + y).into(), (w, h).into());
        // Reserve the footer first, even in very short terminals.
        let header = height.saturating_sub(1).min(3);
        let toolbar = height.saturating_sub(header + 1).min(3);
        let top = header + toolbar;
        let body = height.saturating_sub(top + 1);
        Layout::new([
            ChildLayout::new(0, rect(0, 0, width, header)),
            ChildLayout::new(1, rect(0, header, width, toolbar)),
            ChildLayout::new(2, rect(0, top, width, body)),
            ChildLayout::new(3, rect(0, height.saturating_sub(1), width, height.min(1))),
        ])
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }
    fn constraints(&self) -> Constraints {
        Constraints {
            max_height: Some(INLINE_HEIGHT),
            ..Constraints::fill()
        }
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::from_constraints(self.constraints()).preferred_height(INLINE_HEIGHT)
    }
}

fn frame_text(buffer: &Buffer) -> String {
    let mut result = String::new();
    for row in buffer.content().chunks(buffer.size().width.max(1) as usize) {
        for cell in row {
            result.push_str(&cell.content.symbol());
        }
        result.push('\n');
    }
    result
}

fn key(c: char) -> Event {
    Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()))
}

fn smoke() -> Result<(), Box<dyn std::error::Error>> {
    let clock = ManualClock::default();
    let mut runtime = app(State::default())
        .with_clock(clock.clone())
        .update(update)
        .view(view)
        .build()
        .size(INLINE_WIDTH + 20, INLINE_HEIGHT);
    runtime.step(&[])?;
    for _ in 0..30 {
        clock.advance(Duration::from_millis(33));
        runtime.step(&[])?;
    }
    assert!(runtime.state().time > 0.0);
    assert!(frame_text(runtime.frame()).contains("O B S E R V A T O R Y"));
    assert!(!frame_text(runtime.frame()).contains("TELEMETRY"));
    assert!(!frame_text(runtime.frame()).contains("SIGNAL /"));
    assert_eq!(runtime.focused_key(), Some("pause"));
    assert!(runtime.frame().content().iter().any(|cell| cell
        .content
        .as_char()
        .is_some_and(|c| ('\u{2801}'..='\u{28ff}').contains(&c))));
    runtime.step(&[key('p')])?;
    let frozen = runtime.state().time;
    clock.advance(Duration::from_millis(33));
    runtime.step(&[])?;
    assert_eq!(runtime.state().time, frozen);
    for scene in ['2', '3', '4', '5', '6', '1'] {
        runtime.step(&[key(scene)])?;
        assert_eq!(runtime.state().scene, scene as usize - '1' as usize);
        for _ in 0..8 {
            clock.advance(Duration::from_millis(100));
            runtime.step(&[])?;
        }
        let dots: String = runtime
            .frame()
            .content()
            .iter()
            .filter_map(|cell| cell.content.as_char())
            .filter(|c| ('\u{2801}'..='\u{28ff}').contains(c))
            .collect();
        assert!(dots.len() > 100, "scene {scene} must draw visible geometry");
    }
    runtime.step(&[key('c'), key('s')])?;
    assert_eq!(runtime.state().palette, 1);
    assert_eq!(runtime.state().rate(), 2.0);
    runtime.step(&[key('?')])?;
    clock.advance(Duration::from_millis(500));
    runtime.step(&[])?;
    assert!(frame_text(runtime.frame()).contains("FLIGHT MANUAL"));
    runtime.step(&[key('?')])?;
    assert!(!runtime.state().help);
    // Scene shortcuts and clicks never add a scene to the Tab order.
    assert_eq!(runtime.focused_key(), Some("pause"));
    for expected in ["color", "speed", "pause", "color", "speed", "pause"] {
        runtime.step(&[Event::Key(KeyEvent::new(
            KeyCode::Tab,
            KeyModifiers::empty(),
        ))])?;
        assert_eq!(runtime.focused_key(), Some(expected));
    }
    runtime.step(&[Event::Key(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::empty(),
    ))])?;
    assert!(!runtime.state().paused);
    // Find the visible scene button in the actual frame, then click it.
    let scene_row = frame_text(runtime.frame())
        .lines()
        .position(|line| line.contains("2 Warp"))
        .unwrap() as u16;
    runtime.step(&[key('2')])?;
    runtime.step(&[
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 2,
            row: scene_row,
            modifiers: KeyModifiers::empty(),
        }),
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 2,
            row: scene_row,
            modifiers: KeyModifiers::empty(),
        }),
    ])?;
    assert_eq!(runtime.state().scene, 0);
    assert_eq!(runtime.focused_key(), Some("pause"));
    for (width, height) in [
        (80, 24),
        (60, 20),
        (40, 12),
        (12, 4),
        (1, 1),
        (120, 38),
        (INLINE_WIDTH, INLINE_HEIGHT),
        (INLINE_WIDTH + 20, INLINE_HEIGHT + 10),
    ] {
        runtime.step(&[Event::Resize(width, height)])?;
        assert_eq!(runtime.frame().size().width, width);
    }
    runtime.step(&[key('q')])?;
    assert!(runtime.exit_reason().is_some());
    println!(
        "PASS: animation, title, braille output, pause, scenes, palette, speed, modal, skipped scene focus, mouse, resize and exit."
    );
    Ok(())
}

/// Export real buffer cells, preserving their colors; this is not a mockup.
fn svg(buffer: &Buffer) -> String {
    fn escape(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
    fn color(color: Option<crossterm::style::Color>, fallback: &str) -> String {
        match color {
            Some(crossterm::style::Color::Rgb { r, g, b }) => format!("#{r:02x}{g:02x}{b:02x}"),
            _ => fallback.to_owned(),
        }
    }
    let width = buffer.size().width as usize;
    let height = buffer.size().height as usize;
    let svg_width = width * 9 + 32;
    let svg_height = height * 18 + 32;
    let mut out = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{svg_width}\" height=\"{svg_height}\" viewBox=\"0 0 {svg_width} {svg_height}\"><rect width=\"100%\" height=\"100%\" fill=\"#070c17\"/><g font-family=\"Menlo,DejaVu Sans Mono,monospace\" font-size=\"14\">" );
    for (index, cell) in buffer.content().iter().enumerate() {
        let x = index % width * 9 + 16;
        let y = index / width * 18 + 16;
        let colors = cell.content.style.colors;
        let bg = color(colors.and_then(|c| c.background), "#070c17");
        let fg = color(colors.and_then(|c| c.foreground), "#d6e7f3");
        let _ = write!(out, "<rect x=\"{x}\" y=\"{y}\" width=\"9\" height=\"18\" fill=\"{bg}\"/><text x=\"{x}\" y=\"{}\" fill=\"{fg}\" xml:space=\"preserve\">{}</text>", y + 14, escape(&cell.content.symbol()));
    }
    out.push_str("</g></svg>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_size_respects_the_available_terminal_space() {
        let store = WidgetStore::new();
        let theme = Theme::dark();
        let ctx = MeasureCtx::new(&store, &theme);
        for (width, height) in [
            (1, 1),
            (40, 12),
            (80, 24),
            (100, 28),
            (120, 50),
            (INLINE_WIDTH, INLINE_HEIGHT),
            (200, 100),
        ] {
            let size = view(&State::default()).measure(
                SizeProposal {
                    width: AxisLimit::Exact(width),
                    height: AxisLimit::AtMost(height),
                },
                &ctx,
            );
            assert_eq!(size.width, width.min(INLINE_WIDTH));
            assert_eq!(size.height, height.min(INLINE_HEIGHT));
        }
    }

    #[test]
    fn complete_headless_flow() {
        super::smoke().unwrap();
    }
}
