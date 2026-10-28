//! An open meadow: continuous ground, rolling terrain, wind-blown grass and trees.
use crate::{
    geometry::Model,
    math::{random, smooth, V3},
    raster::Rgb,
};
use std::f32::consts::{PI, TAU};

pub const HAZE: Rgb = [0.54, 0.68, 0.58];

fn hash(x: i32, z: i32) -> f32 {
    random(
        (x as u32)
            .wrapping_mul(1597334677)
            .wrapping_add((z as u32).wrapping_mul(3812015801)) as usize,
    )
}

fn noise(x: f32, z: f32) -> f32 {
    let ix = x.floor() as i32;
    let iz = z.floor() as i32;
    let u = smooth(x - x.floor());
    let v = smooth(z - z.floor());
    let a = hash(ix, iz) * (1.0 - u) + hash(ix + 1, iz) * u;
    let b = hash(ix, iz + 1) * (1.0 - u) + hash(ix + 1, iz + 1) * u;
    a * (1.0 - v) + b * v
}

pub fn height(x: f32, z: f32) -> f32 {
    // The clearing is flat for foot contact and lunging; hills grow beyond it.
    let outer = smooth(((x * x + z * z).sqrt() - 5.5) / 6.0);
    outer * (0.35 + noise(x * 0.12, z * 0.12) * 1.6 + noise(x * 0.31, z * 0.31) * 0.35)
}

pub fn grass_color(x: f32, z: f32) -> Rgb {
    let broad = noise(x * 0.32, z * 0.32);
    let patches = noise(x * 1.1 + 18.0, z * 1.1);
    let grain = (x * 19.7 + (z * 5.3).sin()).sin() * (z * 23.1 + x * 7.2).cos();
    [
        0.16 + broad * 0.095 + patches * 0.028 + grain * 0.009,
        0.29 + broad * 0.13 + patches * 0.035 + grain * 0.013,
        0.095 + broad * 0.045 + patches * 0.012,
    ]
}

pub fn fog(rgb: Rgb, distance: f32) -> Rgb {
    let amount = 1.0 - (-((distance - 13.0).max(0.0)) * 0.039).exp();
    std::array::from_fn(|i| rgb[i] * (1.0 - amount) + HAZE[i] * amount)
}

pub fn sky(ray: V3) -> Rgb {
    let ray = ray.unit();
    let horizon = (1.0 - ray.1.max(0.0) * 2.0).clamp(0.0, 1.0);
    let sun = ray.dot(V3(-0.48, 0.48, -0.73).unit()).max(0.0).powi(48);
    let clouds = smooth((noise(ray.0 * 6.0 + 10.0, ray.2 * 6.0 + ray.1 * 9.0) - 0.45) * 5.0) * 0.10;
    std::array::from_fn(|i| {
        ([0.23, 0.43, 0.53][i] * (1.0 - horizon)
            + HAZE[i] * horizon
            + sun * [0.23, 0.18, 0.08][i]
            + clouds)
            .min(1.0)
    })
}

fn foliage(m: &mut Model<'_>, center: V3, size: V3, color: Rgb) {
    let vertex = |i: usize, j: usize| {
        let u = i as f32 * TAU / 10.0;
        let v = j as f32 * PI / 6.0;
        center
            + V3(
                size.0 * v.sin() * u.cos(),
                size.1 * v.cos(),
                size.2 * v.sin() * u.sin(),
            )
    };
    for j in 0..6 {
        for i in 0..10 {
            m.quad(
                [
                    vertex(i, j),
                    vertex(i + 1, j),
                    vertex(i + 1, j + 1),
                    vertex(i, j + 1),
                ],
                color,
                false,
            );
        }
    }
}

