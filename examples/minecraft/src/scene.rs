//! CPU voxel ray casting, pixel textures and a two-pixel terminal-cell display.
use crate::{
    world::{Block, Hit, Vec3, World, HOTBAR},
    Msg,
};
use rsille::{render::chunk::Chunk, tui::prelude::*};

pub const BG: Color = Color::Rgb(28, 30, 26);
pub const TEXT: Color = Color::Rgb(236, 236, 219);
type Rgb = [f64; 3];

fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    std::array::from_fn(|i| a[i] * (1.0 - t) + b[i] * t)
}
fn scale(a: Rgb, t: f64) -> Rgb {
    a.map(|v| (v * t).clamp(0.0, 255.0))
}
fn color(a: Rgb) -> Color {
    Color::Rgb(a[0] as u8, a[1] as u8, a[2] as u8)
}
fn hash(x: i32, y: i32, z: i32) -> f64 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ (z as u32).wrapping_mul(2_147_483_647);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    (h ^ (h >> 16)) as f64 / u32::MAX as f64
}

pub fn base(block: Block) -> Rgb {
    match block {
        Block::Grass => [92.0, 151.0, 50.0],
        Block::Dirt => [132.0, 91.0, 58.0],
        Block::Stone => [134.0, 137.0, 133.0],
        Block::Sand => [222.0, 210.0, 146.0],
        Block::Wood => [107.0, 79.0, 41.0],
        Block::Leaves => [53.0, 115.0, 38.0],
        Block::Planks => [179.0, 141.0, 82.0],
        Block::Glass => [170.0, 215.0, 221.0],
        Block::Water => [47.0, 116.0, 197.0],
        Block::Bedrock => [55.0, 57.0, 56.0],
        Block::Wool => [225.0, 226.0, 211.0],
        Block::Coal => [61.0, 55.0, 48.0],
        Block::Air => [135.0, 192.0, 238.0],
    }
}

fn sky(direction: Vec3, world: &World) -> Rgb {
    if world.night {
        let stars = hash(
            (direction[0] * 500.0) as i32,
            (direction[1] * 500.0) as i32,
            (direction[2] * 500.0) as i32,
        );
        if direction[1] > 0.12 && stars > 0.998 {
            return [205.0, 219.0, 236.0];
        }
    }
    let horizon = if world.night {
        [41.0, 52.0, 85.0]
    } else {
        [184.0, 215.0, 241.0]
    };
    let zenith = if world.night {
        [13.0, 21.0, 47.0]
    } else {
        [97.0, 166.0, 232.0]
    };
    let mut result = mix(horizon, zenith, (direction[1] * 1.2).clamp(0.0, 1.0));
    // Square sun/moon, and flat, square-edged clouds at a fixed world altitude.
    let sun = [0.43, 0.29, 0.85];
    if (direction[0] - sun[0]).abs() < 0.065
        && (direction[1] - sun[1]).abs() < 0.065
        && direction[2] > 0.5
    {
        result = if world.night {
            [219.0, 226.0, 221.0]
        } else {
            [255.0, 246.0, 188.0]
        };
    }
    if direction[1] > 0.035 {
        let distance = (24.0 - world.eye()[1]) / direction[1];
        if distance > 0.0 && distance < 170.0 {
            let x = world.player[0] + direction[0] * distance + world.time * 0.22;
            let z = world.player[2] + direction[2] * distance;
            if hash((x / 9.0).floor() as i32, 17, (z / 6.0).floor() as i32) > 0.56 {
                let cloud = if world.night {
                    [66.0, 74.0, 99.0]
                } else {
                    [242.0, 245.0, 232.0]
                };
                result = mix(cloud, result, (distance / 220.0).min(0.7));
            }
        }
    }
    result
}

