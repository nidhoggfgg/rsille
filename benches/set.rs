use criterion::{criterion_group, criterion_main, Criterion};
use rsille::Canvas;

fn render_10000(size: i32) {
    let mut c = Canvas::new();
    let size_f = size as f64;
    for _ in 0..10000 {
        c.set(0.0, 0.0);
        for i in 0..size {
            c.set(size_f, i as f64);
        }
        c.draw();
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("0x0", |b| b.iter(|| render_10000(0)));
    c.bench_function("10x10", |b| b.iter(|| render_10000(10)));
    c.bench_function("20x20", |b| b.iter(|| render_10000(20)));
    c.bench_function("40x40", |b| b.iter(|| render_10000(40)));
    c.bench_function("100x100", |b| b.iter(|| render_10000(100)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
