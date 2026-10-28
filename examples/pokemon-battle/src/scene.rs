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
    V3(side * 0.74, 2.61 - water * 0.80, 0.64 + water * 0.20)
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
    // The tail is an extruded lightning bolt, rather than a camera-facing sprite.
    let wag = (t * 3.0).sin() * 0.07;
    let sections = [
        (-0.36, 0.48, 0.11),
        (-0.91, 0.85, 0.13),
        (-0.68, 1.09, 0.15),
        (-1.22, 1.31, 0.21),
        (-1.22 + wag, 1.94, 0.26),
    ];
    for i in 0..sections.len() - 1 {
        let (x, y, w) = sections[i];
        let (xx, yy, ww) = sections[i + 1];
        let points = [
            V3(x - w, y, -0.36),
            V3(x + w, y, -0.36),
            V3(xx + ww, yy, -0.36),
            V3(xx - ww, yy, -0.36),
        ];
        let color = if i == 0 { [0.43, 0.22, 0.09] } else { YELLOW };
        m.quad(points, color, false);
        m.quad(points.map(|p| p + V3(0.0, 0.0, -0.16)), color, false);
        for j in 0..4 {
            let (a, b) = (points[j], points[(j + 1) % 4]);
            m.quad(
                [a, b, b + V3(0.0, 0.0, -0.16), a + V3(0.0, 0.0, -0.16)],
                color,
                false,
            );
        }
    }
    m.ball(V3(0.0, 0.79, 0.0), V3(0.57, 0.71, 0.43), YELLOW);
    for side in [-1.0, 1.0] {
        m.ball(V3(side * 0.36, 0.12, 0.16), V3(0.29, 0.15, 0.39), YELLOW);
        let hand = V3(
            side * (0.55 + charge * 0.10),
            0.84 + charge * 0.20,
            0.27 + charge * 0.19,
        );
        m.tube(V3(side * 0.43, 1.03, 0.05), hand, 0.18, 0.13, YELLOW, false);
        m.ball(hand, V3(0.17, 0.19, 0.17), YELLOW);
        // Brown back stripes are shallow solid patches on the rear of the torso.
        m.ball(
            V3(side * 0.23, 0.70, -0.385),
            V3(0.23, 0.095, 0.055),
            [0.42, 0.23, 0.08],
        );
        m.ball(
            V3(side * 0.21, 0.97, -0.37),
            V3(0.20, 0.085, 0.05),
            [0.42, 0.23, 0.08],
        );
    }
    m.ball(V3(0.0, 1.58, 0.07), V3(0.65, 0.53, 0.49), YELLOW);
    for side in [-1.0, 1.0] {
        let root = V3(side * 0.38, 1.91, 0.04);
        let middle = V3(side * (0.60 + 0.035 * (t * 3.0).sin()), 2.49, 0.0);
        let tip = V3(side * 0.69, 2.80, -0.03);
        m.tube(root, middle, 0.155, 0.115, YELLOW, false);
        m.tube(middle, tip, 0.116, 0.012, BLACK, false);
        m.ball(
            V3(side * 0.255, 1.68, 0.493),
            V3(0.105, 0.139, 0.056),
            BLACK,
        );
        m.ball(
            V3(side * 0.24 - 0.022, 1.73, 0.542),
            V3(0.032, 0.040, 0.018),
            WHITE,
        );
        m.ball(
            V3(side * 0.47, 1.41, 0.40),
            V3(0.138, 0.12, 0.067),
            [0.94, 0.13, 0.11],
        );
    }
    m.ball(V3(0.0, 1.52, 0.568), V3(0.053, 0.036, 0.025), BLACK);
    for side in [-1.0, 1.0] {
        m.tube(
            V3(0.0, 1.38, 0.553),
            V3(side * 0.075, 1.34, 0.55),
            0.015,
            0.015,
            BLACK,
            false,
        );
        m.tube(
            V3(side * 0.075, 1.34, 0.55),
            V3(side * 0.14, 1.39, 0.53),
            0.015,
            0.012,
            BLACK,
            false,
        );
    }
}

