//! Immutable layout output, shared by the layout and paint phases.

use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

use render::area::Area;

use crate::widget::{MeasureCtx, Widget, WidgetKey, WidgetPath};

/// The allocation of a child in `Widget::children()`. Order is paint order.
#[derive(Debug, Clone, Copy)]
pub struct ChildLayout {
    pub index: usize,
    pub area: Area,
}

impl ChildLayout {
    pub fn new(index: usize, area: Area) -> Self {
        Self { index, area }
    }
}

/// A node's layout output. Optional immutable data can hold container-specific
/// geometry (for example a divider or scroll viewport), avoiding paint-time measurement.
#[derive(Clone, Default)]
pub struct Layout {
    pub children: Vec<ChildLayout>,
    data: Option<Rc<dyn Any>>,
}

impl Layout {
    pub fn new(children: impl IntoIterator<Item = ChildLayout>) -> Self {
        Self {
            children: children.into_iter().collect(),
            data: None,
        }
    }

    pub fn with_data<T: 'static>(mut self, data: T) -> Self {
        self.data = Some(Rc::new(data));
        self
    }

    pub fn data<T: 'static>(&self) -> Option<&T> {
        self.data.as_ref()?.downcast_ref()
    }
}

/// Geometry from the last paint. Target and display use the parent's layout
/// space; clip and hit use screen coordinates. Scroll content supplies the
/// translation separately, retaining complete allocations for local input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutGeometry {
    pub target: Area,
    pub display: Area,
    pub clip: Area,
    pub hit: Option<Area>,
    pub coordinate_offset: (i32, i32),
}

/// Requests are independent: scheduling an update need not paint or lay out.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Invalidation {
    pub layout: bool,
    pub paint: bool,
    pub next_frame: bool,
}

impl Invalidation {
    pub fn request_paint(&mut self) {
        self.paint = true;
    }
    pub fn request_layout(&mut self) {
        self.layout = true;
        self.paint = true;
    }
    pub fn request_next_frame(&mut self) {
        self.next_frame = true;
    }
    pub(crate) fn merge(&mut self, other: Self) {
        self.layout |= other.layout;
        self.paint |= other.paint;
        self.next_frame |= other.next_frame;
    }
}

/// Cumulative counters for comparing the same scene across paint-only and
/// layout updates. These count framework layout calls, including leaves.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LayoutStats {
    pub computations: u64,
    pub cache_hits: u64,
}

#[derive(Default)]
pub(crate) struct LayoutCache {
    nodes: RefCell<HashMap<WidgetPath, (Area, Rc<Layout>)>>,
    stats: std::cell::Cell<LayoutStats>,
}

impl LayoutCache {
    pub fn clear(&self) {
        self.nodes.borrow_mut().clear();
    }
    pub fn stats(&self) -> LayoutStats {
        self.stats.get()
    }

    pub fn resolve<M>(&self, widget: &dyn Widget<M>, area: Area, ctx: &MeasureCtx) -> Rc<Layout> {
        let mut stats = self.stats.get();
        if let Some((previous_area, layout)) = self.nodes.borrow().get(ctx.path()) {
            if *previous_area == area {
                stats.cache_hits += 1;
                self.stats.set(stats);
                return layout.clone();
            }
        }
        stats.computations += 1;
        self.stats.set(stats);
        let layout = Rc::new(widget.layout(area, ctx));
        for child in &layout.children {
            let widget = widget.children().get(child.index).unwrap_or_else(|| {
                panic!("Invalid layout child {} at {:?}", child.index, ctx.path())
            });
            if child.area.width() > 0 && child.area.height() > 0 {
                self.resolve(
                    widget.as_ref(),
                    child.area,
                    &ctx.child_ctx(WidgetKey::for_child(child.index, widget.as_ref())),
                );
            }
        }
        self.nodes
            .borrow_mut()
            .insert(ctx.path().clone(), (area, layout.clone()));
        layout
    }
}
