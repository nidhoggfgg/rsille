//! Battle HUD and command menu built on native TUI controls and focus routing.
use crate::{
    battle::{ENEMY_HP, MAX_PP, PLAYER_HP, SKILLS},
    Game, Msg,
};
use rsille::{
    render::{
        area::{Area, Size},
        chunk::Chunk,
    },
    tui::prelude::*,
};

type Rgb = [u8; 3];
const INK: Rgb = [9, 17, 29];
const WHITE: Rgb = [245, 240, 216];
const MUTED: Rgb = [172, 190, 185];
const SELECTED_FILL: Rgb = [28, 42, 52];
const ACCENTS: [Rgb; 4] = [
    [255, 211, 83],
    [232, 191, 157],
    [129, 211, 232],
    [215, 172, 248],
];
fn color([r, g, b]: Rgb) -> Color {
    Color::Rgb(r, g, b)
}
fn paint(fg: Rgb, bg: Rgb) -> Style {
    Style::default().fg(color(fg)).bg(color(bg))
}
pub fn wide(size: Size) -> bool {
    size.width >= 112 && size.height >= 30
}

pub fn theme() -> Theme {
    let mut theme = Theme::dark();
    theme.styles.surface = paint(WHITE, INK);
    theme.styles.surface_elevated = paint(WHITE, INK);
    theme.styles.text = paint(WHITE, INK);
    theme.styles.border = Style::default().bg(color(INK));
    theme.styles.primary_action = paint(WHITE, INK);
    theme.styles.primary_action_hover = paint(WHITE, INK).bold();
    theme.styles.primary_action_focused = paint(WHITE, SELECTED_FILL).bold();
    theme.styles.border_focused = paint(WHITE, SELECTED_FILL);
    theme.styles.interactive_focused = paint(WHITE, SELECTED_FILL).bold();
    theme.styles.disabled = paint([88, 109, 130], INK);
    theme
}

fn frosted(widget: impl IntoWidget<Msg>, opacity: f64, slanted: bool) -> Visual<Msg> {
    let backdrop = Backdrop::frosted().opacity(opacity);
    let backdrop = if slanted {
        backdrop.mask(|x, y, size| {
            let diagonal = f64::from(x) + f64::from(y) + 1.0;
            let distance =
                (diagonal - f64::from(size.height)).min(f64::from(size.width) - diagonal);
            (distance / 2.0).clamp(0.0, 1.0)
        })
    } else {
        backdrop
    };
    visual(widget).backdrop(backdrop)
}

fn native_label(value: impl Into<String>, fg: Rgb, bg: Rgb) -> Label<Msg> {
    label(value).style(paint(fg, bg)).height(1)
}

/// Decorations only. No hit testing, focus state or pointer event handling.
fn line(chunk: &mut Chunk, x: u16, y: u16, width: u16, text: &str, fg: Rgb, bg: Rgb) {
    let size = chunk.area().size();
    if x >= size.width || y >= size.height {
        return;
    }
    let value: String = text
        .chars()
        .take(width.min(size.width - x) as usize)
        .collect();
    let _ = chunk.set_string(x, y, &value, paint(fg, bg).to_render_style());
}

/// A slashed silhouette. Unpainted corners leave the live scene untouched.
fn ribbon(chunk: &mut Chunk, accent: Rgb, fill: Rgb) {
    let (w, h) = (chunk.area().width(), chunk.area().height());
    for y in 0..h {
        let left = h - 1 - y;
        let right = w.saturating_sub(y + 1);
        if left >= right {
            continue;
        }
        let _ = chunk.fill(
            left,
            y,
            right - left,
            1,
            ' ',
            paint(accent, fill).to_render_style(),
        );
        line(chunk, left, y, 1, "╱", accent, fill);
        line(chunk, right, y, 1, "╱", accent, fill);
        if y == 0 {
            let _ = chunk.fill(
                left + 1,
                y,
                right - left - 1,
                1,
                '▁',
                paint(accent, fill).to_render_style(),
            );
        }
    }
}

