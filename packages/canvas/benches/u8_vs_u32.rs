use criterion::{criterion_group, criterion_main, Criterion};
use rand::{rngs::StdRng, Rng, SeedableRng};

const BASE_CHAR: u32 = 0x2800;

fn bench_u8(pixels: &[u8]) -> Vec<char> {
    let mut result = vec![char::REPLACEMENT_CHARACTER; pixels.len()];
    for (i, pixel) in pixels.iter().enumerate() {
        result[i] = unsafe { char::from_u32_unchecked(BASE_CHAR + *pixel as u32) }
    }
    return result;
}

fn bench_u32(pixels: &[u32]) -> Vec<char> {
    let mut result = vec![char::REPLACEMENT_CHARACTER; pixels.len()];
    for (i, pixel) in pixels.iter().enumerate() {
        result[i] = unsafe { char::from_u32_unchecked(BASE_CHAR + pixel) }
    }
    return result;
}

fn criterion_benchmark(bencher: &mut Criterion) {
    let num = 100000;
    let mut rng = StdRng::seed_from_u64(42);
    let mut u8s = Vec::new();
    let mut u32s = Vec::new();
    for _ in 0..num {
        let v = rng.gen_range(0..=u8::MAX);
        u8s.push(v as u8);
        u32s.push(v as u32);
    }
    bencher.bench_function("u8", |b| b.iter(|| bench_u8(&u8s)));
    bencher.bench_function("u32", |b| b.iter(|| bench_u32(&u32s)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
