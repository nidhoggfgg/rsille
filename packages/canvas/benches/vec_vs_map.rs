use std::collections::HashMap;

use canvas::utils::get_pos;
use criterion::{criterion_group, criterion_main, Criterion};
use rand::{rngs::StdRng, Rng, SeedableRng};

const WIDTH: i32 = 800;
const HEIGHT: i32 = 400;

fn bench_vec(points: &[(i32, i32)]) {
    let mut width = 1;
    let mut height = 1;
    let mut result: Vec<u8> = vec![0];
    for p in points {
        let (col, row) = get_pos(p.0, p.1);
        if col >= width {
            let new_width = col + 1;
            let (uw, un, uh) = (width as usize, new_width as usize, height as usize);
            let mut new_result = vec![0; un * uh];
            for (i, d) in result.into_iter().enumerate() {
                new_result[(i / uw as usize) * un + (i % uw as usize)] = d
            }
            result = new_result;

            width = col + 1;
        }
        if row >= height {
            let nums = width * (row - height + 1);
            result.append(&mut vec![0; nums as usize]);
            height = row + 1;
        }
        result[(row * width + col) as usize] = 1;
    }
}

fn bench_vec_fixed(points: &[(i32, i32)]) {
    let width = WIDTH;
    let height = HEIGHT;
    let mut result = vec![0_u8; (width * height) as usize];
    for p in points {
        let (col, row) = get_pos(p.0, p.1);
        result[(row * width + col) as usize] = 1;
    }
}

fn bench_map(points: &[(i32, i32)]) {
    let mut result = HashMap::new();
    for p in points {
        let p = get_pos(p.0, p.1);
        result.insert(p, 1);
    }
}

fn criterion_benchmark(bencher: &mut Criterion) {
    let num = 1000;
    let mut rng = StdRng::seed_from_u64(42);
    let mut points = Vec::with_capacity(num);
    for _ in 0..num {
        // 800 * 400 terminal size
        let x = rng.gen_range(0..WIDTH * 2);
        let y = rng.gen_range(0..HEIGHT * 4);
        points.push((x, y));
    }
    bencher.bench_function("vec", |b| b.iter(|| bench_vec(&points)));
    bencher.bench_function("vec_fixed_size", |b| b.iter(|| bench_vec_fixed(&points)));
    bencher.bench_function("map", |b| b.iter(|| bench_map(&points)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