/// Safe central allocation; artwork around it has no pointer handlers.
struct CommandFrame {
    children: Vec<Box<dyn Widget<Msg>>>,
    large: bool,
}
impl Widget<Msg> for CommandFrame {
    fn key(&self) -> Option<&str> {
        Some("card-frame")
    }
    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }
    fn constraints(&self) -> Constraints {
        Constraints::fill()
    }
    fn layout(&self, area: Area, _ctx: &MeasureCtx) -> Layout {
        let inset = if self.large { 8 } else { 6 };
        Layout::new([ChildLayout::new(
            0,
            area.shrink_saturating(1, 1, inset, inset),
        )])
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.render_children(chunk, self);
    }
}

// Keep one wrapper level for stable button identities across responsive layouts.
struct FixedHeight {
    height: u16,
    children: Vec<Box<dyn Widget<Msg>>>,
}
impl Widget<Msg> for FixedHeight {
    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }
    fn constraints(&self) -> Constraints {
        Constraints {
            min_height: self.height,
            max_height: Some(self.height),
            ..Constraints::fill()
        }
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.render_children(chunk, self);
    }
}

struct HealthHud {
    accent: Rgb,
    index: usize,
    children: Vec<Box<dyn Widget<Msg>>>,
}
impl Widget<Msg> for HealthHud {
    fn key(&self) -> Option<&str> {
        Some(if self.index == 0 {
            "player-vitals"
        } else {
            "foe-vitals"
        })
    }
    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }
    fn constraints(&self) -> Constraints {
        Constraints::fill()
    }
    fn layout(&self, area: Area, _ctx: &MeasureCtx) -> Layout {
        Layout::new([ChildLayout::new(0, area.shrink_saturating(1, 1, 10, 1))])
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        // A small round team emblem sits beside the open meter, independently
        // of the text's glass effect so both halves retain their painted colors.
        let pixel = |x: f32, y: f32| -> Option<Rgb> {
            let dx = x - 3.5;
            let dy = y - 3.5;
            let radius = dx.hypot(dy);
            if radius > 3.8 {
                None
            } else if radius < 0.85 {
                Some(WHITE)
            } else if radius < 1.8 || dy.abs() < 0.6 {
                Some(INK)
            } else if radius > 3.0 {
                Some(WHITE)
            } else if dy < 0.0 {
                Some(self.accent)
            } else {
                Some([190, 204, 209])
            }
        };
        let area = chunk.area();
        for y in 0..4 {
            for x in 0..8 {
                let top = pixel(x as f32, (y * 2) as f32);
                let bottom = pixel(x as f32, (y * 2 + 1) as f32);
                if top.is_none() && bottom.is_none() {
                    continue;
                }
                let scene = match chunk
                    .buffer()
                    .get((area.x() + x, area.y() + y + 1).into())
                    .and_then(|cell| cell.style.colors)
                    .and_then(|colors| colors.background)
                {
                    Some(crossterm::style::Color::Rgb { r, g, b }) => [r, g, b],
                    _ => [0; 3],
                };
                let _ = chunk.set_char(
                    x,
                    y + 1,
                    '▀',
                    paint(top.unwrap_or(scene), bottom.unwrap_or(scene)).to_render_style(),
                );
            }
        }
        ctx.render_children(chunk, self);
    }
}

