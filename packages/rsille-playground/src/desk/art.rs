//! Sample-colored braille graphics. No terminal escape sequences or assets.
use super::{ui::Palette, Msg, State};
use rsille::canvas::Canvas;
use rsille::tui::prelude::*;
use std::f64::consts::TAU;

pub(super) fn orbit(
    s: &State,
    p: Palette,
) -> CanvasWidget<Msg, impl Fn(&mut Canvas, CanvasContext)> {
    let time = s.time;
    let pointer = s.pointer;
    let ripple_age = s.ripple_age;
    canvas(move |ink: &mut Canvas, ctx: CanvasContext| {
        ink.set_background(p.card);
        let w = f64::from(ctx.sample_width());
        let h = f64::from(ctx.sample_height());
        let center = (w * 0.5, h * 0.5);
        let bend = pointer
            .map(|(x, y)| ((x - 0.5) * w * 0.13, (y - 0.5) * h * 0.13))
            .unwrap_or((0.0, 0.0));
        // Deterministic background points keep the scene calm as it moves.
        for i in 0..32 {
            let x = ((i * 73 + 19) % 997) as f64 / 997.0 * (w - 1.0);
            let y = ((i * 131 + 31) % 991) as f64 / 991.0 * (h - 1.0);
            ink.set_color(x, y, p.line);
        }
        for orbit in 0..4 {
            let angle = -0.6 + f64::from(orbit) * 0.52 + time * 0.045;
            let radius = 0.28 + f64::from(orbit) * 0.035;
            for i in 0..300 {
                let a = f64::from(i) / 300.0 * TAU;
                let (x, y) = (a.cos() * w * radius, a.sin() * h * 0.16);
                let point = (
                    center.0 + x * angle.cos() - y * angle.sin() + bend.0,
                    center.1 + x * angle.sin() * 0.56 + y * angle.cos() + bend.1,
                );
                let color = match orbit {
                    0 => p.mint,
                    1 => p.line,
                    2 => p.violet,
                    _ => p.line,
                };
                ink.set_color(point.0, point.1, color);
            }
            // Orbiting comet with a softly fading tail.
            for tail in 0..22 {
                let a = time * (0.65 + f64::from(orbit) * 0.12) + f64::from(orbit) * 1.8
                    - f64::from(tail) * 0.024;
                let (x, y) = (a.cos() * w * radius, a.sin() * h * 0.16);
                let color = p.line.interpolate(
                    if orbit % 2 == 0 { p.mint } else { p.peach },
                    1.0 - f64::from(tail) / 24.0,
                );
                ink.set_color(
                    center.0 + x * angle.cos() - y * angle.sin() + bend.0,
                    center.1 + x * angle.sin() * 0.56 + y * angle.cos() + bend.1,
                    color,
                );
            }
        }
        // A small breathing core; sample coordinates preserve smooth movement.
        for i in 0..60 {
            let angle = f64::from(i) / 60.0 * TAU;
            let r = 2.8 + (time * 1.3).sin() * 0.5;
            ink.set_color(
                center.0 + bend.0 + angle.cos() * r,
                center.1 + bend.1 + angle.sin() * r,
                p.peach,
            );
        }
        if ripple_age < 1.5 {
            let (x, y) = pointer.unwrap_or((0.5, 0.5));
            let radius = ripple_age * w * 0.42;
            let color = p
                .mint
                .interpolate(p.line, (ripple_age / 1.5).clamp(0.0, 1.0));
            for i in 0..180 {
                let angle = f64::from(i) / 180.0 * TAU;
                ink.set_color(
                    x * w + radius * angle.cos(),
                    y * h + radius * angle.sin() * 0.8,
                    color,
                );
            }
        }
    })
    .viewport(CanvasViewport::new(0.0, 0.0, 1.0, 1.0))
    .key("orbit")
    .on_pointer(Msg::Pointer)
}

