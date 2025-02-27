use std::cmp;

use canvas::Canvas;
use criterion::{criterion_group, criterion_main, Criterion};

fn bresenham(x0: i32, y0: i32, x1: i32, y1: i32) -> Canvas {
    let mut result = Canvas::new();
    let mut steep = false;
    let (mut x0, mut y0, mut x1, mut y1) = (x0, y0, x1, y1);
    if (x0 - x1).unsigned_abs() < (y0 - y1).unsigned_abs() {
        (x0, y0) = (y0, x0);
        (x1, y1) = (y1, x1);
        steep = true;
    }
    if x0 > x1 {
        (x0, x1) = (x1, x0);
        (y0, y1) = (y1, y0);
    }
    let dx = x1 - x0;
    let dy = y1 - y0;
    let derror2 = dy.unsigned_abs() as i32 * 2;
    let mut error2 = 0_i32;
    let mut y = y0;
    if steep {
        for x in x0..=x1 {
            result.set(y, x);
            error2 += derror2;
            if error2 > dx {
                y += if y1 > y0 { 1 } else { -1 };
                error2 -= dx * 2;
            }
        }
    } else {
        for x in x0..=x1 {
            result.set(x, y);
            error2 += derror2;
            if error2 > dx {
                y += if y1 > y0 { 1 } else { -1 };
                error2 -= dx * 2;
            }
        }
    }
    return result;
}

fn normal(x1: i32, y1: i32, x2: i32, y2: i32) -> Canvas {
    let mut result = Canvas::new();
    let d = |v1, v2| {
        if v1 <= v2 {
            (v2 - v1, 1.0)
        } else {
            (v1 - v2, -1.0)
        }
    };

    let (xdiff, xdir) = d(x1, x2);
    let (ydiff, ydif) = d(y1, y2);
    let r = cmp::max(xdiff, ydiff);

    for i in 0..=r {
        let r = r as f64;
        let i = i as f64;
        let (xd, yd) = (xdiff as f64, ydiff as f64);
        let x = x1 as f64 + i * xd / r * xdir;
        let y = y1 as f64 + i * yd / r * ydif;
        result.set(x, y);
    }

    result
}

fn criterion_benchmark(bencher: &mut Criterion) {
    let (x1, y1, x2, y2) = (-1, -2, 15, 42);
    bencher.bench_function("bresnham", |b| {
        b.iter(|| {
            for _ in 0..100 {
                bresenham(x1, y1, x2, y2);
            }
        })
    });
    bencher.bench_function("normal", |b| {
        b.iter(|| {
            for _ in 0..100 {
                normal(x1, y1, x2, y2);
            }
        })
    });
    let a = bresenham(x1, y1, x2, y2);
    let b = normal(x1, y1, x2, y2);
    assert_eq!(a, b);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
