//! Three individually costumed chibi rigs, drawn from curved silhouettes.
use crate::{
    motion::Pose,
    raster::{Point, Raster, Rgb, INK, WHITE},
};
use std::f32::consts::TAU;
const SKIN: Rgb = [1.0, 0.86, 0.78];
const NAVY: Rgb = [0.10, 0.16, 0.35];
const CREAM: Rgb = [1.0, 0.91, 0.76];
const LILAC: Rgb = [0.65, 0.58, 0.80];

struct Doll<'a> {
    r: &'a mut Raster,
    p: Pose,
    head: bool,
}
impl Doll<'_> {
    fn point(&self, mut x: f32, mut y: f32) -> Point {
        if self.head {
            let (s, c) = self.p.head.sin_cos();
            let v = y + 25.;
            (x, y) = (x * c - v * s, -25. + x * s + v * c);
        }
        x *= 1. + self.p.squash;
        y = (y + 10.) * (1. - self.p.squash) - 10.;
        let (s, c) = self.p.lean.sin_cos();
        (
            self.p.x + x * c - (y + 10.) * s,
            self.p.y - 10. + x * s + (y + 10.) * c,
        )
    }
    fn blob(&mut self, p: &[Point], c: Rgb) {
        let p: Vec<_> = p.iter().map(|&(x, y)| self.point(x, y)).collect();
        self.r.blob(&p, c);
    }
    fn poly(&mut self, p: &[Point], c: Rgb) {
        let p: Vec<_> = p.iter().map(|&(x, y)| self.point(x, y)).collect();
        self.r.polygon(&p, c);
    }
    fn ellipse(&mut self, x: f32, y: f32, rx: f32, ry: f32, c: Rgb) {
        let p: Vec<_> = (0..28)
            .map(|i| {
                let a = i as f32 * TAU / 28.;
                self.point(x + rx * a.cos(), y + ry * a.sin())
            })
            .collect();
        self.r.polygon(&p, c);
    }
    fn line(&mut self, a: Point, b: Point, w: f32, c: Rgb) {
        self.r
            .line(self.point(a.0, a.1), self.point(b.0, b.1), w, c);
    }
    fn curve(&mut self, p: [Point; 4], w: f32, c: Rgb) {
        let p = p.map(|(x, y)| self.point(x, y));
        self.r.curve(p, w, c);
    }
    fn knot(&mut self, x: f32, y: f32, size: f32, c: Rgb) {
        for i in 0..6 {
            let a = i as f32 * TAU / 6.;
            self.ellipse(
                x + a.cos() * size * 0.5,
                y + a.sin() * size * 0.5,
                size * 0.43,
                size * 0.43,
                c,
            );
        }
        self.ellipse(x, y, size * 0.35, size * 0.35, WHITE);
    }
}