pub(super) fn signal(
    s: &State,
    p: Palette,
) -> CanvasWidget<Msg, impl Fn(&mut Canvas, CanvasContext)> {
    let time = s.time;
    canvas(move |ink: &mut Canvas, ctx: CanvasContext| {
        ink.set_background(p.card);
        let w = ctx.sample_width();
        let h = f64::from(ctx.sample_height());
        for x in (0..w).step_by(8) {
            for y in [h * 0.22, h * 0.5, h * 0.78] {
                ink.set_color(f64::from(x), y, p.line);
            }
        }
        for x in 0..w {
            let u = f64::from(x) / f64::from(w.max(1));
            let phase = u * 12.0 + time * 0.42;
            let y = h * (0.48 + phase.sin() * 0.18 + (phase * 2.3).cos() * 0.065 + u * 0.13);
            ink.set_color(f64::from(x), y, p.mint);
            ink.set_color(f64::from(x), y - 0.7, p.mint);
            let y2 = h * (0.32 + (phase - 1.5).sin() * 0.13 + u * 0.08);
            ink.set_color(f64::from(x), y2, p.violet.interpolate(p.line, 0.35));
            if x % 3 == 0 {
                for yfill in (2..y as i32).step_by(4) {
                    ink.set_color(x, yfill, p.line);
                }
            }
        }
    })
    .key("signal-canvas")
}

/// A projected, depth-lit mesh with two topologies and a pointer-driven camera.
pub(super) fn space(
    s: &State,
    p: Palette,
    backdrop: bool,
) -> CanvasWidget<Msg, impl Fn(&mut Canvas, CanvasContext)> {
    let time = s.desk.time;
    let morph = s.desk.morph;
    let pointer = s.desk.pointer.unwrap_or((0.5, 0.5));
    let pulse = time - s.desk.pulse;
    canvas(move |ink: &mut Canvas, ctx: CanvasContext| {
        let bg = if backdrop { p.bg } else { p.card };
        ink.set_background(bg);
        let (w, h) = (
            f64::from(ctx.sample_width()),
            f64::from(ctx.sample_height()),
        );
        let center = (w * 0.5, h * 0.5);
        for i in 0..65 {
            let x = ((i * 173 + 47) % 997) as f64 / 997.0 * (w - 1.0);
            let y = ((i * 97 + 131) % 991) as f64 / 991.0 * (h - 1.0);
            let glint = (time * 0.6 + f64::from(i)).sin() * 0.15 + 0.4;
            ink.set_color(x, y, bg.interpolate(p.line, glint));
        }
        let yaw = time * 0.23 + (pointer.0 - 0.5) * 0.8;
        let tilt = 0.52 + (pointer.1 - 0.5) * 0.5;
        let project = |i: usize, j: usize| {
            let u = i as f64 / 28.0 * TAU;
            let v = j as f64 / 12.0 * TAU;
            let radius = 1.15 + 0.42 * v.cos();
            let torus = (radius * u.cos(), radius * u.sin(), 0.42 * v.sin());
            let helix = (
                (u * 2.0 + v).cos() * 0.65,
                (u / TAU - 0.5) * 2.8,
                (u * 2.0 + v).sin() * 0.65,
            );
            let mix = |a: f64, b: f64| a + (b - a) * morph;
            let (x, y, z) = (
                mix(torus.0, helix.0),
                mix(torus.1, helix.1),
                mix(torus.2, helix.2),
            );
            let (x, z) = (
                x * yaw.cos() + z * yaw.sin(),
                -x * yaw.sin() + z * yaw.cos(),
            );
            let (y, z) = (
                y * tilt.cos() - z * tilt.sin(),
                y * tilt.sin() + z * tilt.cos(),
            );
            let perspective = 3.8 / (4.5 - z);
            let scale = (w * 0.30).min(h * 0.40);
            (
                center.0 + x * scale * perspective,
                center.1 + y * scale * perspective,
                z,
            )
        };
        // Draw rear samples first; nearby points retain the brighter colors.
        let mut points = (0..28)
            .flat_map(|i| (0..12).map(move |j| (i, j, project(i, j))))
            .collect::<Vec<_>>();
        points.sort_by(|a, b| a.2 .2.total_cmp(&b.2 .2));
        for (i, j, (x, y, z)) in points {
            let light = ((z + 1.6) / 3.2).clamp(0.0, 1.0);
            let accent = p.violet.interpolate(p.mint, light);
            let color = if backdrop {
                bg.interpolate(accent, 0.12 + light * 0.25)
            } else {
                p.line.interpolate(accent, 0.25 + light * 0.75)
            };
            let next = project((i + 1) % 28, j);
            // The helix is open at both ends, while the torus closes on itself.
            if (i < 27 || morph < 0.05) && j % 2 == 0 {
                colored_line(ink, (x, y), (next.0, next.1), bg.interpolate(color, 0.6));
            }
            if j % 3 == 0 {
                let next = project(i, (j + 1) % 12);
                colored_line(ink, (x, y), (next.0, next.1), bg.interpolate(color, 0.5));
            }
            ink.set_color(x, y, color);
            if !backdrop && light > 0.72 {
                ink.set_color(x + 0.8, y, color);
            }
        }
        // An expanding wave responds to clicks in either the field or the lens.
        if (0.0..1.8).contains(&pulse) {
            let radius = pulse * w * 0.6;
            let color = bg.interpolate(p.peach, (1.0 - pulse / 1.8) * 0.75);
            for i in 0..220 {
                let a = f64::from(i) / 220.0 * TAU;
                ink.set_color(
                    pointer.0 * w + a.cos() * radius,
                    pointer.1 * h + a.sin() * radius,
                    color,
                );
            }
        }
    })
    .viewport(CanvasViewport::new(0.0, 0.0, 1.0, 1.0))
    .on_pointer(|pointer| Msg::Desk(super::desktop::Action::Pointer(pointer)))
}

