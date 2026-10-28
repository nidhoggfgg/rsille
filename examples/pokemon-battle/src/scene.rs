//! Procedural Pokémon meshes and move choreography in an open meadow.
use crate::{
    battle::{Battle, Move},
    geometry::Model,
    math::{random, smooth, V3},
    raster::{Camera, Raster},
};
use std::f32::consts::{PI, TAU};

const YELLOW: [f32; 3] = [1.0, 0.72, 0.065];
const BLACK: [f32; 3] = [0.035, 0.028, 0.045];
const BLUE: [f32; 3] = [0.14, 0.45, 0.74];

const LEFT: V3 = V3(-2.05, 0.0, 0.85);
const RIGHT: V3 = V3(1.85, 0.0, -0.75);
const FACING: f32 = 1.96;

fn muzzle(side: f32, water: f32) -> V3 {
    V3(side * 0.79, 2.53 - water * 0.64, 0.68 + water * 0.22)
}

fn front(yaw: f32) -> V3 {
    V3(yaw.sin(), 0.0, yaw.cos())
}
fn local(p: V3, yaw: f32) -> V3 {
    let (s, c) = yaw.sin_cos();
    V3(p.0 * c + p.2 * s, p.1, p.2 * c - p.0 * s)
}
const WHITE: [f32; 3] = [0.98, 0.98, 0.89];

pub fn render(
    raster: &mut Raster,
    width: usize,
    height: usize,
    view_height: usize,
    battle: Battle,
    camera: Camera,
) {
    raster.begin(width, height, view_height, camera);
    let t = battle.time as f32;
    let electric = battle
        .strength(Move::Thunderbolt)
        .max(battle.strength(Move::ThunderWave));
    let motion = |attack| {
        battle
            .attack()
            .filter(|(m, _)| *m == attack)
            .map_or(0.0, |(_, p)| (p * PI).sin().powi(2))
    };
    let dash = motion(Move::QuickAttack) * 3.2 + motion(Move::IronTail) * 2.7;
    let tackle = motion(Move::Tackle) * 3.1;
    let water = battle.strength(Move::HydroPump);
    let mut world = Model {
        raster,
        origin: V3::ZERO,
        yaw: 0.0,
    };
    crate::environment::draw(&mut world, t);
    let axis = front(FACING);
    let recoil_left = water * (t * 42.0).sin() * 0.06;
    let recoil_right = electric * (t * 45.0).sin() * 0.065;
    let left = LEFT
        + axis * (electric * 0.16 + recoil_left + dash)
        + V3(0.0, 0.04 + (t * 3.4).sin().abs() * 0.045, 0.0);
    let right = RIGHT - axis * (water * 0.16 + recoil_right + tackle)
        + V3(
            0.0,
            0.04 + (t * 3.0 + 1.0).sin().abs() * 0.035 - battle.strength(Move::Withdraw) * 0.22,
            0.0,
        );
    for (p, r) in [(left, 0.83), (right, 1.08)] {
        for i in 0..5 {
            let f = i as f32 / 5.0;
            world.disc(
                V3(p.0, 0.008 + i as f32 * 0.001, p.2),
                r * (1.0 - f * 0.38),
                0.64 * (1.0 - f * 0.38),
                [0.10 - f * 0.035, 0.19 - f * 0.065, 0.065 - f * 0.025],
            );
        }
    }
    pikachu(
        &mut Model {
            raster: world.raster,
            origin: left,
            yaw: FACING
                + battle
                    .attack()
                    .filter(|(m, _)| *m == Move::IronTail)
                    .map_or(0.0, |(_, p)| p * TAU),
        },
        t,
        electric,
    );
    blastoise(
        &mut Model {
            raster: world.raster,
            origin: right,
            yaw: FACING - PI,
        },
        t,
        water,
    );
    if battle.guard {
        world.ring(right + V3(0.0, 0.35, 0.0), 0.88, 0.035, [0.39, 0.84, 0.91]);
        world.ring(right + V3(0.0, 1.1, 0.0), 0.83, 0.025, [0.39, 0.84, 0.91]);
    }
    if battle.paralyzed {
        for i in 0..4 {
            let a = t * 2.0 + i as f32 * TAU / 4.0;
            let p = right + V3(a.cos() * 0.8, 1.1 + (a * 2.0).sin() * 0.6, a.sin() * 0.7);
            world.line(p, p + V3(0.08, 0.16, 0.0), [1.0, 0.85, 0.13]);
        }
    }
    effects(&mut world, battle, left, right);
}