pub fn draw(r: &mut Raster, id: usize, p: Pose, t: f32) {
    let mut d = Doll { r, p, head: false };
    let hair = [[0.18, 0.29, 0.58], [0.82, 0.32, 0.13], [0.78, 0.74, 0.89]][id];
    let shine = [[0.36, 0.54, 0.86], [1., 0.62, 0.31], [0.99, 0.97, 1.]][id];
    let flow = p.lag + (t * TAU / 3. + id as f32).sin() * 1.1;
    // The whale's broad flukes and the dragon's jointed wings are not shared parts.
    if id == 0 {
        // Curl the fluke inward near the edge instead of clipping it on recoil.
        let tail = -15. + flow * 0.4 + (26. - p.x).max(0.) * 0.8;
        d.curve(
            [(-5., -6.), (-13., -1.), (tail - 3., -7.), (tail, -14.)],
            3.0,
            NAVY,
        );
        d.blob(
            &[
                (tail, -12.),
                (tail - 7., -16.),
                (tail - 9., -22.),
                (tail - 3., -20.),
                (tail, -17.),
                (tail + 5., -21.),
                (tail + 7., -24.),
                (tail + 6., -16.),
            ],
            hair,
        );
        d.curve(
            [
                (tail - 7., -20.),
                (tail - 5., -16.),
                (tail - 2., -15.),
                (tail, -14.),
            ],
            0.6,
            shine,
        );
    } else if id == 2 {
        let flap = (t * TAU / 1.5).sin() * 3. + p.attack * 2.;
        for s in [-1., 1.] {
            let wing = [
                (s * 5., -19.),
                (s * 12., -26. - flap),
                (s * 22., -28. - flap),
                (s * 19., -18.),
                (s * 18., -11.),
                (s * 13., -15.),
                (s * 8., -12.),
            ];
            d.blob(&wing, INK);
            let inner = [
                (s * 6., -19.),
                (s * 13., -25. - flap),
                (s * 20., -26. - flap),
                (s * 17., -19.),
                (s * 17., -13.),
                (s * 12., -17.),
                (s * 8., -14.),
            ];
            d.blob(&inner, LILAC);
            d.curve(
                [
                    (s * 6., -19.),
                    (s * 10., -23.),
                    (s * 16., -28. - flap),
                    (s * 21., -27. - flap),
                ],
                0.8,
                WHITE,
            );
            d.line(
                (s * 12., -23. - flap * 0.5),
                (s * 17., -13.),
                0.5,
                [0.87, 0.82, 0.96],
            );
        }
        d.curve(
            [(5., -5.), (20., 0.), (24. + flow, -9.), (17. + flow, -15.)],
            2.4,
            LILAC,
        );
        d.curve(
            [(5., -5.), (20., 0.), (24. + flow, -9.), (17. + flow, -15.)],
            1.5,
            WHITE,
        );
        for i in 0..4 {
            d.ellipse(15. + i as f32 * 1.6, -3. - i as f32 * 0.7, 0.6, 0.5, LILAC);
        }
    }
    // Each hair mass has a different length and curl rhythm; roots follow the head,
    // while the tips lag the torso instead of translating as a rigid cutout.
    for s in [-1., 1.] {
        let tip = if id == 0 { -4. } else { -1.5 };
        d.blob(
            &[
                (s * 7., -30.),
                (s * 12., -24.),
                (s * 11. + flow, -14.),
                (s * 15. + flow, tip - 3.),
                (s * 10. + flow, tip),
                (s * 7. + flow, -7.),
                (s * 6., -21.),
            ],
            hair,
        );
        d.curve(
            [
                (s * 10., -25.),
                (s * 7., -16.),
                (s * 16. + flow, -7.),
                (s * 11. + flow, tip - 1.),
            ],
            0.9,
            shine,
        );
        if id == 1 {
            d.curve(
                [
                    (s * 9., -18.),
                    (s * 18. + flow, -7.),
                    (s * 17. + flow, 1.),
                    (s * 10. + flow, -3.),
                ],
                1.1,
                hair,
            );
        }
    }
    // Alternating knees and little shoes make even airborne poses feel alive.
    let skip = (t * TAU / 1.5).sin() * p.attack * 2.4;
    for s in [-1., 1.] {
        let foot = s * 3.4 + s * skip;
        d.line((s * 3., -7.), (foot, -1.6 - skip.abs() * 0.25), 1.4, WHITE);
        d.ellipse(
            foot,
            -0.8 - skip.abs() * 0.25,
            2.3,
            1.2,
            if id == 2 { LILAC } else { NAVY },
        );
    }
    let hem = flow * 0.35;
    if id == 0 {
        // Navy maid dress, white scalloped apron and dark bodice: never a white tube.
        d.blob(
            &[
                (-4., -17.),
                (4., -17.),
                (6., -10.),
                (10. + hem, -4.),
                (5., -2.5),
                (-6., -2.5),
                (-10. + hem, -4.),
                (-6., -10.),
            ],
            NAVY,
        );
        d.blob(
            &[
                (-4., -11.),
                (4., -11.),
                (6. + hem, -5.),
                (3., -3.),
                (-3., -3.),
                (-6. + hem, -5.),
            ],
            WHITE,
        );
        for x in -3..=3 {
            d.ellipse(x as f32 * 1.6 + hem * 0.3, -3.6, 1.0, 0.7, CREAM);
        }
        d.poly(
            &[(-3., -18.), (-1.5, -17.), (-2., -12.), (-3.5, -12.)],
            WHITE,
        );
        d.poly(&[(3., -18.), (1.5, -17.), (2., -12.), (3.5, -12.)], WHITE);
        d.ellipse(0., -6., 1.9, 0.9, hair);
        d.poly(&[(1., -6.), (3., -7.5), (3., -5.5)], hair);
        d.ellipse(0., -13., 0.45, 0.45, WHITE);
    } else if id == 1 {
        // Orange underskirt, black corset, cream open robe, broad cuffs and black hem.
        d.blob(
            &[
                (-4., -16.),
                (4., -16.),
                (10. + hem, -4.),
                (4., -2.),
                (-6., -2.),
                (-10. + hem, -4.),
            ],
            [0.16, 0.12, 0.17],
        );
        d.poly(
            &[(-2., -14.), (2., -14.), (6., -3.), (-5., -3.)],
            [0.95, 0.46, 0.20],
        );
        for s in [-1., 1.] {
            d.blob(
                &[
                    (s * 4., -18.),
                    (s * 7., -12.),
                    (s * 10. + hem, -4.),
                    (s * 6., -3.),
                    (s * 3., -11.),
                ],
                CREAM,
            );
            d.line((s * 6., -8.), (s * 8., -4.), 0.45, hair);
        }
        d.poly(
            &[(-4., -17.), (4., -17.), (3.6, -12.), (-3.6, -12.)],
            [0.18, 0.12, 0.15],
        );
        for y in [-16., -14.] {
            d.ellipse(0., y, 0.4, 0.4, [1., 0.72, 0.36]);
        }
    } else {
        d.blob(
            &[
                (-4., -18.),
                (4., -18.),
                (5., -11.),
                (10. + hem, -3.),
                (4., -2.),
                (-4., -2.),
                (-10. + hem, -3.),
                (-5., -11.),
            ],
            LILAC,
        );
        d.blob(
            &[
                (-3.7, -18.),
                (3.7, -18.),
                (4., -10.),
                (8. + hem, -4.),
                (0., -5.),
                (-8. + hem, -4.),
                (-4., -10.),
            ],
            WHITE,
        );
        d.blob(
            &[
                (-4., -10.),
                (0., -7.),
                (4., -10.),
                (6., -3.),
                (0., -2.),
                (-6., -3.),
            ],
            [0.90, 0.87, 0.97],
        );
        d.knot(0., -12., 1.5, [0.34, 0.47, 0.43]);
        d.knot(-5., -5., 1.1, [0.43, 0.50, 0.48]);
    }
    // Jointed arms: shoulders, elbows, hands and held book all follow the same rig.
    for (side, angle) in [-1., 1.].into_iter().zip(p.arms) {
        let elbow = (side * (4.5 + angle.sin() * 5.), -17. + angle.cos() * 4.);
        let hand = (
            elbow.0 + side * (angle * 0.7 + 0.3).sin() * 3.,
            elbow.1 + (angle * 0.7 + 0.3).cos() * 3.,
        );
        let sleeve = if id == 0 {
            NAVY
        } else if id == 1 {
            CREAM
        } else {
            WHITE
        };
        d.line((side * 4., -17.), elbow, 2.1, sleeve);
        d.line(elbow, hand, 1.7, if id == 0 { WHITE } else { sleeve });
        d.ellipse(
            side * 4.5,
            -16.5,
            2.5,
            2.2,
            if id == 0 { WHITE } else { sleeve },
        );
        d.ellipse(hand.0, hand.1, 1.6, 1.6, SKIN);
        if id == 1 && side > 0. {
            let (x, y) = hand;
            d.poly(
                &[
                    (x - 2.3, y - 4.),
                    (x + 4., y - 3.),
                    (x + 4., y + 4.),
                    (x - 2.3, y + 3.),
                ],
                [0.27, 0.11, 0.08],
            );
            d.line((x + 3.4, y - 2.5), (x + 3.4, y + 3.), 0.7, CREAM);
            for i in 0..8 {
                let a = i as f32 * TAU / 8.;
                d.line(
                    (x + 0.5, y),
                    (x + 0.5 + a.cos() * 1.8, y + a.sin() * 1.8),
                    0.35,
                    [1., 0.68, 0.31],
                );
            }
        }
    }
    // Neck ribbon differs from hair; the dark bow is an important maid/Claude cue.
    if id != 2 {
        d.poly(&[(-0.4, -18.), (-3., -19.5), (-3., -16.5)], INK);
        d.poly(&[(0.4, -18.), (3., -19.5), (3., -16.5)], INK);
        d.ellipse(0., -18., 0.7, 0.7, if id == 0 { shine } else { hair });
    }
    d.head = true;
    // Bigger, rounder head, smaller chin and eyes that follow the other characters.
    d.ellipse(0., -26., 11., 10.1, INK);
    d.ellipse(0., -26., 10.5, 9.6, hair);
    d.blob(
        &[
            (-8.7, -29.),
            (-8.9, -23.),
            (-5., -18.5),
            (0., -17.7),
            (5., -18.5),
            (8.9, -23.),
            (8.7, -29.),
            (0., -33.),
        ],
        SKIN,
    );
    d.ellipse(-6., -22.5, 2.1, 1.05, [1., 0.63, 0.64]);
    d.ellipse(6., -22.5, 2.1, 1.05, [1., 0.63, 0.64]);
    let blink = (-((t + id as f32 * 0.7).rem_euclid(4.) - 3.6).powi(2) / 0.006).exp();
    let shut = blink.max(p.hit * 0.95);
    let iris = [[0.20, 0.43, 0.75], [0.54, 0.29, 0.15], [0.57, 0.36, 0.72]][id];
    for side in [-1., 1.] {
        let x = side * 3.8;
        if p.joy > 0.8 && side < 0. || shut > 0.85 {
            d.curve(
                [
                    (x - 1.8, -24.5),
                    (x - 0.5, -26.),
                    (x + 0.5, -26.),
                    (x + 1.8, -24.5),
                ],
                0.65,
                INK,
            );
        } else {
            let height = 2.6 * (1. - shut * 0.8);
            d.ellipse(x, -24.9, 2.25, height + 0.3, WHITE);
            d.ellipse(x + p.gaze * 0.5, -24.8, 1.55, height, iris);
            d.ellipse(x + p.gaze * 0.6, -24.9, 0.72, height * 0.9, INK);
            d.ellipse(x - 0.5, -25.8, 0.75, 0.8, WHITE);
            d.ellipse(x + 0.6, -23.8, 0.35, 0.35, WHITE);
            d.curve(
                [
                    (x - 2., -26.6),
                    (x - 1., -27.5),
                    (x + 1., -27.4),
                    (x + 2., -26.5),
                ],
                0.65,
                INK,
            );
            d.line((x + side * 1.8, -26.5), (x + side * 2.6, -27.2), 0.5, INK);
        }
    }
    if p.hit > 0.35 {
        d.ellipse(0., -20.8, 1.25, 1.3, [0.51, 0.20, 0.28]);
    } else if p.joy > 0.55 {
        d.blob(
            &[
                (-1.7, -21.1),
                (0., -20.7),
                (1.7, -21.1),
                (0.9, -19.6),
                (-0.9, -19.6),
            ],
            [0.55, 0.20, 0.29],
        );
        d.ellipse(0., -19.9, 0.8, 0.4, [1., 0.55, 0.61]);
    } else {
        d.curve(
            [(-1.4, -20.8), (-0.5, -19.6), (0.6, -19.6), (1.4, -20.8)],
            0.45,
            [0.55, 0.24, 0.29],
        );
    }
    // Curved, asymmetric fringes avoid three identical helmets.
    if id == 0 {
        d.blob(
            &[
                (-10., -27.),
                (-8., -33.),
                (-2., -35.5),
                (6., -33.),
                (9.5, -27.),
                (5., -28.5),
                (3., -31.),
                (0., -27.),
                (-2., -31.5),
                (-5., -27.),
                (-6., -30.),
            ],
            hair,
        );
    } else if id == 1 {
        d.blob(
            &[
                (-10., -26.),
                (-8., -33.),
                (-2., -35.5),
                (5., -33.5),
                (9., -26.5),
                (5., -28.),
                (2., -31.5),
                (-1., -30.),
                (-5., -25.5),
                (-6., -30.),
            ],
            hair,
        );
    } else {
        d.blob(
            &[
                (-10., -27.),
                (-7., -33.),
                (0., -35.5),
                (8., -32.),
                (10., -27.),
                (6., -28.5),
                (5., -31.5),
                (1., -27.5),
                (-1., -31.),
                (-4., -27.),
                (-6., -30.),
            ],
            WHITE,
        );
    }
    d.curve(
        [(-7., -31.), (-4., -34.), (-1., -34.), (2., -32.5)],
        0.85,
        shine,
    );
    if id == 0 {
        d.curve(
            [(-8., -33.), (-4., -37.), (4., -37.), (8., -33.)],
            1.4,
            WHITE,
        );
        for i in -3..=3 {
            d.ellipse(
                i as f32 * 2.5,
                -34.5 - (1. - (i as f32 / 3.5).powi(2)) * 1.7,
                1.6,
                1.3,
                WHITE,
            );
        }
        d.curve(
            [(-2., -35.), (-7., -41.), (-11., -38.), (-9., -34.)],
            0.7,
            hair,
        );
        for s in [-1., 1.] {
            d.blob(
                &[
                    (s * 8., -28.),
                    (s * 14., -25.),
                    (s * 11., -30.),
                    (s * 8., -31.),
                ],
                shine,
            );
        }
        d.ellipse(8., -32.5, 1.4, 1.5, shine);
    } else if id == 1 {
        d.poly(
            &[
                (7., -30.),
                (9., -29.),
                (11. + flow * 0.1, -22.),
                (8., -24.),
                (7., -21.),
            ],
            INK,
        );
        for i in 0..12 {
            let a = i as f32 * TAU / 12.;
            d.line(
                (7., -31.),
                (7. + a.cos() * 3.2, -31. + a.sin() * 3.2),
                0.65,
                [0.99, 0.64, 0.29],
            );
        }
        d.ellipse(7., -31., 1.1, 1.1, [0.50, 0.24, 0.14]);
    } else {
        for s in [-1., 1.] {
            d.curve(
                [
                    (s * 7., -33.),
                    (s * 10., -35.),
                    (s * 8., -37.),
                    (s * 7., -40.),
                ],
                1.65,
                INK,
            );
            d.curve(
                [
                    (s * 7., -33.),
                    (s * 10., -35.),
                    (s * 8., -37.),
                    (s * 7., -40.),
                ],
                1.05,
                WHITE,
            );
            d.line((s * 7.8, -35.), (s * 9.2, -35.7), 0.35, LILAC);
            d.line((s * 7.3, -37.), (s * 8.5, -37.6), 0.35, LILAC);
        }
        d.knot(8., -29.5, 2.1, [0.30, 0.46, 0.40]);
    }
}