pub fn draw(m: &mut Model<'_>, time: f32) {
    // The analytic ground in Raster covers the entire view, even beyond this mesh.
    for iz in -20..20 {
        for ix in -20..20 {
            let x = ix as f32 * 2.0;
            let z = iz as f32 * 2.0;
            let points = [(x, z), (x + 2.0, z), (x + 2.0, z + 2.0), (x, z + 2.0)]
                .map(|(x, z)| V3(x, height(x, z), z));
            if points.iter().all(|p| p.1 < 0.001) {
                continue;
            }
            m.raster.terrain([points[0], points[1], points[2]]);
            m.raster.terrain([points[0], points[2], points[3]]);
        }
    }
    // Uneven clusters, with short blades in the fight space and taller foreground grass.
    for i in 0..1250 {
        let x = (random(i * 7) - 0.5) * 29.0;
        let z = (random(i * 7 + 1) - 0.5) * 29.0;
        let base = V3(x, height(x, z) + 0.008, z);
        let outside = smooth(((x * x + z * z).sqrt() - 3.6) / 3.4);
        let h = 0.06 + random(i * 7 + 2) * 0.12 + outside * 0.23;
        let wind = (time * 1.65 + x * 0.48 + z * 0.31).sin() * h * 0.21;
        let green = 0.33 + random(i * 7 + 3) * 0.20;
        for blade in 0..3 {
            let a = random(i * 7 + 4) * TAU + blade as f32 * 2.1;
            let width = 0.024 + outside * 0.027;
            let side = V3(a.cos() * width, 0.0, a.sin() * width);
            let tip = base
                + V3(
                    wind + a.sin() * h * 0.21,
                    h * (0.78 + blade as f32 * 0.11),
                    a.cos() * h * 0.20,
                );
            m.tri(
                [base - side, base + side, tip],
                [
                    0.19 + random(i * 7 + 5) * 0.08,
                    green,
                    0.075 + random(i * 7 + 6) * 0.06,
                ],
                false,
            );
        }
    }
    // A tree line surrounds the clearing, so free camera orbits still see a landscape.
    for i in 0..31 {
        let a = i as f32 * TAU / 31.0 + random(i + 400) * 0.13;
        let radius = 12.0 + random(i + 450) * 12.0;
        let p = V3(
            a.cos() * radius,
            height(a.cos() * radius, a.sin() * radius),
            a.sin() * radius,
        );
        let h = 2.4 + random(i + 500) * 2.3;
        m.tube(
            p,
            p + V3(0.06, h * 0.78, 0.0),
            0.19,
            0.10,
            [0.23, 0.19, 0.12],
            false,
        );
        let hue = random(i + 550);
        for layer in 0..3 {
            let angle = layer as f32 * 2.3 + i as f32;
            let offset = V3(
                angle.sin() * 0.48,
                h * (0.64 + layer as f32 * 0.12),
                angle.cos() * 0.40,
            );
            foliage(
                m,
                p + offset,
                V3(1.1 + h * 0.10, 0.9 + h * 0.12, 1.05 + h * 0.09),
                [0.11 + hue * 0.07, 0.27 + hue * 0.12, 0.12 + hue * 0.04],
            );
        }
    }
    // Small stones and meadow flowers replace the stadium's artificial perimeter.
    for i in 0..22 {
        let a = random(i + 650) * TAU;
        let r = 5.8 + random(i + 700) * 4.0;
        let x = a.cos() * r;
        let z = a.sin() * r;
        let size = 0.09 + random(i + 750) * 0.17;
        foliage(
            m,
            V3(x, height(x, z) + size * 0.3, z),
            V3(size * 1.5, size * 0.65, size),
            [0.32, 0.36, 0.28],
        );
    }
    for i in 0..70 {
        let x = (random(i + 800) - 0.5) * 22.0;
        let z = (random(i + 900) - 0.5) * 22.0;
        if x * x + z * z < 16.0 {
            continue;
        }
        let y = height(x, z);
        let p = V3(x, y + 0.18, z);
        m.line(V3(x, y, z), p, [0.22, 0.38, 0.12]);
        foliage(
            m,
            p,
            V3(0.045, 0.025, 0.045),
            if i % 3 == 0 {
                [0.95, 0.78, 0.31]
            } else {
                [0.83, 0.86, 0.71]
            },
        );
    }
}