fn pikachu(m: &mut Model<'_>, t: f32, charge: f32) {
    // Broad, flat zigzags keep the lightning silhouette legible from the back.
    let wag = (t * 3.0).sin() * 0.055;
    for (outline, color) in [
        (
            [(0.02, 0.44), (0.22, 0.42), (0.46, 0.64), (0.27, 0.70)],
            [0.43, 0.22, 0.09],
        ),
        (
            [(0.27, 0.70), (0.46, 0.64), (0.65, 1.06), (0.38, 1.11)],
            YELLOW,
        ),
        (
            [(0.38, 1.11), (0.65, 1.06), (0.35, 1.52), (0.08, 1.49)],
            YELLOW,
        ),
        (
            [(0.08, 1.49), (0.35, 1.52), (1.37, 2.00), (0.80, 2.22)],
            YELLOW,
        ),
    ] {
        m.extrude(
            &outline.map(|(x, y)| V3(x + wag * y, y, -0.49 - x * 0.35)),
            V3(0.0, 0.0, -0.105),
            color,
        );
    }
    // A low, pear-shaped body flows into a broad cheeked head, without a neck.
    m.loft(
        &[
            (V3(0.0, 0.15, 0.0), 0.0, 0.0),
            (V3(0.0, 0.24, 0.0), 0.40, 0.32),
            (V3(0.0, 0.48, 0.0), 0.57, 0.43),
            (V3(0.0, 0.72, 0.0), 0.60, 0.45),
            (V3(0.0, 0.96, 0.0), 0.51, 0.39),
            (V3(0.0, 1.24, 0.02), 0.40, 0.30),
            (V3(0.0, 1.40, 0.03), 0.0, 0.0),
        ],
        YELLOW,
    );
    for side in [-1.0, 1.0] {
        m.ball(V3(side * 0.35, 0.12, 0.17), V3(0.25, 0.14, 0.34), YELLOW);
        let hand = V3(
            side * (0.50 + charge * 0.13),
            0.73 + charge * 0.23,
            0.35 + charge * 0.16,
        );
        m.tube(V3(side * 0.42, 1.03, 0.12), hand, 0.17, 0.12, YELLOW, false);
        m.ball(hand, V3(0.14, 0.17, 0.16), YELLOW);
    }
    for (y, width, depth) in [(0.66, 0.54, 0.45), (0.94, 0.52, 0.40)] {
        // Follow the back surface, so the stripes read as markings, not knobs.
        for i in 0..12 {
            let point = |j: usize, yy: f32| {
                let x = (j as f32 / 12.0 - 0.5) * 0.72;
                V3(x, yy, -depth * (1.0 - (x / width).powi(2)).sqrt() - 0.012)
            };
            m.quad(
                [
                    point(i, y - 0.045),
                    point(i + 1, y - 0.045),
                    point(i + 1, y + 0.045),
                    point(i, y + 0.045),
                ],
                [0.42, 0.23, 0.08],
                false,
            );
        }
    }
    m.loft(
        &[
            (V3(0.0, 1.10, 0.08), 0.0, 0.0),
            (V3(0.0, 1.20, 0.08), 0.44, 0.31),
            (V3(0.0, 1.38, 0.08), 0.64, 0.46),
            (V3(0.0, 1.58, 0.08), 0.65, 0.46),
            (V3(0.0, 1.79, 0.06), 0.59, 0.40),
            (V3(0.0, 1.97, 0.03), 0.43, 0.28),
            (V3(0.0, 2.04, 0.02), 0.25, 0.17),
            (V3(0.0, 2.07, 0.02), 0.0, 0.0),
        ],
        YELLOW,
    );
    for side in [-1.0, 1.0] {
        let lean = if side < 0.0 { 0.12 } else { 0.0 };
        let sway = (t * 3.0 + side).sin() * 0.025;
        let ear = |x: f32, y: f32, z: f32| V3(side * (x + lean * (y - 1.9) + sway), y, z);
        let boundary = (ear(0.63, 2.57 - lean, 0.015), 0.105, 0.056);
        m.loft(
            &[
                (ear(0.37, 1.88, 0.04), 0.11, 0.085),
                (ear(0.47, 2.13, 0.035), 0.145, 0.080),
                (ear(0.57, 2.40 - lean, 0.025), 0.13, 0.066),
                boundary,
            ],
            YELLOW,
        );
        m.loft(
            &[
                boundary,
                (ear(0.69, 2.75 - lean, 0.0), 0.070, 0.040),
                (ear(0.72, 2.88 - lean, -0.01), 0.0, 0.0),
            ],
            BLACK,
        );
        m.ball(
            V3(side * 0.255, 1.66, 0.476),
            V3(0.087, 0.117, 0.043),
            BLACK,
        );
        m.ball(
            V3(side * 0.255 - 0.022, 1.704, 0.511),
            V3(0.027, 0.033, 0.012),
            WHITE,
        );
        m.ball(
            V3(side * 0.515, 1.39, 0.366),
            V3(0.135, 0.115, 0.070),
            [0.94, 0.13, 0.11],
        );
    }
    m.ball(V3(0.0, 1.51, 0.546), V3(0.043, 0.027, 0.020), BLACK);
    for side in [-1.0, 1.0] {
        m.tube(
            V3(0.0, 1.39, 0.545),
            V3(side * 0.068, 1.35, 0.535),
            0.012,
            0.012,
            BLACK,
            false,
        );
        m.tube(
            V3(side * 0.068, 1.35, 0.535),
            V3(side * 0.13, 1.39, 0.527),
            0.012,
            0.010,
            BLACK,
            false,
        );
    }
}