fn texture(hit: &Hit, world: &World) -> Rgb {
    let axis = if hit.normal[0] != 0 {
        0
    } else if hit.normal[1] != 0 {
        1
    } else {
        2
    };
    let (u, v) = match axis {
        0 => (hit.point[2].rem_euclid(1.0), hit.point[1].rem_euclid(1.0)),
        1 => (hit.point[0].rem_euclid(1.0), hit.point[2].rem_euclid(1.0)),
        _ => (hit.point[0].rem_euclid(1.0), hit.point[1].rem_euclid(1.0)),
    };
    let (px, py) = ((u * 16.0) as i32, (v * 16.0) as i32);
    let noise = hash(px + hit.cell[0] * 16, py + hit.cell[1] * 16, hit.cell[2]);
    let mut rgb = base(hit.block);
    match hit.block {
        Block::Grass if axis != 1 => {
            rgb = if v > 0.76 - noise * 0.15 {
                base(Block::Grass)
            } else {
                base(Block::Dirt)
            };
        }
        Block::Wood => {
            if axis == 1 {
                let ring = ((u - 0.5).abs().max((v - 0.5).abs()) * 12.0) as i32;
                rgb = scale([172.0, 142.0, 87.0], if ring % 2 == 0 { 1.0 } else { 0.78 });
            } else {
                rgb = scale(rgb, if px % 4 == 0 { 0.68 } else { 1.12 });
            }
        }
        Block::Planks => {
            if py % 4 == 0 || (px + (py / 4 % 2) * 8) % 16 == 0 {
                rgb = scale(rgb, 0.7);
            }
        }
        Block::Stone | Block::Bedrock => {
            if (px + py / 4 * 3) % 8 == 0 || py % 8 == 0 {
                rgb = scale(rgb, 0.83);
            }
        }
        Block::Glass => {
            rgb = if px == 0 || px == 15 || py == 0 || py == 15 {
                [202.0, 232.0, 229.0]
            } else if (px - py).abs() < 2 {
                [164.0, 206.0, 214.0]
            } else {
                [74.0, 108.0, 113.0]
            };
        }
        Block::Water => {
            let ripple = (hit.point[0] * 3.0 + world.time * 1.3).sin()
                * (hit.point[2] * 4.0 - world.time).sin();
            rgb = mix(rgb, [131.0, 183.0, 225.0], 0.22 + ripple * 0.12);
        }
        _ => {}
    }
    let lighting = if hit.normal[1] > 0 {
        1.0
    } else if hit.normal[0] != 0 {
        0.73
    } else {
        0.86
    };
    let grain = if hit.block == Block::Water {
        1.0
    } else {
        0.84 + noise * 0.26
    };
    rgb = scale(rgb, lighting * grain * if world.night { 0.40 } else { 1.0 });
    // Very subtle face borders retain the recognizable one-meter voxel grid.
    if !(0.012..=0.988).contains(&u) || !(0.012..=0.988).contains(&v) {
        rgb = scale(rgb, 0.84);
    }
    rgb
}

pub struct Game {
    pub world: World,
}

#[derive(Default)]
struct Drag {
    last: Option<(u16, u16)>,
}

impl Widget<Msg> for Game {
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.record_bounds(chunk.area());
        draw(chunk, &self.world);
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<Msg>) {
        if ctx.was_handled() || !self.world.active() {
            return;
        }
        let Event::Mouse(mouse) = event else {
            return;
        };
        if let Some(bounds) = ctx.bounds() {
            let Some((x, y)) = ctx.local_mouse_position(event) else {
                return;
            };
            let (w, h) = (usize::from(bounds.width()), usize::from(bounds.height()));
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
                && w >= 32
                && h >= 10
                && (h - 5..h - 2).contains(&usize::from(y))
            {
                let slot = (w / 8).clamp(4, 10);
                let left = (w - slot * 8) / 2;
                if (left..left + slot * 8).contains(&usize::from(x)) {
                    ctx.emit(Msg::Select((usize::from(x) - left) / slot));
                    ctx.set_handled();
                }
                return;
            }
            if y == 0 || usize::from(y) >= h.saturating_sub(6) {
                return;
            }
        }
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => ctx.emit(Msg::Mine),
            MouseEventKind::Down(MouseButton::Right) => ctx.emit(Msg::Place),
            MouseEventKind::Down(MouseButton::Middle) => {
                ctx.state_mut::<Drag>().last = Some((mouse.column, mouse.row));
            }
            MouseEventKind::Drag(MouseButton::Middle) => {
                if let Some((x, y)) = ctx
                    .state_mut::<Drag>()
                    .last
                    .replace((mouse.column, mouse.row))
                {
                    ctx.emit(Msg::Look(
                        (mouse.column as f64 - x as f64) * 0.035,
                        (y as f64 - mouse.row as f64) * 0.065,
                    ));
                }
            }
            MouseEventKind::Up(MouseButton::Middle) => ctx.state_mut::<Drag>().last = None,
            MouseEventKind::ScrollUp => ctx.emit(Msg::Cycle(-1)),
            MouseEventKind::ScrollDown => ctx.emit(Msg::Cycle(1)),
            _ => return,
        }
        ctx.set_handled();
    }

    fn constraints(&self) -> Constraints {
        Constraints::fill()
    }
    fn key(&self) -> Option<&str> {
        Some("voxel-world")
    }
}

