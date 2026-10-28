//! Native TUI controls, rounded panels and layout with skill-specific decorations.
use crate::{
    battle::{Phase, ENEMY_HP, MAX_PP, PLAYER_HP, SKILLS},
    Game, Msg,
};
use rsille::{
    render::{area::Size, chunk::Chunk},
    tui::prelude::*,
};

type Rgb = [u8; 3];
const INK: Rgb = [9, 17, 29];
const WHITE: Rgb = [232, 241, 245];
const MUTED: Rgb = [120, 148, 169];
const ACCENTS: [Rgb; 4] = [
    [255, 211, 83],
    [255, 145, 117],
    [129, 211, 232],
    [192, 160, 255],
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
    // Let native Panel borders inherit each card's elemental foreground.
    theme.styles.border = Style::default().bg(color(INK));
    theme.styles.primary_action = paint(WHITE, [15, 25, 40]);
    theme.styles.primary_action_hover = paint(WHITE, [32, 43, 58]);
    theme.styles.primary_action_focused = paint(WHITE, [32, 43, 58]).bold();
    theme.styles.border_focused = paint(ACCENTS[0], INK);
    theme.styles.interactive_focused = paint(WHITE, [32, 43, 58]).bold();
    theme.styles.disabled = paint([88, 109, 130], INK);
    theme
}