fn blastoise(m: &mut Model<'_>, t: f32, water: f32) {
    // A wide shell and low, planted limbs give Blastoise its heavy silhouette.
    m.ball(
        V3(0.0, 1.17, -0.25),
        V3(1.00, 1.05, 0.60),
        [0.86, 0.81, 0.65],
    );
    m.ball(
        V3(0.0, 1.20, -0.44),
        V3(0.91, 0.96, 0.58),
        [0.40, 0.24, 0.15],
    );
    // The central scute and radial seams sit on the curved shell surface.
    let shell = |x: f32, y: f32| {
        V3(
            x,
            y,
            -0.44
                - 0.59
                    * (1.0 - (x / 0.91).powi(2) - ((y - 1.20) / 0.96).powi(2))
                        .max(0.0)
                        .sqrt(),
        )
    };
    let scute = [
        (-0.34, 1.48),
        (0.0, 1.75),
        (0.34, 1.48),
        (0.34, 0.98),
        (0.0, 0.72),
        (-0.34, 0.98),
    ];
    for i in 0..scute.len() {
        let (a, b) = (scute[i], scute[(i + 1) % scute.len()]);
        for j in 0..8 {
            let point = |f: f32| shell(a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f);
            m.tube(
                point(j as f32 / 8.0),
                point((j + 1) as f32 / 8.0),
                0.018,
                0.018,
                [0.23, 0.14, 0.09],
                false,
            );
        }
        let end = (a.0 * 2.1, 1.20 + (a.1 - 1.20) * 1.65);
        for j in 0..8 {
            let point = |f: f32| shell(a.0 + (end.0 - a.0) * f, a.1 + (end.1 - a.1) * f);
            m.tube(
                point(j as f32 / 8.0),
                point((j + 1) as f32 / 8.0),
                0.016,
                0.016,
                [0.23, 0.14, 0.09],
                false,
            );
        }
    }
    for side in [-1.0, 1.0] {
        m.ball(
            V3(side * 0.73, 1.98, -0.24),
            V3(0.26, 0.23, 0.29),
            [0.09, 0.23, 0.34],
        );
        let a = V3(side * 0.75, 2.04, -0.28);
        let b = muzzle(side, water);
        let direction = (b - a).unit();
        m.pipe(a, b, 0.175, [0.53, 0.66, 0.73]);
        m.pipe(
            b - direction * 0.12,
            b + direction * 0.03,
            0.205,
            [0.66, 0.77, 0.81],
        );
    }
    m.ball(V3(0.0, 1.10, 0.06), V3(0.84, 0.87, 0.56), BLUE);
    m.ball(
        V3(0.0, 1.08, 0.44),
        V3(0.70, 0.77, 0.23),
        [0.87, 0.75, 0.48],
    );
    for y in [0.64_f32, 0.98, 1.32, 1.61] {
        let half_width = 0.69 * (1.0 - ((y - 1.08) / 0.77).powi(2)).sqrt();
        for i in 0..20 {
            let point = |j: usize| {
                let x = (j as f32 / 10.0 - 1.0) * half_width;
                let yy = y - 0.035 * (1.0 - (x / half_width).powi(2));
                let z = 0.44
                    + 0.238
                        * (1.0 - (x / 0.70).powi(2) - ((yy - 1.08) / 0.77).powi(2))
                            .max(0.0)
                            .sqrt();
                V3(x, yy, z)
            };
            m.tube(
                point(i),
                point(i + 1),
                0.012,
                0.012,
                [0.57, 0.45, 0.29],
                false,
            );
        }
    }
    m.tube(
        V3(0.0, 0.52, -0.66),
        V3(0.0, 0.36, -1.07),
        0.19,
        0.04,
        BLUE,
        false,
    );
    for side in [-1.0, 1.0] {
        m.ball(V3(side * 0.58, 0.38, 0.02), V3(0.39, 0.40, 0.39), BLUE);
        m.ball(V3(side * 0.62, 0.16, 0.29), V3(0.35, 0.18, 0.40), BLUE);
        for claw in [-1.0, 0.0, 1.0] {
            m.tube(
                V3(side * 0.62 + claw * 0.16, 0.13, 0.57),
                V3(side * 0.62 + claw * 0.16, 0.10, 0.75),
                0.072,
                0.014,
                WHITE,
                false,
            );
        }
        let elbow = V3(side * 1.00, 1.37, 0.13);
        let hand = V3(side * 1.04, 1.02 + water * 0.1, 0.48);
        m.ball(V3(side * 0.76, 1.57, 0.07), V3(0.35, 0.36, 0.35), BLUE);
        m.tube(V3(side * 0.78, 1.56, 0.09), elbow, 0.31, 0.29, BLUE, false);
        m.tube(elbow, hand, 0.29, 0.25, BLUE, false);
        m.ball(hand, V3(0.27, 0.24, 0.28), BLUE);
        for claw in [-1.0, 0.0, 1.0] {
            m.tube(
                hand + V3(claw * 0.15, -0.05, 0.21),
                hand + V3(claw * 0.16, -0.10, 0.38),
                0.065,
                0.012,
                WHITE,
                false,
            );
        }
    }
    let nod = (t * 2.8).sin() * 0.018;
    let head = |x: f32, y: f32, z: f32| V3(x, y + nod, z);
    m.loft(
        &[
            (head(0.0, 1.75, 0.17), 0.0, 0.0),
            (head(0.0, 1.84, 0.20), 0.39, 0.32),
            (head(0.0, 2.03, 0.20), 0.58, 0.45),
            (head(0.0, 2.25, 0.17), 0.60, 0.43),
            (head(0.0, 2.41, 0.10), 0.49, 0.33),
            (head(0.0, 2.48, 0.07), 0.32, 0.24),
            (head(0.0, 2.51, 0.06), 0.0, 0.0),
        ],
        BLUE,
    );
    m.ball(head(0.0, 1.99, 0.48), V3(0.48, 0.225, 0.32), BLUE);
    m.ball(
        head(0.0, 1.84, 0.47),
        V3(0.40, 0.10, 0.24),
        [0.23, 0.49, 0.72],
    );
    for side in [-1.0, 1.0] {
        m.loft(
            &[
                (head(side * 0.42, 2.33, 0.02), 0.13, 0.14),
                (head(side * 0.53, 2.49, -0.01), 0.09, 0.09),
                (head(side * 0.57, 2.58, -0.03), 0.0, 0.0),
            ],
            BLUE,
        );
        // Slanted eyes and an overhanging brow replace the round, startled face.
        let eye = |x, y, z| head(side * x, y, z);
        m.extrude(
            &[
                eye(0.14, 2.26, 0.608),
                eye(0.46, 2.34, 0.461),
                eye(0.43, 2.17, 0.515),
                eye(0.18, 2.16, 0.617),
            ],
            V3(0.0, 0.0, -0.035),
            WHITE,
        );
        m.ball(
            eye(0.265, 2.224, 0.586),
            V3(0.054, 0.072, 0.028),
            [0.30, 0.13, 0.14],
        );
        m.ball(eye(0.255, 2.223, 0.610), V3(0.025, 0.057, 0.014), BLACK);
        m.ball(eye(0.248, 2.249, 0.622), V3(0.012, 0.015, 0.007), WHITE);
        m.tube(
            eye(0.13, 2.285, 0.611),
            eye(0.48, 2.365, 0.456),
            0.045,
            0.050,
            BLUE,
            false,
        );
        m.tube(
            eye(0.32, 1.875, 0.716),
            eye(0.30, 1.81, 0.724),
            0.036,
            0.004,
            WHITE,
            false,
        );
        m.ball(
            eye(0.12, 2.065, 0.785),
            V3(0.022, 0.014, 0.009),
            [0.06, 0.19, 0.28],
        );
    }
    for i in 0..16 {
        let point = |j: usize| {
            let x = (j as f32 / 8.0 - 1.0) * 0.37;
            head(
                x,
                1.875 + (x / 0.37).powi(2) * 0.025,
                0.48 + 0.312
                    * (1.0 - (x / 0.48).powi(2) - ((1.875 - 1.99) / 0.225_f32).powi(2))
                        .max(0.0)
                        .sqrt(),
            )
        };
        m.tube(
            point(i),
            point(i + 1),
            0.012,
            0.012,
            [0.06, 0.17, 0.24],
            false,
        );
    }
}