fn text(chunk: &mut Chunk, x: u16, y: u16, value: &str, fg: Color, bg: Color) {
    if y < chunk.area().height() && x < chunk.area().width() {
        let _ = chunk.set_string(
            x,
            y,
            value,
            Style::default().fg(fg).bg(bg).to_render_style(),
        );
    }
}

fn draw(chunk: &mut Chunk, world: &World) {
    let (w, h) = (
        chunk.area().width() as usize,
        chunk.area().height() as usize,
    );
    if w == 0 || h == 0 {
        return;
    }
    let _ = chunk.fill(
        0,
        0,
        w as u16,
        h as u16,
        ' ',
        Style::default().bg(BG).to_render_style(),
    );
    text(chunk, 1, 0, "M I N E C R A F T", TEXT, BG);
    if w >= 72 {
        text(
            chunk,
            24,
            0,
            "a little voxel world / RSILLE",
            Color::Rgb(149, 167, 125),
            BG,
        );
    }
    if w >= 100 {
        text(
            chunk,
            w as u16 - 28,
            0,
            "CREATIVE  /  [?] field guide",
            TEXT,
            BG,
        );
    }
    if h < 10 || w < 32 {
        text(chunk, 1, 2, "Resize to at least 32 x 10", TEXT, BG);
        text(chunk, 1, 3, "Q quit / ? controls", TEXT, BG);
        return;
    }
    let rows = h - 7;
    let ph = rows * 2;
    let mut pixels = vec![[0.0; 3]; w * ph];
    let forward = world.direction();
    let right = [world.yaw.cos(), 0.0, -world.yaw.sin()];
    let up = [
        -world.yaw.sin() * world.pitch.sin(),
        world.pitch.cos(),
        -world.yaw.cos() * world.pitch.sin(),
    ];
    let focal = w as f64 * 0.62;
    let target = world.target().map(|hit| hit.cell);
    for y in 0..ph {
        for x in 0..w {
            let sx = (x as f64 + 0.5 - w as f64 * 0.5) / focal;
            let sy = (ph as f64 * 0.5 - y as f64 - 0.5) / focal;
            let mut direction: Vec3 =
                std::array::from_fn(|i| forward[i] + right[i] * sx + up[i] * sy);
            let length = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
            direction = direction.map(|v| v / length);
            let sky = sky(direction, world);
            let rgb = if let Some(hit) = world.ray(world.eye(), direction, 58.0, true) {
                let mut surface = texture(&hit, world);
                if target == Some(hit.cell) {
                    let fractions = hit.point.map(|v| v.rem_euclid(1.0));
                    let edge_count = fractions
                        .iter()
                        .filter(|&&f| !(0.025..=0.975).contains(&f))
                        .count();
                    if edge_count >= 2 {
                        surface = [248.0, 244.0, 209.0];
                    }
                }
                mix(surface, sky, ((hit.distance - 22.0) / 44.0).clamp(0.0, 0.8))
            } else {
                sky
            };
            pixels[y * w + x] = rgb;
        }
    }
    tool(&mut pixels, w, ph, world.swing);
    for y in 0..rows {
        for x in 0..w {
            let style = Style::default()
                .fg(color(pixels[y * 2 * w + x]))
                .bg(color(pixels[(y * 2 + 1) * w + x]))
                .to_render_style();
            let _ = chunk.set_char(x as u16, (y + 1) as u16, '▀', style);
        }
    }
    let cross = Style::default()
        .fg(Color::Rgb(255, 255, 240))
        .bg(color(pixels[(ph / 2) * w + w / 2]))
        .bold();
    let _ = chunk.set_char(
        (w / 2) as u16,
        (rows / 2 + 1) as u16,
        '+',
        cross.to_render_style(),
    );
    let target_label = world.target().map_or_else(
        || "No block in reach".into(),
        |hit| format!("{}  /  {:.1}m", hit.block.name(), hit.distance),
    );
    text(chunk, 2, 2, &format!(" {target_label} "), TEXT, BG);
    if w >= 90 {
        text(
            chunk,
            w as u16 - 27,
            2,
            &format!(
                " XYZ {:02.0} / {:02.0} / {:02.0} ",
                world.player[0], world.player[1], world.player[2]
            ),
            TEXT,
            BG,
        );
    }
    let bottom = (h - 6) as u16;
    let slot_width = (w / 8).clamp(4, 10);
    let left = (w - slot_width * 8) / 2;
    text(
        chunk,
        left as u16,
        bottom,
        "♥ ♥ ♥ ♥ ♥ ♥ ♥ ♥ ♥ ♥",
        Color::Rgb(229, 79, 65),
        BG,
    );
    if w >= 72 {
        text(
            chunk,
            (left + slot_width * 8 - 22) as u16,
            bottom,
            if world.flying {
                "FLYING / Space up C down"
            } else {
                "CREATIVE / infinite blocks"
            },
            Color::Rgb(190, 189, 150),
            BG,
        );
    }
    for (i, block) in HOTBAR.into_iter().enumerate() {
        let selected = i == world.selected;
        let panel = if selected {
            Color::Rgb(92, 92, 70)
        } else {
            Color::Rgb(48, 50, 43)
        };
        let edge = if selected {
            Color::Rgb(240, 228, 167)
        } else {
            Color::Rgb(103, 107, 91)
        };
        let x = (left + i * slot_width) as u16;
        let _ = chunk.fill(
            x,
            bottom + 1,
            slot_width as u16 - 1,
            3,
            ' ',
            Style::default().bg(panel).to_render_style(),
        );
        text(
            chunk,
            x,
            bottom + 1,
            &format!("{}{}", if selected { ">" } else { " " }, i + 1),
            edge,
            panel,
        );
        let rgb = base(block);
        let _ = chunk.set_char(
            x + 2,
            bottom + 1,
            '▀',
            Style::default()
                .fg(color(rgb))
                .bg(color(scale(rgb, 0.65)))
                .to_render_style(),
        );
        let short = match block {
            Block::Planks => "Plank",
            Block::Leaves => "Leaf",
            Block::Wood => "Log",
            _ => block.name(),
        };
        let name: String = short.chars().take(slot_width - 2).collect();
        text(
            chunk,
            x + 1,
            bottom + 2,
            &name,
            if selected {
                TEXT
            } else {
                Color::Rgb(167, 171, 151)
            },
            panel,
        );
        text(
            chunk,
            x + 1,
            bottom + 3,
            &"─".repeat(slot_width - 3),
            edge,
            panel,
        );
    }
    let caption = if world.notice_time > 0.0 {
        world.notice.clone()
    } else {
        format!(
            "{} selected  /  mined {}  /  placed {}",
            HOTBAR[world.selected].name(),
            world.mined,
            world.placed
        )
    };
    text(
        chunk,
        1,
        bottom + 4,
        &caption,
        Color::Rgb(194, 205, 163),
        BG,
    );
    text(
        chunk,
        1,
        bottom + 5,
        if w >= 110 {
            "WASD walk   Arrows look   Space jump   E mine / R place   1-8 blocks   F fly   N day/night   ? help   Q quit"
        } else {
            "WASD move  Arrows look  E mine  R place  1-8 blocks  ? help  Q quit"
        },
        Color::Rgb(150, 158, 134),
        BG,
    );
}

