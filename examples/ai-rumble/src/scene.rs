pub use crate::motion::{Beat, PERIOD};
use crate::{
    characters, motion,
    raster::{Point, Raster, Rgb, INK, WHITE},
};
use std::f32::consts::{PI, TAU};
pub const COLORS: [Rgb; 3] = [[0.32, 0.65, 1.], [1., 0.60, 0.31], [0.71, 0.93, 0.85]];

#[derive(Default)]
pub struct Scene {
    pub raster: Raster,
}
impl Scene {
    pub fn render(&mut self, width: usize, height: usize, time: f64) {
        let t = time.rem_euclid(PERIOD) as f32;
        let beat = Beat::at(time);
        let r = &mut self.raster;
        r.begin(width, height, t);
        r.ellipse((64., 54.5), 61., 4., [0.13, 0.13, 0.23], 1.);
        r.ring((64., 54.5), 60., 0.065, [0.28, 0.25, 0.40]);
        for (i, p) in beat.poses.iter().enumerate() {
            let height = (54. - p.y).max(0.);
            r.ellipse((p.x, 55.), (8. - height * 0.18).max(3.), 1.3, INK, 0.6);
            // Curved speed ribbons use previous positions, so the trail follows the arc.
            if p.lag.abs() > 1.2 {
                for n in 1..=3 {
                    let old = motion::poses(time - n as f64 * 0.10)[i];
                    r.curve(
                        [
                            (old.x, old.y - 12. + n as f32 * 2.),
                            (old.x + p.lag, old.y - 15.),
                            (p.x + p.lag, p.y - 15.),
                            (p.x, p.y - 12.),
                        ],
                        0.35,
                        COLORS[i].map(|v| v * (0.35 - n as f32 * 0.06)),
                    );
                }
            }
        }
        // Airborne characters pass behind, then in front as their paths descend.
        let mut order = [0, 1, 2];
        order.sort_by(|&a, &b| beat.poses[a].y.total_cmp(&beat.poses[b].y));
        for id in order {
            characters::draw(r, id, beat.poses[id], t);
        }
        effects(r, time, t);
        for (i, x) in [24., 64., 104.].into_iter().enumerate() {
            r.text(["DEEPSEEK", "CLAUDE", "CHATGPT"][i], x, 59., COLORS[i]);
        }
    }
}