fn hp_color(hp: u16, max: u16) -> Rgb {
    if hp * 4 <= max {
        [255, 108, 126]
    } else if hp * 2 <= max {
        [255, 199, 94]
    } else {
        [109, 230, 184]
    }
}
fn health(game: &Game, index: usize) -> HealthHud {
    let (name, hp, max, accent, element) = if index == 0 {
        (
            "PIKACHU",
            game.battle.player_hp,
            PLAYER_HP,
            ACCENTS[0],
            "ELECTRIC",
        )
    } else {
        (
            "BLASTOISE",
            game.battle.enemy_hp,
            ENEMY_HP,
            ACCENTS[2],
            "WATER",
        )
    };
    let status = if hp == 0 {
        "FAINTED"
    } else if index == 1 && game.battle.paralyzed {
        "PARALYZED"
    } else if index == 1 && game.battle.guard {
        "GUARD UP"
    } else {
        element
    };
    let content = grid()
        .rows("1 1 1")
        .columns("1fr")
        .child(
            grid()
                .columns("1fr 6")
                .child(native_label(name, WHITE, INK).style(paint(WHITE, INK).bold()))
                .child(native_label("Lv.25", accent, INK)),
        )
        .child(
            grid()
                .columns("3 1fr")
                .child(native_label("HP", accent, INK))
                .child(
                    progress_bar(hp as f64 / max as f64)
                        .key(format!("hp-{index}"))
                        .width(12)
                        .variant(ProgressBarVariant::Block)
                        .style(paint([58, 82, 68], INK))
                        .fill_style(paint(hp_color(hp, max), INK)),
                ),
        )
        .child(
            grid()
                .columns("1fr 8")
                .child(native_label(status, accent, INK))
                .child(native_label(format!("{hp:03}/{max:03}"), WHITE, INK)),
        );
    HealthHud {
        accent,
        index,
        children: vec![Box::new(frosted(content, 0.45, false))],
    }
}

/// Move decoration reads native focus / hover; Button still owns all interaction.
struct MoveCard {
    index: usize,
    pp: u8,
    enabled: bool,
    large: bool,
    key: String,
    children: Vec<Box<dyn Widget<Msg>>>,
}
impl Widget<Msg> for MoveCard {
    fn key(&self) -> Option<&str> {
        Some(&self.key)
    }
    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }
    fn constraints(&self) -> Constraints {
        Constraints::fill()
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let (w, h) = (chunk.area().width(), chunk.area().height());
        if w < 2 || h < 2 {
            return;
        }
        let focused = self.enabled && ctx.is_focus_within();
        let hover = self.enabled && ctx.is_hover_within();
        let accent = if self.enabled {
            ACCENTS[self.index]
        } else {
            [91, 112, 117]
        };
        let edge = if focused || hover { WHITE } else { accent };
        let fill = if focused { SELECTED_FILL } else { INK };
        let detail = if focused { WHITE } else { accent };
        ribbon(chunk, edge, fill);
        ctx.render_children(chunk, self);
        line(
            chunk,
            4,
            1,
            2,
            &format!("{:02}", self.index + 1),
            detail,
            fill,
        );
        line(
            chunk,
            4,
            h - 1,
            3,
            if focused { "▶▶" } else { "╱╱" },
            edge,
            fill,
        );
        if self.large {
            line(
                chunk,
                9,
                1,
                w.saturating_sub(26).min(10),
                ["ELECTRIC", "NORMAL", "STEEL", "ELECTRIC"][self.index],
                detail,
                fill,
            );
            line(
                chunk,
                w.saturating_sub(19),
                1,
                10,
                &format!("PP {:02}/{:02}", self.pp, MAX_PP[self.index]),
                detail,
                fill,
            );
            line(
                chunk,
                9,
                h - 1,
                w.saturating_sub(17),
                if self.pp == 0 {
                    "NO PP LEFT"
                } else if !self.enabled {
                    "STANDBY"
                } else {
                    [
                        "48 DMG / SUPER EFFECTIVE",
                        "23 DMG / PHYSICAL",
                        "34 DMG / BREAKS GUARD",
                        "PARALYZE / STATUS",
                    ][self.index]
                },
                edge,
                fill,
            );
        } else {
            line(
                chunk,
                w.saturating_sub(15),
                0,
                11,
                &format!("PP {:02}/{:02}", self.pp, MAX_PP[self.index]),
                detail,
                fill,
            );
        }
    }
}

fn focused_move(path: &WidgetPath) -> Option<usize> {
    match path.as_slice().last()? {
        WidgetKey::Named(key) => key.strip_prefix("move-")?.parse().ok(),
        _ => None,
    }
}

