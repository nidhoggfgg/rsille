//! Compare identical headless scenes with reused versus invalidated layout.
//! `cargo bench -p tui --bench layout`
use std::{
    alloc::{GlobalAlloc, Layout as AllocLayout, System},
    cell::Cell,
    hint::black_box,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use render::chunk::Chunk;
use tui::prelude::*;

struct CountingAllocator;
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: AllocLayout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: AllocLayout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: AllocLayout, size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: AllocLayout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

struct CountedLabel {
    label: Label,
    measurements: Rc<Cell<u64>>,
}
impl Widget<()> for CountedLabel {
    fn constraints(&self) -> Constraints {
        Widget::constraints(&self.label)
    }
    fn layout_style_with(&self, ctx: &MeasureCtx) -> LayoutStyle {
        self.label.layout_style_with(ctx)
    }
    fn measure(&self, proposal: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
        self.measurements.set(self.measurements.get() + 1);
        self.label.measure(proposal, ctx)
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        self.label.render(chunk, ctx);
    }
}

fn run(force_layout: bool) {
    let measurements = Rc::new(Cell::new(0));
    let measured = measurements.clone();
    let mut app = App::new(())
        .with_motion_policy(MotionPolicy::disabled())
        .build(
            |_, ()| {},
            move |_| {
                let mut scene = col();
                for _ in 0..8 {
                    let mut line = row();
                    for _ in 0..8 {
                        line = line.child(CountedLabel {
                            label: label("中文👩🏽‍💻 text").wrap_char(),
                            measurements: measured.clone(),
                        });
                    }
                    scene = scene.child(line);
                }
                scene
            },
        )
        .size(100, 24);
    app.step(&[]).unwrap();
    let layout_before = app.layout_stats();
    let measures_before = measurements.get();
    let allocations_before = ALLOCATIONS.load(Ordering::Relaxed);
    let started = Instant::now();
    const FRAMES: u64 = 2000;
    for _ in 0..FRAMES {
        if force_layout {
            app.request_layout();
        } else {
            app.request_paint();
        }
        black_box(app.step(&[]).unwrap());
        black_box(app.frame());
    }
    let elapsed = started.elapsed();
    let allocations = ALLOCATIONS.load(Ordering::Relaxed) - allocations_before;
    println!(
        "{}: measure/frame={:.0}, layout/frame={:.0}, allocations/frame={:.0}, us/frame={:.1}",
        if force_layout {
            "layout + paint"
        } else {
            "paint only    "
        },
        (measurements.get() - measures_before) as f64 / FRAMES as f64,
        (app.layout_stats().computations - layout_before.computations) as f64 / FRAMES as f64,
        allocations as f64 / FRAMES as f64,
        elapsed.as_secs_f64() * 1_000_000.0 / FRAMES as f64,
    );
}
fn main() {
    run(false);
    run(true);
}
