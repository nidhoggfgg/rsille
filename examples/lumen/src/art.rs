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