/// Arrow keys request framework focus. Tab, mouse targeting, activation and disabled
/// controls are handled by the runtime and Button, never by manual cell rectangles.
struct MoveDeck {
    columns: usize,
    enabled: [bool; 4],
    children: Vec<Box<dyn Widget<Msg>>>,
}
impl Widget<Msg> for MoveDeck {
    fn key(&self) -> Option<&str> {
        Some("move-deck")
    }
    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }
    fn constraints(&self) -> Constraints {
        Constraints::fill()
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.render_children(chunk, self);
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<Msg>) {
        if ctx.phase() != EventPhase::Bubble || ctx.was_handled() {
            return;
        }
        let Event::Key(key) = event else {
            return;
        };
        if key.kind == KeyEventKind::Release {
            return;
        }
        let Some(current) = ctx.focused_path().and_then(focused_move) else {
            return;
        };
        let (dx, dy) = match key.code {
            KeyCode::Left => (-1, 0),
            KeyCode::Right => (1, 0),
            KeyCode::Up => (0, -1),
            KeyCode::Down => (0, 1),
            _ => return,
        };
        let columns = self.columns as i32;
        let rows = 4 / columns;
        let mut next = current;
        for _ in 0..4 {
            let x = (next as i32 % columns + dx).rem_euclid(columns);
            let y = (next as i32 / columns + dy).rem_euclid(rows);
            next = (y * columns + x) as usize;
            if self.enabled[next] {
                ctx.request_focus(
                    ctx.path()
                        .child("skills-grid")
                        .child(format!("skill-glass-{next}"))
                        .child(format!("skill-card-{next}"))
                        .child("card-frame")
                        .child(0usize)
                        .child(format!("move-{next}")),
                );
                break;
            }
        }
    }
}

struct StaggeredMoves {
    large: bool,
    children: Vec<Box<dyn Widget<Msg>>>,
}
impl Widget<Msg> for StaggeredMoves {
    fn key(&self) -> Option<&str> {
        Some("skills-grid")
    }
    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }
    fn constraints(&self) -> Constraints {
        Constraints::fill()
    }
    fn layout(&self, area: Area, _ctx: &MeasureCtx) -> Layout {
        let gap = if self.large { 4 } else { 2 };
        let left = area.width().saturating_sub(gap) / 2;
        let right = area.width().saturating_sub(gap + left);
        let h = if self.large { 4 } else { 3 };
        let positions = if self.large {
            [
                (0, 0, left),
                (left + gap, 1, right),
                (3, 5, left.saturating_sub(3)),
                (left + gap - 2, 6, right),
            ]
        } else {
            [
                (0, 0, left),
                (left + gap, 0, right),
                (2, 4, left.saturating_sub(2)),
                (left + gap + 2, 4, right.saturating_sub(2)),
            ]
        };
        Layout::new(
            positions
                .into_iter()
                .enumerate()
                .filter_map(|(index, (x, y, w))| {
                    Area::new((area.x() + x, area.y() + y).into(), (w, h).into())
                        .clamp_to(&area)
                        .map(|bounds| ChildLayout::new(index, bounds))
                }),
        )
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.render_children(chunk, self);
    }
}

fn moves(game: &Game, large: bool) -> MoveDeck {
    let enabled = std::array::from_fn(|i| game.available(i));
    let children = (0..4)
        .map(|i| {
            let control = button(SKILLS[i].name())
                .key(format!("move-{i}"))
                .variant(ButtonVariant::Primary)
                .disabled(!enabled[i])
                .on_click(move || Msg::Use(i));
            let frame = CommandFrame {
                large,
                children: vec![Box::new(FixedHeight {
                    height: if large { 2 } else { 1 },
                    children: vec![Box::new(control)],
                })],
            };
            Box::new(
                frosted(
                    MoveCard {
                        index: i,
                        pp: game.battle.pp[i],
                        enabled: enabled[i],
                        large,
                        key: format!("skill-card-{i}"),
                        children: vec![Box::new(frame)],
                    },
                    0.48,
                    true,
                )
                .key(format!("skill-glass-{i}")),
            ) as Box<dyn Widget<Msg>>
        })
        .collect();
    MoveDeck {
        columns: 2,
        enabled,
        children: vec![Box::new(StaggeredMoves { large, children })],
    }
}

