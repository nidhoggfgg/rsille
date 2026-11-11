//! Independent panels shared by the REPL composer and CLI demo.
use crate::{notebook::FormulaOutput, Plot};
use rsille::render::{area::Area, chunk::Chunk};
use rsille::tui::prelude::*;
use std::{
    rc::Rc,
    time::{Duration, Instant},
};

pub(crate) const MAX_PANELS: usize = 6;
const TIMER: &str = "grid-rotation";
const INTERVAL: Duration = Duration::from_millis(50);
const PRESETS: [&str; 4] = [
    "surface(sin(x)*cos(y), -pi, pi, -pi, pi)",
    "param3(cos(t), sin(t), t/4, 0, 6*pi)",
    "param3((2+cos(3*t))*cos(2*t), (2+cos(3*t))*sin(2*t), sin(3*t), 0, 2*pi)",
    "polar(2*cos(5*theta), 0, 2*pi)",
];

#[derive(Clone)]
pub(crate) struct Panel {
    pub(crate) plot: Rc<Plot>,
    pub(crate) bounds: [f64; 4],
}

pub(crate) struct State {
    panels: Vec<Panel>,
    pixels: (u16, u16),
    angle: f64,
    started: Option<Instant>,
    frozen: bool,
}
impl State {
    pub(crate) fn new(mut sources: Vec<String>, range: [f64; 2]) -> Result<Self, String> {
        if sources.is_empty() {
            sources = PRESETS.iter().map(|s| (*s).into()).collect();
        }
        if sources.len() > MAX_PANELS {
            return Err("--grid accepts one to six panels".into());
        }
        let panels = sources
            .iter()
            .enumerate()
            .map(|(i, source)| {
                let plot = if source.len() > 4096 {
                    Err("Expression exceeds 4096 bytes".into())
                } else {
                    Plot::parse(source)
                }
                .map_err(|e| format!("Panel {}: {e}", i + 1))?;
                let bounds = plot.auto_bounds(range);
                Ok(Panel {
                    plot: Rc::new(plot),
                    bounds,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(Self {
            panels,
            pixels: (1, 2),
            angle: 0.0,
            started: None,
            frozen: false,
        })
    }

    fn update(&mut self, msg: Msg, ctx: &mut UpdateCtx<Msg>) {
        match msg {
            Msg::Tick if !self.frozen && self.panels.iter().any(|p| p.plot.is_spatial()) => {
                let started = *self.started.get_or_insert(ctx.now());
                self.angle = (ctx.now().saturating_duration_since(started).as_secs_f64() * 0.6)
                    .rem_euclid(std::f64::consts::TAU);
                ctx.debounce(TIMER, INTERVAL, Msg::Tick);
            }
            Msg::Submit if !self.frozen => {
                self.frozen = true;
                ctx.cancel_debounce(TIMER);
                ctx.commit_frame(Msg::Exit);
            }
            Msg::Exit => ctx.exit(),
            _ => {}
        }
    }
}

#[derive(Clone)]
pub(crate) enum Msg {
    Tick,
    Submit,
    Exit,
}

// Split the available cells, not a guessed pixel viewport. A narrow terminal
// stacks panels; tiny heights degrade to clipped/empty cells without overlap.
fn cells(width: u16, height: u16, count: usize) -> Vec<(u16, u16, u16, u16)> {
    let columns = if width >= 80 && count > 1 { 2 } else { 1 };
    let rows = count.div_ceil(columns);
    (0..count)
        .map(|i| {
            let x0 = (i % columns) * usize::from(width) / columns;
            let x1 = (i % columns + 1) * usize::from(width) / columns;
            let y0 = (i / columns) * usize::from(height) / rows;
            let y1 = (i / columns + 1) * usize::from(height) / rows;
            (
                x0 as u16,
                y0 as u16,
                (x1 - x0).saturating_sub(1) as u16,
                (y1 - y0) as u16,
            )
        })
        .collect()
}

pub(crate) struct Grid<M> {
    frozen: bool,
    submit: Option<fn() -> M>,
    children: Vec<Box<dyn Widget<M>>>,
}
impl<M: 'static> Widget<M> for Grid<M> {
    fn children(&self) -> &[Box<dyn Widget<M>>] {
        &self.children
    }
    fn constraints(&self) -> Constraints {
        Constraints {
            min_width: 1,
            min_height: 30,
            max_height: Some(30),
            ..Constraints::fill()
        }
    }
    fn layout(&self, area: Area, _: &MeasureCtx) -> Layout {
        let panels = cells(
            area.width(),
            area.height().saturating_sub(1),
            self.children.len() / 3,
        );
        // Share the tallest measured formula in each grid row so graphs align.
        // Reserve the heading and at least three graph rows whenever possible.
        let heights: Vec<_> = panels
            .iter()
            .map(|&(_, y, _, h)| {
                panels
                    .iter()
                    .enumerate()
                    .filter(|(_, cell)| cell.1 == y)
                    .map(|(i, _)| self.children[i * 3 + 1].constraints().min_height)
                    .max()
                    .unwrap_or(0)
                    .min(h.saturating_sub(4))
            })
            .collect();
        Layout::new(
            panels
                .into_iter()
                .enumerate()
                .flat_map(|(i, (x, y, w, h))| {
                    let formula = heights[i];
                    let heading = h.min(1);
                    let graph = h.saturating_sub(heading + formula);
                    [
                        ChildLayout::new(
                            i * 3,
                            Area::new((area.x() + x, area.y() + y + 1).into(), (w, heading).into()),
                        ),
                        ChildLayout::new(
                            i * 3 + 1,
                            Area::new(
                                (area.x() + x, area.y() + y + 1 + heading).into(),
                                (w, formula).into(),
                            ),
                        ),
                        ChildLayout::new(
                            i * 3 + 2,
                            Area::new(
                                (area.x() + x, area.y() + y + 1 + heading + formula).into(),
                                (w, graph).into(),
                            ),
                        ),
                    ]
                }),
        )
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let _ = chunk.set_string(
            0,
            0,
            if self.frozen {
                "Grid · submitted (static)"
            } else if self.submit.is_some() {
                "Grid preview · Enter/F5: freeze & commit · Esc: quit"
            } else {
                "Grid preview (not submitted)"
            },
            Style::default().to_render_style(),
        );
        ctx.render_children(chunk, self);
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        let Some(submit) = self.submit else { return };
        if !matches!(ctx.phase(), EventPhase::Capture | EventPhase::Target) {
            return;
        }
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press
                && key.modifiers == KeyModifiers::NONE
                && matches!(key.code, KeyCode::Enter | KeyCode::F(5))
            {
                ctx.emit(submit());
                ctx.stop_propagation();
            }
        }
    }
}
struct Graph {
    plot: Rc<Plot>,
    bounds: [f64; 4],
    pixels: (u16, u16),
    angle: f64,
}
impl<M: 'static> Widget<M> for Graph {
    fn constraints(&self) -> Constraints {
        Constraints::fill()
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let area = ctx.layout_target().unwrap_or_else(|| chunk.area());
        if area.width() == 0 || area.height() == 0 {
            return;
        }
        let size = (
            u32::from(area.width()) * u32::from(self.pixels.0),
            u32::from(area.height()) * u32::from(self.pixels.1),
        );
        raster_canvas::<M>(self.plot.image_at(self.bounds, size, self.angle))
            .align_x(RasterAlign::Start)
            .render(chunk, ctx);
    }
}
fn view(state: &State) -> Grid<Msg> {
    let mut grid = panels_view(&state.panels, state.pixels, state.angle, state.frozen);
    grid.submit = Some(|| Msg::Submit);
    grid
}

pub(crate) fn panels_view<M: 'static>(
    panels: &[Panel],
    pixels: (u16, u16),
    angle: f64,
    frozen: bool,
) -> Grid<M> {
    let mut children: Vec<Box<dyn Widget<M>>> = Vec::new();
    for (i, panel) in panels.iter().enumerate() {
        children.push(Box::new(label(format!(
            "Panel {} · {}",
            i + 1,
            if panel.plot.is_spatial() { "3D" } else { "2D" }
        ))));
        children.push(Box::new(
            FormulaOutput {
                source: panel.plot.clone(),
                cell_pixels: pixels,
            }
            .scope(format!("panel-{i}")),
        ));
        children.push(Box::new(Graph {
            plot: panel.plot.clone(),
            bounds: panel.bounds,
            pixels,
            angle,
        }));
    }
    Grid {
        children,
        frozen,
        submit: None,
    }
}
fn builder(mut state: State, output: RasterOutput) -> App<State, Msg> {
    if output.policy == RasterPolicy::RequireSixel {
        if let Some(pixels) = output.capabilities.cell_pixels {
            state.pixels = (pixels.width, pixels.height);
        }
    }
    App::new(state)
        .mode(Mode::Inline)
        .disable_mouse_capture()
        .with_motion_policy(MotionPolicy::disabled())
        .raster_output(output)
        .with_quit_key(KeyCode::Esc)
}
pub(crate) fn application(state: State, output: RasterOutput) -> Runtime<State, Msg> {
    let runtime = builder(state, output)
        .update(State::update)
        .view(view)
        .build();
    runtime.handle().send(Msg::Tick).expect("runtime is alive");
    runtime
}
pub(crate) fn smoke_test(state: State) -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = application(state, RasterOutput::text_only()).size(100, 32);
    runtime.step(&[])?;
    assert!(runtime.frame().has_raster());
    runtime.handle().send(Msg::Submit)?;
    assert!(runtime.step(&[])?.committed);
    assert!(runtime.state().frozen);
    runtime.step(&[])?;
    println!("Grid passed: independent panels, raster preview and frozen commit");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn validation_and_independent_ranges_and_caches() {
        assert!(State::new(vec!["sin(x)".into(); 7], crate::X_RANGE).is_err());
        for source in ["", "sin(x);", "surface(x, 1, 0, 0, 1)"] {
            let error = State::new(vec!["x".into(), source.into()], crate::X_RANGE)
                .err()
                .unwrap();
            assert!(error.starts_with("Panel 2:"));
        }
        assert!(State::new(vec!["x".repeat(4097)], crate::X_RANGE).is_err());
        let state = State::new(
            vec![
                "0.01*sin(x); 0.02*cos(x)".into(),
                "1000*x".into(),
                "param(t,t^2,0,1)".into(),
                PRESETS[0].into(),
            ],
            [-2.0, 3.0],
        )
        .unwrap();
        let a = &state.panels[0];
        let b = &state.panels[1];
        assert!(a.bounds[3] < 0.1);
        assert!(b.bounds[3] > 2000.0);
        assert_eq!(&a.bounds[..2], &[-2.0, 3.0]);
        assert!(state.panels[2].bounds[0] > -1.0);
        let first = a.plot.image_at(a.bounds, (80, 40), 0.0);
        b.plot.image_at(b.bounds, (40, 20), 0.5);
        assert!(Arc::ptr_eq(
            &first,
            &a.plot.image_at(a.bounds, (80, 40), 1.0)
        ));
        assert!(!Arc::ptr_eq(
            &first,
            &a.plot.image_at(a.bounds, (81, 40), 0.0)
        ));
    }

    #[test]
    fn responsive_cells_are_bounded_and_nonoverlapping() {
        for width in [0, 1, 30, 79, 80, 101] {
            for height in [0, 1, 4, 29, 50] {
                for count in 1..=6 {
                    let areas = cells(width, height, count);
                    assert_eq!(areas.len(), count);
                    for (i, &(x, y, w, h)) in areas.iter().enumerate() {
                        assert!(x + w <= width && y + h <= height);
                        for &(a, b, c, d) in &areas[..i] {
                            assert!(
                                w == 0
                                    || h == 0
                                    || c == 0
                                    || d == 0
                                    || x + w <= a
                                    || a + c <= x
                                    || y + h <= b
                                    || b + d <= y
                            );
                        }
                    }
                }
            }
        }
        assert_eq!(cells(79, 29, 4)[1].0, 0);
        assert!(cells(80, 29, 4)[1].0 > 0);
    }

    #[test]
    fn native_grid_measures_formulas_aligns_graphs_and_clips_small_viewports() {
        for (width, height) in [(100, 32), (160, 32), (30, 20), (100, 10)] {
            let mut state = State::new(Vec::new(), crate::X_RANGE).unwrap();
            state.pixels = (8, 16);
            let mut rt = builder(state, RasterOutput::text_only())
                .update(State::update)
                .view(view)
                .build()
                .size(width, height);
            rt.step(&[]).unwrap();
            let origins: Vec<_> = rt
                .frame()
                .raster_cells()
                .filter(|(_, _, r)| r.x == 0 && r.y == 0)
                .collect();
            let mut graph_tops = Vec::new();

            let area = rt.layout_geometry(&WidgetPath::root()).unwrap().target;
            let grid_cells = cells(area.width(), area.height().saturating_sub(1), 4);
            for (i, &(x, y, w, h)) in grid_cells.iter().enumerate() {
                let measured = grid_cells
                    .iter()
                    .enumerate()
                    .filter(|(_, cell)| cell.1 == y)
                    .map(|(j, _)| rt.state().panels[j].plot.formula.height(16))
                    .max()
                    .unwrap();
                let rows = measured.min(h.saturating_sub(4));
                if rows > 0 {
                    let expected = rt.state().panels[i]
                        .plot
                        .formula
                        .image((u32::from(w) * 8, u32::from(rows) * 16), 16)
                        .unwrap();
                    assert!(origins.iter().any(|(a, b, r)| *a == x
                        && *b == y + 2
                        && r.canvas.pixels() == expected.pixels()), "viewport {width}x{height}, panel {i}, expected {x},{} {w}x{rows}; origins {:?}", y + 2, origins.iter().map(|(x,y,r)| (*x,*y,r.target.width,r.target.height)).collect::<Vec<_>>());
                }
                let graph_y = y + 2 + rows;
                assert!(origins.iter().any(|(a, b, _)| *a == x && *b == graph_y));
                assert!(h.saturating_sub(1 + rows) >= 1);
                graph_tops.push(graph_y);
            }
            if width >= 80 {
                assert_eq!(graph_tops[0], graph_tops[1]);
                assert_eq!(graph_tops[2], graph_tops[3]);
            }
        }
    }

    #[test]
    fn native_sizes_rotation_and_static_commit() {
        for pixels in [(1, 2), (8, 16)] {
            let clock = rsille::tui::runtime::ManualClock::default();
            let mut state = State::new(Vec::new(), crate::X_RANGE).unwrap();
            state.pixels = pixels;
            let mut rt = builder(state, RasterOutput::text_only())
                .with_clock(clock.clone())
                .update(State::update)
                .view(view)
                .build()
                .size(100, 32);
            rt.handle().send(Msg::Tick).unwrap();
            rt.step(&[]).unwrap();
            let images = |rt: &Runtime<State, Msg>| {
                let area = rt.layout_geometry(&WidgetPath::root()).unwrap().target;
                let formula_tops: Vec<_> = cells(area.width(), area.height().saturating_sub(1), 4)
                    .iter()
                    .map(|cell| area.y() + cell.1 + 2)
                    .collect();
                rt.frame()
                    .raster_cells()
                    .filter(|(_, y, r)| r.x == 0 && r.y == 0 && !formula_tops.contains(y))
                    .map(|(_, _, r)| {
                        assert_eq!(
                            r.canvas.width(),
                            u32::from(r.target.width) * u32::from(pixels.0)
                        );
                        assert_eq!(
                            r.canvas.height(),
                            u32::from(r.target.height) * u32::from(pixels.1)
                        );
                        r.canvas.clone()
                    })
                    .collect::<Vec<_>>()
            };
            let original = images(&rt);
            assert_eq!(original.len(), 4);
            clock.advance(Duration::from_secs(1));
            rt.step(&[]).unwrap();
            let preview = images(&rt);
            assert_ne!(original[0].pixels(), preview[0].pixels());
            assert_eq!(original[3].pixels(), preview[3].pixels());
            let angle = rt.state().angle;
            assert!(
                rt.step(&[Event::Key(KeyEvent::new(
                    KeyCode::Enter,
                    KeyModifiers::NONE
                ))])
                .unwrap()
                .committed
            );
            assert_eq!(rt.state().angle, angle);
            for (a, b) in preview.iter().zip(images(&rt)) {
                assert_eq!(a.pixels(), b.pixels());
            }
            assert!(rt.state().frozen);

            rt.handle().send(Msg::Tick).unwrap();
            clock.advance(Duration::from_secs(5));
            rt.step(&[]).unwrap();
            assert_eq!(rt.state().angle, angle);
        }
    }
}