fn blastoise(m: &mut Model<'_>, t: f32, water: f32) {
    // Broad brown shell and ivory rim, with shoulder-mounted, hollow steel cannons.
    m.ball(
        V3(0.0, 1.17, -0.26),
        V3(0.94, 1.09, 0.59),
        [0.86, 0.81, 0.65],
    );
    m.ball(
        V3(0.0, 1.20, -0.43),
        V3(0.83, 0.98, 0.53),
        [0.35, 0.20, 0.13],
    );
    for side in [-1.0, 1.0] {
        m.tube(
            V3(0.0, 2.12, -0.68),
            V3(side * 0.64, 1.52, -0.81),
            0.025,
            0.025,
            [0.17, 0.12, 0.10],
            false,
        );
        m.tube(
            V3(side * 0.64, 1.52, -0.81),
            V3(side * 0.57, 0.72, -0.82),
            0.025,
            0.025,
            [0.17, 0.12, 0.10],
            false,
        );
        m.tube(
            V3(side * 0.57, 0.72, -0.82),
            V3(0.0, 0.28, -0.61),
            0.025,
            0.025,
            [0.17, 0.12, 0.10],
            false,
        );
        m.ball(
            V3(side * 0.66, 1.95, -0.22),
            V3(0.28, 0.25, 0.28),
            [0.09, 0.23, 0.34],
        );
        let a = V3(side * 0.67, 2.02, -0.25);
        let b = muzzle(side, water);
        let direction = (b - a).unit();
        m.pipe(a, b, 0.205, [0.53, 0.66, 0.73]);
        m.pipe(
            b - direction * 0.12,
            b + direction * 0.03,
            0.234,
            [0.66, 0.77, 0.81],
        );
    }
    m.ball(V3(0.0, 1.10, 0.06), V3(0.76, 0.91, 0.52), BLUE);
    m.ball(
        V3(0.0, 1.08, 0.43),
        V3(0.64, 0.79, 0.21),
        [0.87, 0.75, 0.48],
    );
    for y in [0.62_f32, 0.96, 1.30, 1.62] {
        for i in 0..14 {
            let x = -0.49 + i as f32 * 0.07;
            let z = 0.43
                + 0.218
                    * (1.0 - (x / 0.68).powi(2) - ((y - 1.08) / 0.89).powi(2))
                        .max(0.0)
                        .sqrt();
            m.line(V3(x, y, z), V3(x + 0.07, y, z), [0.57, 0.45, 0.29]);
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
        m.ball(V3(side * 0.57, 0.36, 0.04), V3(0.37, 0.37, 0.41), BLUE);
        m.ball(V3(side * 0.58, 0.16, 0.29), V3(0.35, 0.19, 0.41), BLUE);
        for claw in [-1.0, 0.0, 1.0] {
            m.tube(
                V3(side * 0.58 + claw * 0.16, 0.13, 0.57),
                V3(side * 0.58 + claw * 0.16, 0.10, 0.78),
                0.072,
                0.014,
                WHITE,
                false,
            );
        }
        let elbow = V3(side * 0.94, 1.40, 0.02);
        let hand = V3(side * 1.01, 0.97 + water * 0.1, 0.45);
        m.ball(V3(side * 0.69, 1.63, 0.01), V3(0.33, 0.37, 0.33), BLUE);
        m.tube(elbow, hand, 0.30, 0.26, BLUE, false);
        m.ball(hand, V3(0.28, 0.27, 0.30), BLUE);
        for claw in [-1.0, 0.0, 1.0] {
            m.tube(
                hand + V3(claw * 0.15, -0.05, 0.21),
                hand + V3(claw * 0.16, -0.10, 0.44),
                0.065,
                0.012,
                WHITE,
                false,
            );
        }
    }
    let nod = (t * 2.8).sin() * 0.018;
    m.ball(V3(0.0, 2.12 + nod, 0.13), V3(0.59, 0.50, 0.50), BLUE);
    m.ball(V3(0.0, 1.94 + nod, 0.44), V3(0.49, 0.26, 0.36), BLUE);
    for side in [-1.0, 1.0] {
        m.tube(
            V3(side * 0.37, 2.42, 0.0),
            V3(side * 0.50, 2.71, -0.04),
            0.13,
            0.01,
            BLUE,
            false,
        );
        m.ball(
            V3(side * 0.30, 2.23 + nod, 0.535),
            V3(0.15, 0.115, 0.046),
            WHITE,
        );
        m.ball(
            V3(side * 0.29, 2.22 + nod, 0.575),
            V3(0.065, 0.09, 0.022),
            [0.30, 0.13, 0.14],
        );
        m.ball(
            V3(side * 0.28, 2.23 + nod, 0.595),
            V3(0.032, 0.071, 0.012),
            BLACK,
        );
        m.ball(
            V3(side * 0.28 - 0.013, 2.26 + nod, 0.605),
            V3(0.016, 0.019, 0.008),
            WHITE,
        );
        m.tube(
            V3(side * 0.13, 2.37, 0.535),
            V3(side * 0.47, 2.33, 0.48),
            0.05,
            0.038,
            BLUE,
            false,
        );
        m.tube(
            V3(side * 0.36, 1.90, 0.65),
            V3(side * 0.33, 1.82, 0.68),
            0.047,
            0.004,
            WHITE,
            false,
        );
        m.ball(
            V3(side * 0.13, 2.02, 0.761),
            V3(0.025, 0.015, 0.009),
            [0.06, 0.19, 0.28],
        );
    }
    m.tube(
        V3(-0.32, 1.84, 0.68),
        V3(0.32, 1.84, 0.68),
        0.018,
        0.018,
        [0.06, 0.17, 0.24],
        false,
    );
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