pub(crate) fn scene_widget(game: &Game, large: bool) -> impl Widget<Msg> {
    let battle = game.battle;
    let camera = game.camera;
    let raster = game.raster.clone();
    let picture = canvas(move |surface, dimensions| {
        let (w, h) = (
            dimensions.sample_width() as usize,
            dimensions.sample_height() as usize,
        );
        let scale = if dimensions.encoding() == Encoding::HalfBlock {
            2
        } else {
            1
        };
        let mut raster = raster.borrow_mut();
        let visible_rows = dimensions
            .cell_height()
            .saturating_sub(if large { 12 } else { 8 })
            .max(1) as usize;
        let view_height =
            visible_rows * dimensions.encoding().samples_per_cell().1 as usize * scale;
        crate::scene::render(
            &mut raster,
            w * scale,
            h * scale,
            view_height,
            battle,
            camera,
        );
        surface.set_background(crossterm::style::Color::Black);
        for y in 0..h {
            for x in 0..w {
                let mut rgb = [0.0; 3];
                for dy in 0..scale {
                    for dx in 0..scale {
                        let p = raster.pixels[(y * scale + dy) * raster.width + x * scale + dx];
                        for c in 0..3 {
                            rgb[c] += p[c] / (scale * scale) as f32;
                        }
                    }
                }
                let [r, g, b] = rgb.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
                surface.set_color(
                    x as f64,
                    (h - 1 - y) as f64,
                    crossterm::style::Color::Rgb { r, g, b },
                );
            }
        }
    })
    .encoding(game.encoding)
    .key("battle-canvas");
    let scene = drag_region(picture)
        .key("camera-drag")
        .threshold_cells(1)
        .on_drag(|event| {
            if let DragEvent::Update(event) = event {
                Msg::Orbit(event.delta.x as f32 * 0.018, event.delta.y as f32 * 0.025)
            } else {
                Msg::Ignore
            }
        });
    let inset = if large { 3 } else { 1 };
    let hud_width = if large { 42 } else { 34 };
    overlay(scene)
        .key("battle-scene")
        .layer(
            OverlayLayer::new(health(game, 1))
                .floating(OverlayAnchor::TopRight)
                .offset(-inset, if large { 3 } else { 1 })
                .size(hud_width, 5),
        )
        .layer(
            OverlayLayer::new(health(game, 0))
                .floating(OverlayAnchor::BottomLeft)
                .offset(inset, if large { -2 } else { -9 })
                .size(hud_width, 5),
        )
}

pub fn view(game: &Game) -> impl Widget<Msg> {
    let large = wide(game.viewport);
    let menu = if large {
        grid()
            .columns("1fr 2fr")
            .gap_column(3)
            .child(spacer())
            .child(moves(game, true))
    } else {
        grid().columns("1fr").child(moves(game, false))
    };
    let footer = grid()
        .key("battle-controls")
        .columns("1fr")
        .rows(if large { "10 1 1" } else { "7 1" })
        .padding(Padding::horizontal(if game.viewport.width >= 60 {
            3
        } else {
            0
        }))
        .child(menu);
    let footer = if large {
        footer.child(spacer().height(1))
    } else {
        footer
    };
    let help = match (large, game.paused) {
        (true, true) => " [1-4] MOVE   [ARROWS / TAB] SELECT   [ENTER] CONFIRM                         [DRAG] CAMERA   [P] RESUME  [R] REMATCH   [Q] EXIT",
        (true, false) => " [1-4] MOVE   [ARROWS / TAB] SELECT   [ENTER] CONFIRM                         [DRAG] CAMERA   [P] PAUSE   [R] REMATCH   [Q] EXIT",
        (false, true) => "1-4 move  ARROWS/TAB select  ENTER confirm  P resume R rematch  Q exit",
        (false, false) => "1-4 move  ARROWS/TAB select  ENTER confirm  P pause  R rematch  Q exit",
    };
    let footer = footer.child(frosted(native_label(help, MUTED, INK), 0.55, false));
    overlay(scene_widget(game, large))
        .key("pokemon-battle")
        .layer(
            OverlayLayer::new(footer)
                .floating(OverlayAnchor::Bottom)
                .size(game.viewport.width, if large { 12 } else { 8 }),
        )
}