fn tool(pixels: &mut [Rgb], width: usize, height: usize, swing: f64) {
    // A deliberately low-resolution, stepped pickaxe and sleeve, drawn in screen space.
    let unit = (width as f64 / 130.0)
        .min(height as f64 / 60.0)
        .clamp(0.35, 2.0);
    let ox = width as f64 - 27.0 * unit - swing * 5.0;
    let oy = height as f64 - 27.0 * unit + (swing * std::f64::consts::PI).sin() * 6.0;
    for y in 0..height {
        for x in width / 2..width {
            let u = (x as f64 - ox) / unit;
            let v = (y as f64 - oy) / unit;
            let rgb = if (18.0..30.0).contains(&u) && v > 24.0 {
                Some([56.0, 157.0, 167.0])
            } else if (16.0..25.0).contains(&u) && (18.0..26.0).contains(&v) {
                Some([193.0, 140.0, 98.0])
            } else if (4.0..25.0).contains(&v) && (u - v * 0.65).abs() < 1.9 {
                Some([113.0, 79.0, 40.0])
            } else if (1.0..7.0).contains(&u) && (1.0..8.0).contains(&v)
                || (5.0..16.0).contains(&u) && (0.0..4.0).contains(&v)
                || (13.0..18.0).contains(&u) && (2.0..10.0).contains(&v)
            {
                Some([183.0, 194.0, 187.0])
            } else {
                None
            };
            if let Some(rgb) = rgb {
                pixels[y * width + x] = rgb;
            }
        }
    }
}
