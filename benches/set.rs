use criterion::{criterion_group, criterion_main, Criterion};
use rsille::Canvas;

fn render(size: i32, c: &mut Canvas) {
    let mut v = vec![];
    c.set(0.0, 0.0);
    for i in 0..size {
        for j in 0..size {
            c.set(i as f64, j as f64);
        }
    }
    c.print_on(&mut v, false).unwrap();
}

fn criterion_benchmark(bencher: &mut Criterion) {
    let mut c = Canvas::new();
    bencher.bench_function("0 * 0", |b| b.iter(|| render(0, &mut c)));
    c.reset();
    bencher.bench_function("10 * 10", |b| b.iter(|| render(10, &mut c)));
    c.reset();
    bencher.bench_function("20 * 20", |b| b.iter(|| render(20, &mut c)));
    c.reset();
    bencher.bench_function("40 * 40", |b| b.iter(|| render(40, &mut c)));
    c.reset();
    bencher.bench_function("100 * 100", |b| b.iter(|| render(100, &mut c)));
    c.reset();
    bencher.bench_function("500 * 500 huge", |b| b.iter(|| render(500, &mut c)));
    c.reset();
    bencher.bench_function("1000 * 1000 crazy", |b| b.iter(|| render(1000, &mut c)));
    c.reset();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