fn effects(m: &mut Model<'_>, battle: Battle, left: V3, right: V3) {
    let Some((attack, progress)) = battle.attack() else {
        return;
    };
    let strength = battle.strength(attack);
    let time = battle.time as f32;
    match attack {
        Move::Thunderbolt | Move::ThunderWave => {
            let source = left + local(V3(0.0, 1.42, 0.53), FACING);
            let target = right + local(V3(0.0, 1.45, 0.5), FACING - PI);
            let reach = smooth(progress / 0.28);
            for strand in 0..3 {
                let mut prev = source;
                for i in 1..17 {
                    let f = i as f32 / 16.0;
                    let jitter = (time * 17.0).floor() as usize;
                    let deviation =
                        (random(i + strand * 100 + jitter * 17) - 0.5) * 0.48 * strength;
                    let p = source.lerp(target, f * reach)
                        + V3(0.0, deviation, (strand as f32 - 1.0) * 0.10);
                    m.tube(
                        prev,
                        p,
                        0.027 * strength,
                        0.027 * strength,
                        [1.0, 0.75, 0.08],
                        true,
                    );
                    m.line(prev, p, [1.0, 0.98, 0.68]);
                    prev = p;
                }
            }
            // Orbiting sparks at the cheeks and the actual 3D impact point.
            for i in 0..18 {
                let f = (time * 1.5 + random(i)).fract();
                let a = random(i + 30) * TAU;
                let center = if i < 7 { source } else { target };
                let p = center
                    + V3(
                        a.cos() * f * 0.7,
                        (a * 2.0).sin() * f * 0.55,
                        a.sin() * f * 0.65,
                    ) * strength;
                let end = p + (p - center).unit() * 0.16;
                m.tube(p, end, 0.018, 0.008, [1.0, 0.88, 0.22], true);
            }
        }
        Move::HydroPump => {
            let target = left + V3(0.0, 1.05, 0.0);
            let reach = smooth(progress / 0.27);
            let end = right.lerp(target, reach);
            for side in [-1.0, 1.0] {
                let source = right + local(muzzle(side, strength), FACING - PI);
                let end = source.lerp(target, reach);
                for i in 0..17 {
                    let f = i as f32 / 16.0;
                    let next_f = ((i + 1) as f32 / 16.0).min(1.0);
                    let p = source.lerp(end, f) + V3(0.0, (f * PI).sin() * 0.10, 0.0);
                    let next = source.lerp(end, next_f) + V3(0.0, (next_f * PI).sin() * 0.10, 0.0);
                    m.tube(
                        p,
                        next,
                        0.075 * strength,
                        0.08 * strength,
                        [0.22, 0.66, 1.0],
                        true,
                    );
                    m.line(
                        p + V3(0.0, 0.035, 0.07),
                        next + V3(0.0, 0.035, 0.07),
                        [0.72, 0.96, 1.0],
                    );
                }
            }
            for i in 0..28 {
                let f = (time * 1.6 + random(i)).fract();
                let a = random(i + 91) * TAU;
                let p = end
                    + V3(
                        a.cos() * f * 0.65,
                        f * 0.9 - f * f * 1.3,
                        a.sin() * f * 0.65,
                    ) * strength;
                m.ball(p, V3(0.045, 0.065, 0.045) * strength, [0.35, 0.83, 1.0]);
            }
            for i in 0..3 {
                let f = (time * 0.9 + i as f32 / 3.0).fract();
                m.ring(
                    V3(left.0, 0.021, left.2),
                    0.15 + f * 0.9,
                    0.025 * (1.0 - f),
                    [0.22, 0.48, 0.63],
                );
            }
        }
        Move::QuickAttack | Move::IronTail | Move::Tackle => {
            let player = attack.player();
            let center = if player { left } else { right };
            let direction = front(FACING) * if player { -1.0 } else { 1.0 };
            let color = if attack == Move::IronTail {
                [0.75, 0.90, 1.0]
            } else {
                [0.95, 0.94, 0.77]
            };
            for i in 0..8 {
                let p = center
                    + direction * 0.5
                    + V3(0.0, 0.35 + i as f32 * 0.16, (i as f32 * 2.3).sin() * 0.3);
                m.line(
                    p,
                    p + direction * (0.7 * strength) + V3(0.0, 0.03, 0.0),
                    color,
                );
            }
            if (0.30..0.68).contains(&progress) {
                let target = if player { right } else { left };
                for i in 0..10 {
                    let a = i as f32 * TAU / 10.0;
                    let p = target + V3(a.cos() * 0.55, 1.0 + a.sin() * 0.55, 0.35);
                    m.line(p, p + V3(a.cos(), a.sin(), 0.0) * 0.2, color);
                }
            }
        }
        Move::Withdraw => {
            for i in 0..3 {
                m.ring(
                    right + V3(0.0, 0.3 + i as f32 * 0.4, 0.0),
                    0.8 * strength,
                    0.03,
                    [0.36, 0.85, 0.93],
                );
            }
        }
        Move::Paralyzed => {}
    }
}
