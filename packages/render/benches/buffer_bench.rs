use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use render::{
    area::{Position, Size},
    buffer::{Buffer, LineState},
    style::Stylized,
};

/// Fill a buffer with test content
fn fill_buffer(buffer: &mut Buffer, pattern: char, count: u16) {
    for i in 0..count {
        let pos = Position {
            x: i % buffer.size().width,
            y: i / buffer.size().width,
        };
        buffer.set(pos, Stylized::plain(pattern)).ok();
    }
}

/// Fill a buffer with wide characters
fn fill_buffer_wide(buffer: &mut Buffer, pattern: char, count: u16) {
    let mut x = 0;
    let mut y = 0;
    let width = buffer.size().width;

    for _ in 0..count {
        let pos = Position { x, y };
        if let Ok(char_width) = buffer.set(pos, Stylized::plain(pattern)) {
            x += char_width as u16;
            if x >= width {
                x = 0;
                y += 1;
            }
        }
    }
}

/// Benchmark basic buffer operations
fn bench_buffer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_operations");

    // Test different buffer sizes
    for size in [
        (10, 10),   // Small: 100 cells
        (40, 25),   // Medium: 1000 cells (typical terminal)
        (100, 50),  // Large: 5000 cells
        (200, 100), // XLarge: 20000 cells
    ] {
        let buffer_size = Size {
            width: size.0,
            height: size.1,
        };

        group.throughput(Throughput::Elements((size.0 * size.1) as u64));

        // Benchmark: set
        group.bench_with_input(
            BenchmarkId::new("set", format!("{}x{}", size.0, size.1)),
            &buffer_size,
            |b, &size| {
                let mut buffer = Buffer::new(size);
                b.iter(|| {
                    fill_buffer(&mut buffer, black_box('x'), size.width * size.height);
                });
            },
        );

        // Benchmark: overwrite
        group.bench_with_input(
            BenchmarkId::new("overwrite", format!("{}x{}", size.0, size.1)),
            &buffer_size,
            |b, &size| {
                let mut buffer = Buffer::new(size);
                fill_buffer(&mut buffer, 'a', size.width * size.height);
                b.iter(|| {
                    fill_buffer(&mut buffer, black_box('x'), size.width * size.height);
                });
            },
        );

        // Benchmark: clear
        group.bench_with_input(
            BenchmarkId::new("clear", format!("{}x{}", size.0, size.1)),
            &buffer_size,
            |b, &size| {
                let mut buffer = Buffer::new(size);
                fill_buffer(&mut buffer, 'a', size.width * size.height);
                b.iter(|| {
                    buffer.clear();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark cell-level diff
fn bench_cell_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("cell_diff");

    let size = Size {
        width: 80,
        height: 24,
    };
    group.throughput(Throughput::Elements((size.width * size.height) as u64));

    // Benchmark: 0% change (best case)
    group.bench_function("no_change", |b| {
        let mut buffer = Buffer::new(size);
        fill_buffer(&mut buffer, 'a', size.width * size.height);
        buffer.clear();
        fill_buffer(&mut buffer, 'a', size.width * size.height);

        b.iter(|| {
            let diff: Vec<_> = buffer.diff().unwrap().collect();
            black_box(diff);
        });
    });

    // Benchmark: 10% change
    group.bench_function("10%_change", |b| {
        let mut buffer = Buffer::new(size);
        fill_buffer(&mut buffer, 'a', size.width * size.height);
        buffer.clear();
        fill_buffer(&mut buffer, 'a', size.width * size.height);
        // Change 10%
        let change_count = (size.width * size.height) / 10;
        fill_buffer(&mut buffer, 'x', change_count);

        b.iter(|| {
            let diff: Vec<_> = buffer.diff().unwrap().collect();
            black_box(diff);
        });
    });

    // Benchmark: 50% change
    group.bench_function("50%_change", |b| {
        let mut buffer = Buffer::new(size);
        fill_buffer(&mut buffer, 'a', size.width * size.height);
        buffer.clear();
        fill_buffer(&mut buffer, 'a', size.width * size.height);
        // Change 50%
        let change_count = (size.width * size.height) / 2;
        fill_buffer(&mut buffer, 'x', change_count);

        b.iter(|| {
            let diff: Vec<_> = buffer.diff().unwrap().collect();
            black_box(diff);
        });
    });

    // Benchmark: 100% change (worst case)
    group.bench_function("100%_change", |b| {
        let mut buffer = Buffer::new(size);
        fill_buffer(&mut buffer, 'a', size.width * size.height);
        buffer.clear();
        fill_buffer(&mut buffer, 'x', size.width * size.height);

        b.iter(|| {
            let diff: Vec<_> = buffer.diff().unwrap().collect();
            black_box(diff);
        });
    });

    group.finish();
}

/// Benchmark line-level diff
fn bench_line_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("line_diff");

    let size = Size {
        width: 80,
        height: 24,
    };
    group.throughput(Throughput::Elements(size.height as u64));

    // Benchmark: 0% change (best case)
    group.bench_function("no_change", |b| {
        let mut buffer = Buffer::new(size);
        fill_buffer(&mut buffer, 'a', size.width * size.height);
        buffer.clear();
        fill_buffer(&mut buffer, 'a', size.width * size.height);

        b.iter(|| {
            let diff: Vec<_> = buffer.diff_lines().collect();
            black_box(diff);
        });
    });

    // Benchmark: 1 line change (typical spinner/progress bar)
    group.bench_function("1_line_change", |b| {
        let mut buffer = Buffer::new(size);
        fill_buffer(&mut buffer, 'a', size.width * size.height);
        buffer.clear();
        fill_buffer(&mut buffer, 'a', size.width * size.height);
        // Change first line
        fill_buffer(&mut buffer, 'x', size.width);

        b.iter(|| {
            let diff: Vec<_> = buffer.diff_lines().collect();
            black_box(diff);
        });
    });

    // Benchmark: 25% lines change
    group.bench_function("25%_lines_change", |b| {
        let mut buffer = Buffer::new(size);
        fill_buffer(&mut buffer, 'a', size.width * size.height);
        buffer.clear();
        fill_buffer(&mut buffer, 'a', size.width * size.height);
        // Change 25% of lines
        let change_count = (size.width * size.height) / 4;
        fill_buffer(&mut buffer, 'x', change_count);

        b.iter(|| {
            let diff: Vec<_> = buffer.diff_lines().collect();
            black_box(diff);
        });
    });

    // Benchmark: 100% change (worst case)
    group.bench_function("100%_change", |b| {
        let mut buffer = Buffer::new(size);
        fill_buffer(&mut buffer, 'a', size.width * size.height);
        buffer.clear();
        fill_buffer(&mut buffer, 'x', size.width * size.height);

        b.iter(|| {
            let diff: Vec<_> = buffer.diff_lines().collect();
            black_box(diff);
        });
    });

    // Benchmark: Process line changes (realistic inline mode scenario)
    group.bench_function("process_line_changes", |b| {
        let mut buffer = Buffer::new(size);
        fill_buffer(&mut buffer, 'a', size.width * size.height);
        buffer.clear();
        fill_buffer(&mut buffer, 'a', size.width * size.height);
        // Change first 2 lines (typical inline mode update)
        fill_buffer(&mut buffer, 'x', size.width * 2);

        b.iter(|| {
            let mut changed_count = 0;
            let mut unchanged_count = 0;
            let mut total_cells = 0;

            for line_diff in buffer.diff_lines() {
                match line_diff.state {
                    LineState::Unchanged => {
                        unchanged_count += 1;
                    }
                    LineState::Changed { cells, .. } => {
                        changed_count += 1;
                        for cell in cells {
                            total_cells += cell.width();
                        }
                    }
                }
            }

            black_box((changed_count, unchanged_count, total_cells));
        });
    });

    group.finish();
}

/// Benchmark wide character handling
fn bench_wide_chars(c: &mut Criterion) {
    let mut group = c.benchmark_group("wide_chars");

    let size = Size {
        width: 80,
        height: 24,
    };

    // Benchmark: Cell diff with wide chars
    group.bench_function("cell_diff_wide", |b| {
        let mut buffer = Buffer::new(size);
        fill_buffer_wide(&mut buffer, '中', size.width * size.height / 2);
        buffer.clear();
        fill_buffer_wide(&mut buffer, '文', size.width * size.height / 2);

        b.iter(|| {
            let diff: Vec<_> = buffer.diff().unwrap().collect();
            black_box(diff);
        });
    });

    // Benchmark: Line diff with wide chars
    group.bench_function("line_diff_wide", |b| {
        let mut buffer = Buffer::new(size);
        fill_buffer_wide(&mut buffer, '中', size.width * size.height / 2);
        buffer.clear();
        fill_buffer_wide(&mut buffer, '文', size.width * size.height / 2);

        b.iter(|| {
            let diff: Vec<_> = buffer.diff_lines().collect();
            black_box(diff);
        });
    });

    // Benchmark: Mixed ASCII and wide chars
    group.bench_function("line_diff_mixed", |b| {
        let mut buffer = Buffer::new(size);
        // Fill with pattern: "a中b文c..."
        for i in 0..(size.width * size.height / 4) {
            let pos = Position {
                x: (i * 4) % size.width,
                y: (i * 4) / size.width,
            };
            buffer.set(pos, Stylized::plain('a')).ok();
            buffer
                .set(
                    Position {
                        x: pos.x + 1,
                        y: pos.y,
                    },
                    Stylized::plain('中'),
                )
                .ok();
            if pos.x + 3 < size.width {
                buffer
                    .set(
                        Position {
                            x: pos.x + 3,
                            y: pos.y,
                        },
                        Stylized::plain('b'),
                    )
                    .ok();
            }
        }

        buffer.clear();

        // Change to different pattern
        for i in 0..(size.width * size.height / 4) {
            let pos = Position {
                x: (i * 4) % size.width,
                y: (i * 4) / size.width,
            };
            buffer.set(pos, Stylized::plain('x')).ok();
            buffer
                .set(
                    Position {
                        x: pos.x + 1,
                        y: pos.y,
                    },
                    Stylized::plain('文'),
                )
                .ok();
            if pos.x + 3 < size.width {
                buffer
                    .set(
                        Position {
                            x: pos.x + 3,
                            y: pos.y,
                        },
                        Stylized::plain('y'),
                    )
                    .ok();
            }
        }

        b.iter(|| {
            let diff: Vec<_> = buffer.diff_lines().collect();
            black_box(diff);
        });
    });

    group.finish();
}

/// Benchmark comparison: cell diff vs line diff
fn bench_diff_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_comparison");

    let size = Size {
        width: 80,
        height: 24,
    };

    // Scenario: Spinner + Progress bar (2 lines change out of 24)
    let mut buffer = Buffer::new(size);
    fill_buffer(&mut buffer, 'a', size.width * size.height);
    buffer.clear();
    fill_buffer(&mut buffer, 'a', size.width * size.height);
    fill_buffer(&mut buffer, 'x', size.width * 2);

    group.bench_function("cell_diff_2_lines", |b| {
        b.iter(|| {
            let diff: Vec<_> = buffer.diff().unwrap().collect();
            black_box(diff);
        });
    });

    group.bench_function("line_diff_2_lines", |b| {
        b.iter(|| {
            let diff: Vec<_> = buffer.diff_lines().collect();
            black_box(diff);
        });
    });

    group.finish();
}

/// Benchmark cell diff scaling across different buffer sizes
fn bench_diff_scaling_cell(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_scaling_cell");

    // Test different buffer sizes from small to extreme
    let sizes = vec![
        ("10x10", 10, 10),         // Tiny: 100 cells
        ("40x25", 40, 25),         // Small: 1,000 cells (typical terminal)
        ("80x24", 80, 24),         // Standard: 1,920 cells (80x24 terminal)
        ("100x50", 100, 50),       // Medium: 5,000 cells
        ("160x48", 160, 48),       // Large: 7,680 cells (retina terminal)
        ("200x100", 200, 100),     // XLarge: 20,000 cells
        ("400x200", 400, 200),     // XXLarge: 80,000 cells
        ("1000x500", 1000, 500),   // Huge: 500,000 cells
        ("2000x1000", 2000, 1000), // Extreme: 2,000,000 cells
    ];

    for (name, width, height) in sizes {
        let size = Size { width, height };
        let total_cells = (width as u64) * (height as u64);
        group.throughput(Throughput::Elements(total_cells));

        // Test 10% change scenario (realistic)
        group.bench_with_input(BenchmarkId::new("10%_change", name), &size, |b, &size| {
            let mut buffer = Buffer::new(size);
            fill_buffer(&mut buffer, 'a', size.width * size.height);
            buffer.clear();
            fill_buffer(&mut buffer, 'a', size.width * size.height);
            // Change 10%
            let change_count = (size.width * size.height) / 10;
            fill_buffer(&mut buffer, 'x', change_count);

            b.iter(|| {
                let diff: Vec<_> = buffer.diff().unwrap().collect();
                black_box(diff);
            });
        });

        // Test no change scenario (best case)
        group.bench_with_input(BenchmarkId::new("no_change", name), &size, |b, &size| {
            let mut buffer = Buffer::new(size);
            fill_buffer(&mut buffer, 'a', size.width * size.height);
            buffer.clear();
            fill_buffer(&mut buffer, 'a', size.width * size.height);

            b.iter(|| {
                let diff: Vec<_> = buffer.diff().unwrap().collect();
                black_box(diff);
            });
        });
    }

    group.finish();
}

/// Benchmark line diff scaling across different buffer sizes
fn bench_diff_scaling_line(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_scaling_line");

    // Test different buffer sizes from small to extreme
    let sizes = vec![
        ("10x10", 10, 10),         // Tiny: 10 lines
        ("40x25", 40, 25),         // Small: 25 lines
        ("80x24", 80, 24),         // Standard: 24 lines (80x24 terminal)
        ("100x50", 100, 50),       // Medium: 50 lines
        ("160x48", 160, 48),       // Large: 48 lines
        ("200x100", 200, 100),     // XLarge: 100 lines
        ("400x200", 400, 200),     // XXLarge: 200 lines
        ("1000x500", 1000, 500),   // Huge: 500 lines
        ("2000x1000", 2000, 1000), // Extreme: 1000 lines
    ];

    for (name, width, height) in sizes {
        let size = Size { width, height };
        group.throughput(Throughput::Elements(height as u64));

        // Test 1 line change (typical inline mode)
        group.bench_with_input(
            BenchmarkId::new("1_line_change", name),
            &size,
            |b, &size| {
                let mut buffer = Buffer::new(size);
                fill_buffer(&mut buffer, 'a', size.width * size.height);
                buffer.clear();
                fill_buffer(&mut buffer, 'a', size.width * size.height);
                // Change first line
                fill_buffer(&mut buffer, 'x', size.width);

                b.iter(|| {
                    let diff: Vec<_> = buffer.diff_lines().collect();
                    black_box(diff);
                });
            },
        );

        // Test 10% lines change
        group.bench_with_input(
            BenchmarkId::new("10%_lines_change", name),
            &size,
            |b, &size| {
                let mut buffer = Buffer::new(size);
                fill_buffer(&mut buffer, 'a', size.width * size.height);
                buffer.clear();
                fill_buffer(&mut buffer, 'a', size.width * size.height);
                // Change 10% of lines
                let change_lines = size.height / 10;
                fill_buffer(&mut buffer, 'x', size.width * change_lines);

                b.iter(|| {
                    let diff: Vec<_> = buffer.diff_lines().collect();
                    black_box(diff);
                });
            },
        );

        // Test no change (best case)
        group.bench_with_input(BenchmarkId::new("no_change", name), &size, |b, &size| {
            let mut buffer = Buffer::new(size);
            fill_buffer(&mut buffer, 'a', size.width * size.height);
            buffer.clear();
            fill_buffer(&mut buffer, 'a', size.width * size.height);

            b.iter(|| {
                let diff: Vec<_> = buffer.diff_lines().collect();
                black_box(diff);
            });
        });
    }

    group.finish();
}

/// Benchmark extreme sizes to test scalability limits
fn bench_extreme_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("extreme_sizes");
    // Use longer measurement time for extreme sizes
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(10));

    // Extreme sizes
    let sizes = vec![
        ("2000x1000", 2000, 1000), // 2M cells - Ultra HD terminal
        ("4000x2000", 4000, 2000), // 8M cells - 8K display
    ];

    for (name, width, height) in sizes {
        let size = Size { width, height };
        let total_cells = (width as u64) * (height as u64);

        // Cell diff - 1% change (realistic for large screens)
        group.throughput(Throughput::Elements(total_cells));
        group.bench_with_input(BenchmarkId::new("cell_diff_1%", name), &size, |b, &size| {
            let mut buffer = Buffer::new(size);
            fill_buffer(&mut buffer, 'a', size.width * size.height);
            buffer.clear();
            fill_buffer(&mut buffer, 'a', size.width * size.height);
            // Change 1%
            let change_count = (size.width * size.height) / 100;
            fill_buffer(&mut buffer, 'x', change_count);

            b.iter(|| {
                let diff: Vec<_> = buffer.diff().unwrap().collect();
                black_box(diff);
            });
        });

        // Line diff - 1 line change (typical update)
        group.throughput(Throughput::Elements(height as u64));
        group.bench_with_input(
            BenchmarkId::new("line_diff_1_line", name),
            &size,
            |b, &size| {
                let mut buffer = Buffer::new(size);
                fill_buffer(&mut buffer, 'a', size.width * size.height);
                buffer.clear();
                fill_buffer(&mut buffer, 'a', size.width * size.height);
                // Change first line
                fill_buffer(&mut buffer, 'x', size.width);

                b.iter(|| {
                    let diff: Vec<_> = buffer.diff_lines().collect();
                    black_box(diff);
                });
            },
        );

        // Line diff - 10 lines change
        group.bench_with_input(
            BenchmarkId::new("line_diff_10_lines", name),
            &size,
            |b, &size| {
                let mut buffer = Buffer::new(size);
                fill_buffer(&mut buffer, 'a', size.width * size.height);
                buffer.clear();
                fill_buffer(&mut buffer, 'a', size.width * size.height);
                // Change first 10 lines
                fill_buffer(&mut buffer, 'x', size.width * 10);

                b.iter(|| {
                    let diff: Vec<_> = buffer.diff_lines().collect();
                    black_box(diff);
                });
            },
        );

        // Memory/allocation test - clear operation
        group.bench_with_input(BenchmarkId::new("clear", name), &size, |b, &size| {
            let mut buffer = Buffer::new(size);
            fill_buffer(&mut buffer, 'a', size.width * size.height);

            b.iter(|| {
                buffer.clear();
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_buffer_operations,
    bench_cell_diff,
    bench_line_diff,
    bench_wide_chars,
    bench_diff_comparison,
    bench_diff_scaling_cell,
    bench_diff_scaling_line,
    bench_extreme_sizes
);
criterion_main!(benches);