fn colored_line(ink: &mut Canvas, a: (f64, f64), b: (f64, f64), color: Color) {
    let steps = ((a.0 - b.0).abs().max((a.1 - b.1).abs()).ceil() as usize).clamp(1, 180);
    for i in (0..=steps).step_by(2) {
        let t = i as f64 / steps as f64;
        ink.set_color(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, color);
    }
}

pub(super) fn desk_signal(
    s: &State,
    p: Palette,
) -> CanvasWidget<Msg, impl Fn(&mut Canvas, CanvasContext)> {
    let time = s.desk.time;
    canvas(move |ink: &mut Canvas, ctx: CanvasContext| {
        ink.set_background(p.card);
        let (w, h) = (
            f64::from(ctx.sample_width()),
            f64::from(ctx.sample_height()),
        );
        for x in (0..ctx.sample_width()).step_by(6) {
            for y in [h * 0.25, h * 0.5, h * 0.75] {
                ink.set_color(f64::from(x), y, p.line);
            }
        }
        let sweep = (time * 0.17).fract();
        for x in 0..ctx.sample_width() {
            let u = f64::from(x) / w.max(1.0);
            let phase = u * 18.0 - time * 2.0;
            let y = h * (0.5 + phase.sin() * 0.18 + (phase * 2.1).cos() * 0.08);
            let light = 1.0 - ((sweep - u + 1.0) % 1.0) * 0.8;
            ink.set_color(f64::from(x), y, p.line.interpolate(p.mint, light));
            ink.set_color(
                f64::from(x),
                h * (0.36 + (phase * 0.7 + 0.9).sin() * 0.13),
                p.violet,
            );
            if (u - sweep).abs() < 0.012 {
                for y in 0..ctx.sample_height() {
                    ink.set_color(x, y, p.line);
                }
            }
        }
    })
    .key("desk-signal-canvas")
}