fn frosted(widget: impl IntoWidget<Msg>, game: &Game, opacity: f32) -> Visual<Msg> {
    visual(widget).custom_effect(crate::glass::Glass {
        backdrop: game.backdrop.clone(),
        opacity,
    })
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

/// Only constrains height. Native Panel owns the frame and title drawing.
struct FixedHeight {
    height: u16,
    children: Vec<Box<dyn Widget<Msg>>>,
}
impl Widget<Msg> for FixedHeight {
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.render_children(chunk, self);
    }
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
fn health(game: &Game, index: usize) -> FixedHeight {
    let (name, hp, max, accent, element, fill) = if index == 0 {
        (
            "PIKACHU",
            game.battle.player_hp,
            PLAYER_HP,
            ACCENTS[0],
            "025 / ELECTRIC",
            [26, 29, 34],
        )
    } else {
        (
            "BLASTOISE",
            game.battle.enemy_hp,
            ENEMY_HP,
            [104, 212, 246],
            "009 / WATER",
            [15, 31, 45],
        )
    };
    let status = if hp == 0 {
        "FAINTED"
    } else if index == 1 && game.battle.paralyzed {
        "PARALYZED"
    } else if index == 1 && game.battle.guard {
        "GUARD UP"
    } else {
        "READY"
    };
    let panel = panel()
        .border(BorderStyle::Rounded)
        .padding(Padding::horizontal(1))
        .style(paint(accent, fill))
        .title(format!(" {element} "))
        .child(native_label(
            format!(
                "{name:<13} Lv.25   {}",
                if index == 0 { "YOU" } else { "FOE" }
            ),
            WHITE,
            fill,
        ))
        .child(
            progress_bar(hp as f64 / max as f64)
                .key(format!("hp-{index}"))
                .width(33)
                .variant(ProgressBarVariant::Segmented)
                .style(paint([48, 69, 58], fill))
                .fill_style(paint(hp_color(hp, max), fill)),
        )
        .child(native_label(
            format!("{hp:03}/{max:03}                 {status}"),
            accent,
            fill,
        ));
    FixedHeight {
        height: 5,
        children: vec![Box::new(panel)],
    }
}
fn compact_health(game: &Game) -> FixedHeight {
    let fill = [15, 26, 39];
    let panel = panel()
        .border(BorderStyle::Rounded)
        .padding(Padding::horizontal(1))
        .style(paint(ACCENTS[0], fill))
        .title(" TRAINER / VITALS ")
        .child(native_label(
            format!("PIKACHU   {:03}/{PLAYER_HP:03}", game.battle.player_hp),
            hp_color(game.battle.player_hp, PLAYER_HP),
            fill,
        ))
        .child(native_label(
            format!("BLASTOISE {:03}/{ENEMY_HP:03}", game.battle.enemy_hp),
            hp_color(game.battle.enemy_hp, ENEMY_HP),
            fill,
        ));
    FixedHeight {
        height: 4,
        children: vec![Box::new(panel)],
    }
}

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
        let focused = ctx.is_focus_within();
        let hover = ctx.is_hover_within();
        let accent = if self.enabled {
            ACCENTS[self.index]
        } else {
            [75, 94, 115]
        };
        let fill = if self.enabled && (focused || hover) {
            [32, 43, 58]
        } else {
            [15, 25, 40]
        };
        // Native Panel owns the frame; its native Button child owns activation.
        ctx.render_children(chunk, self);
        line(
            chunk,
            2,
            0,
            5,
            &format!(" {} {} ", if focused { "◆" } else { "◇" }, self.index + 1),
            accent,
            fill,
        );
        if self.large {
            let icons = [
                ["   ╱█ ", " ╱██  ", "   ╱  "],
                [" ▸▸╱  ", " ▸╱   ", " ╱    "],
                [" ╱━━╲ ", "  ╳╳  ", " ╲━━╱ "],
                [" ╱╲ ╱ ", "╱  ╳  ", "  ╱ ╲ "],
            ];
            for y in 0..3 {
                line(
                    chunk,
                    2,
                    y + 1,
                    w.saturating_sub(4).min(6),
                    icons[self.index][y as usize],
                    accent,
                    fill,
                );
            }
            line(
                chunk,
                9,
                1,
                w.saturating_sub(11),
                [
                    "ELECTRIC / BURST",
                    "NORMAL / RUSH",
                    "STEEL / BREAK",
                    "ELECTRIC / STATUS",
                ][self.index],
                accent,
                fill,
            );
            line(
                chunk,
                9,
                2,
                w.saturating_sub(11),
                ["48 DMG", "23 DMG", "34 DMG", "PARALYZE"][self.index],
                MUTED,
                fill,
            );
            line(
                chunk,
                3,
                4,
                w.saturating_sub(6),
                &format!(
                    "PP {:02}/{:02}   {}",
                    self.pp,
                    MAX_PP[self.index],
                    if self.pp == 0 {
                        "EXHAUSTED"
                    } else if self.enabled && (focused || hover) {
                        "▸ EXECUTE"
                    } else if self.enabled {
                        "AVAILABLE"
                    } else {
                        "STANDBY"
                    }
                ),
                accent,
                fill,
            );
        } else {
            line(
                chunk,
                w.saturating_sub(12),
                0,
                10,
                &format!("PP {:02}/{:02}", self.pp, MAX_PP[self.index]),
                accent,
                fill,
            );
            line(
                chunk,
                3,
                h - 1,
                w.saturating_sub(6),
                [" 48 DMG ", " 23 DMG ", " 34 DMG ", " STATUS "][self.index],
                accent,
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

struct MoveDetail {
    fallback: usize,
}
impl Widget<Msg> for MoveDetail {
    fn constraints(&self) -> Constraints {
        Constraints {
            min_height: 1,
            max_height: Some(1),
            ..Constraints::fill()
        }
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let index = ctx
            .focused_path()
            .and_then(focused_move)
            .unwrap_or(self.fallback);
        native_label(SKILLS[index].description(), MUTED, INK).render(chunk, ctx);
    }
}

fn moves(game: &Game, large: bool) -> MoveDeck {
    let mut grid = grid()
        .key("skills-grid")
        .columns(if large {
            "repeat(4, 1fr)"
        } else {
            "repeat(2, 1fr)"
        })
        .rows(if large { "6" } else { "3 3" })
        .gap_column(2)
        .gap_row(1)
        .trap_focus();
    let enabled = std::array::from_fn(|i| game.available(i));
    for i in 0..4 {
        let control = button(SKILLS[i].name())
            .key(format!("move-{i}"))
            .variant(ButtonVariant::Primary)
            .disabled(!enabled[i])
            .on_click(move || Msg::Use(i));
        let accent = if enabled[i] {
            ACCENTS[i]
        } else {
            [75, 94, 115]
        };
        let frame = panel()
            .key("card-frame")
            .border(BorderStyle::Rounded)
            .title(format!(" {} ", i + 1))
            .padding(Padding::ZERO)
            .style(paint(accent, [15, 25, 40]))
            .hover_border_style(paint(WHITE, [15, 25, 40]))
            .child(FixedHeight {
                height: if large { 4 } else { 1 },
                children: vec![Box::new(control)],
            });
        grid = grid.child(MoveCard {
            index: i,
            pp: game.battle.pp[i],
            enabled: enabled[i],
            large,
            key: format!("skill-card-{i}"),
            children: vec![Box::new(frame)],
        });
    }
    MoveDeck {
        columns: if large { 4 } else { 2 },
        enabled,
        children: vec![Box::new(grid)],
    }
}

fn scene_widget(game: &Game, large: bool) -> impl Widget<Msg> {
    let battle = game.battle;
    let camera = game.camera;
    let raster = game.raster.clone();
    let backdrop = game.backdrop.clone();
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
            .saturating_sub(if large { 11 } else { 10 })
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
        if let Ok(mut backdrop) = backdrop.write() {
            backdrop.update(&raster);
        }
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
    let hud: Box<dyn Widget<Msg>> = if large {
        Box::new(col().gap(1).child(health(game, 0)).child(health(game, 1)))
    } else {
        Box::new(compact_health(game))
    };
    let scene = overlay(scene).key("battle-scene").layer(
        OverlayLayer::new(frosted(hud, game, 0.50).key("vitals-glass"))
            .floating(OverlayAnchor::TopLeft)
            .offset(if large { 3 } else { 2 }, if large { 3 } else { 1 })
            .size(if large { 37 } else { 24 }, if large { 11 } else { 4 }),
    );
    if large {
        scene
            .layer(
                OverlayLayer::new(frosted(
                    native_label("◈  P O K É M O N   /   B A T T L E", ACCENTS[0], INK),
                    game,
                    0.45,
                ))
                .floating(OverlayAnchor::TopLeft)
                .offset(3, 0)
                .size(44, 1),
            )
            .layer(
                OverlayLayer::new(frosted(
                    native_label("I N D I G O   /   D U E L  001", [116, 203, 228], INK),
                    game,
                    0.45,
                ))
                .floating(OverlayAnchor::TopRight)
                .offset(-3, 0)
                .size(32, 1),
            )
    } else {
        scene
    }
}

pub fn view(game: &Game) -> impl Widget<Msg> {
    let large = wide(game.viewport);
    let state = if game.paused {
        "PAUSED"
    } else {
        match game.battle.phase {
            Phase::Choose => "CHOOSE A MOVE",
            Phase::Action { .. } => "BATTLE IN PROGRESS",
            Phase::Won => "VICTORY",
            Phase::Lost => "DEFEAT",
        }
    };
    let badge = native_label(
        format!(" ◈ TURN {:02} / {state} ", game.battle.turn),
        INK,
        ACCENTS[0],
    );
    let message = if game.battle.last_damage > 0 {
        format!("{} / -{} HP", game.battle.message, game.battle.last_damage)
    } else {
        game.battle.message.to_owned()
    };
    let mut footer = grid()
        .key("control-layout")
        .columns("1fr")
        .rows(if large { "1 1 6 1 1 1" } else { "1 1 7 1" })
        .padding(Padding::horizontal(if game.viewport.width >= 60 {
            3
        } else {
            0
        }));
    if large {
        footer = footer
            .child(
                grid()
                    .columns("38 1fr")
                    .gap_column(2)
                    .child(badge)
                    .child(native_label(message, WHITE, INK)),
            )
            .child(spacer().height(1));
    } else {
        footer = footer.child(badge).child(native_label(message, WHITE, INK));
    }
    footer = footer.child(moves(game, large));
    if large {
        footer = footer
            .child(MoveDetail {
                fallback: game.last_move,
            })
            .child(native_label("", MUTED, INK));
    }
    footer=footer.child(native_label("1-4 / CLICK attack   ARROWS / TAB select   ENTER confirm   DRAG orbit   P pause   R rematch   Q leave",MUTED,INK));
    overlay(scene_widget(game, large))
        .key("pokemon-battle")
        .layer(
            OverlayLayer::new(frosted(footer, game, 0.48).key("battle-controls"))
                .floating(OverlayAnchor::Bottom)
                .size(game.viewport.width, if large { 11 } else { 10 }),
        )
}