fn progress(t: f32, start: f32, end: f32) -> Option<f32> {
    if t < start || t >= end {
        None
    } else {
        Some((t - start) / (end - start))
    }
}
fn arc(a: Point, b: Point, u: f32, lift: f32) -> Point {
    (
        a.0 + (b.0 - a.0) * u,
        a.1 + (b.1 - a.1) * u - (PI * u).sin() * lift,
    )
}
fn burst(r: &mut Raster, p: Point, u: f32, c: Rgb) {
    let fade = (PI * u).sin();
    for i in 0..10 {
        let a = i as f32 * TAU / 10.;
        let distance = 2. + u * 16.;
        r.star(
            (p.0 + a.cos() * distance, p.1 + a.sin() * distance * 0.6),
            fade * (1.4 + (i % 3) as f32 * 0.4),
            a + u * 4.,
            c,
        );
    }
    r.ring(p, 2. + u * 14., 0.65, c.map(|v| v * fade * 0.6));
}
fn heart(r: &mut Raster, p: Point, size: f32, c: Rgb) {
    let points = [
        (-1., -0.1),
        (-0.9, -0.7),
        (-0.35, -0.8),
        (0., -0.45),
        (0.35, -0.8),
        (0.9, -0.7),
        (1., -0.1),
        (0., 1.),
    ];
    let points = points.map(|(x, y)| (p.0 + x * size, p.1 + y * size));
    r.blob(&points, c);
}
fn effects(r: &mut Raster, time: f64, t: f32) {
    let poses = motion::poses(time);
    if let Some(u) = progress(t, 1.15, 1.72) {
        let p = arc((36., 34.), (71., 32.), u, 7.);
        let grow = (u * 10.).min(1.);
        r.ellipse(p, 6. * grow, 3.8 * grow, COLORS[0], 1.);
        r.ellipse((p.0 + 1., p.1 + 1.3), 4. * grow, 1.8 * grow, WHITE, 1.);
        r.blob(
            &[
                (p.0 - 4., p.1),
                (p.0 - 9., p.1 - 4.),
                (p.0 - 8., p.1 + 1.),
                (p.0 - 9., p.1 + 4.),
            ],
            COLORS[0],
        );
        r.ellipse((p.0 + 3., p.1 - 1.), 0.7, 0.8, INK, 1.);
        r.curve(
            [
                (p.0, p.1 - 3.),
                (p.0 - 1., p.1 - 8.),
                (p.0 + 3., p.1 - 8.),
                (p.0 + 4., p.1 - 6.),
            ],
            0.7,
            WHITE,
        );
    }
    if let Some(u) = progress(t, 1.65, 2.45) {
        burst(r, (71., 32.), u, COLORS[0]);
    }
    for j in 0..6 {
        let start = 3.65 + j as f32 * 0.08;
        if let Some(u) = progress(t, start, start + 0.65) {
            let p = arc((76., 31.), (110., 28.), u, 9. + j as f32 * 0.6);
            let angle = t * 5. + j as f32;
            let (s, c) = angle.sin_cos();
            let shape = [(-1.8, -2.3), (1.8, -2.3), (1.8, 2.3), (-1.8, 2.3)];
            let shape = shape.map(|(x, y)| (p.0 + x * c - y * s, p.1 + x * s + y * c));
            r.polygon(&shape, [1., 0.90, 0.72]);
            r.star(p, 1.1, angle, COLORS[1]);
        }
    }
    if let Some(u) = progress(t, 4.35, 5.1) {
        burst(r, (110., 28.), u, COLORS[1]);
    }
    if let Some(u) = progress(t, 7.1, 8.1) {
        let from = (poses[2].x - 9., poses[2].y - 21.);
        let end = arc(from, (18., 33.), (u * 1.5).min(1.), 3.);
        let width = (PI * u).sin() * 2.4;
        r.curve(
            [
                from,
                (from.0 - 8., from.1 - 4.),
                (end.0 + 5., end.1 + 4.),
                end,
            ],
            width,
            [0.36, 0.65, 0.66],
        );
        r.curve(
            [
                from,
                (from.0 - 8., from.1 - 4.),
                (end.0 + 5., end.1 + 4.),
                end,
            ],
            width * 0.55,
            WHITE,
        );
        for j in 0..4 {
            let a = t * 8. + j as f32 * TAU / 4.;
            r.star(
                (end.0 + a.cos() * 4., end.1 + a.sin() * 3.),
                width * 0.65,
                a,
                COLORS[2],
            );
        }
    }
    if let Some(u) = progress(t, 7.95, 8.8) {
        burst(r, (18., 32.), u, COLORS[2]);
    }
    // The chase leaves curved confetti trails instead of hiding the actors in a cloud.
    if let Some(u) = progress(t, 11.5, 17.7) {
        let fade = (PI * u).sin();
        for (i, p) in poses.iter().enumerate() {
            for j in 0..3 {
                let old = motion::poses(time - 0.12 * (j + 1) as f64)[i];
                r.star(
                    (old.x - p.lag, old.y - 7. + (t * 6. + j as f32).sin() * 2.),
                    fade * 1.2,
                    t * 4.,
                    COLORS[i],
                );
            }
        }
    }
    if let Some(u) = progress(t, 18.35, 19.7) {
        let grow = (PI * u).sin();
        for (i, p) in poses.iter().enumerate() {
            let from = (p.x, p.y - 16.);
            let center = (64., 34.);
            let bend = (i as f32 - 1.) * 12.;
            r.curve(
                [
                    from,
                    (from.0 + bend, from.1 - 8.),
                    (center.0 - bend, center.1 + 8.),
                    center,
                ],
                grow * 1.5,
                COLORS[i],
            );
            r.ring(center, 2. + grow * (5. + i as f32 * 2.), 0.8, COLORS[i]);
        }
        r.star((64., 34.), grow * 6., t * 4., WHITE);
    }
    if let Some(u) = progress(t, 19.65, 20.6) {
        for (i, c) in COLORS.iter().enumerate() {
            burst(r, (60. + i as f32 * 4., 33.), u, *c);
        }
    }
    if let Some(u) = progress(t, 20.0, 21.5) {
        for (i, p) in poses.iter().enumerate() {
            for j in 0..3 {
                let a = t * 5. + j as f32 * TAU / 3.;
                r.star(
                    (p.x + a.cos() * 8., p.y - 38. + a.sin() * 2.),
                    (PI * u).sin() * 1.4,
                    a,
                    COLORS[i],
                );
            }
        }
    }
    if let Some(u) = progress(t, 21.2, 23.5) {
        for (i, p) in poses.iter().enumerate() {
            let size = (PI * u).sin() * 2.;
            heart(
                r,
                (p.x + 12., p.y - 29. - u * 5.),
                size,
                [1., 0.60 + i as f32 * 0.05, 0.72],
            );
        }
    }
}
