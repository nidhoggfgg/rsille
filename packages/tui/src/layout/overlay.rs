//! Layered containers for overlays, popups, and anchored floating UI.

use crate::layout::{ChildLayout, Layout};
use render::area::Area;

use crate::event::Event;
use crate::focus::{FocusConfig, FocusScope};
use crate::layout::{AxisLimit, Constraints, MeasuredSize, SizeProposal};
use crate::widget::{EventCtx, IntoWidget, MeasureCtx, RenderCtx, Widget, WidgetKey};

/// Cardinal anchor positions used for floating layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayAnchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

/// Rectangle used to anchor a floating layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnchorRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl AnchorRect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn as_area(self, container: Area) -> Area {
        Area::new(
            (container.x() + self.x, container.y() + self.y).into(),
            (self.width, self.height).into(),
        )
    }
}

impl From<(u16, u16, u16, u16)> for AnchorRect {
    fn from(value: (u16, u16, u16, u16)) -> Self {
        Self::new(value.0, value.1, value.2, value.3)
    }
}

/// Placement strategy for an overlay child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayPlacement {
    Fill,
    Floating {
        anchor: OverlayAnchor,
        popup_anchor: OverlayAnchor,
        offset_x: i16,
        offset_y: i16,
        width: Option<u16>,
        height: Option<u16>,
    },
    Anchored {
        rect: AnchorRect,
        anchor: OverlayAnchor,
        popup_anchor: OverlayAnchor,
        offset_x: i16,
        offset_y: i16,
        width: Option<u16>,
        height: Option<u16>,
    },
}

impl Default for OverlayPlacement {
    fn default() -> Self {
        Self::Fill
    }
}

/// Builder for a single floating layer.
pub struct OverlayLayer<M = ()> {
    widget: Box<dyn Widget<M>>,
    placement: OverlayPlacement,
    z_index: i16,
}

impl<M> std::fmt::Debug for OverlayLayer<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OverlayLayer")
            .field("placement", &self.placement)
            .field("z_index", &self.z_index)
            .finish()
    }
}

impl<M> OverlayLayer<M> {
    pub fn new(widget: impl IntoWidget<M>) -> Self {
        Self {
            widget: widget.into_widget(),
            placement: OverlayPlacement::Floating {
                anchor: OverlayAnchor::Center,
                popup_anchor: OverlayAnchor::Center,
                offset_x: 0,
                offset_y: 0,
                width: None,
                height: None,
            },
            z_index: 0,
        }
    }

    pub fn placement(mut self, placement: OverlayPlacement) -> Self {
        self.placement = placement;
        self
    }

    pub fn floating(mut self, anchor: OverlayAnchor) -> Self {
        self.placement = OverlayPlacement::Floating {
            anchor,
            popup_anchor: anchor,
            offset_x: 0,
            offset_y: 0,
            width: None,
            height: None,
        };
        self
    }

    pub fn anchored(
        mut self,
        rect: impl Into<AnchorRect>,
        anchor: OverlayAnchor,
        popup_anchor: OverlayAnchor,
    ) -> Self {
        self.placement = OverlayPlacement::Anchored {
            rect: rect.into(),
            anchor,
            popup_anchor,
            offset_x: 0,
            offset_y: 0,
            width: None,
            height: None,
        };
        self
    }

    pub fn offset(mut self, offset_x: i16, offset_y: i16) -> Self {
        match &mut self.placement {
            OverlayPlacement::Fill => {}
            OverlayPlacement::Floating {
                offset_x: layer_x,
                offset_y: layer_y,
                ..
            }
            | OverlayPlacement::Anchored {
                offset_x: layer_x,
                offset_y: layer_y,
                ..
            } => {
                *layer_x = offset_x;
                *layer_y = offset_y;
            }
        }
        self
    }

    pub fn size(mut self, width: u16, height: u16) -> Self {
        match &mut self.placement {
            OverlayPlacement::Fill => {}
            OverlayPlacement::Floating {
                width: layer_width,
                height: layer_height,
                ..
            }
            | OverlayPlacement::Anchored {
                width: layer_width,
                height: layer_height,
                ..
            } => {
                *layer_width = Some(width);
                *layer_height = Some(height);
            }
        }
        self
    }

    pub fn z_index(mut self, z_index: i16) -> Self {
        self.z_index = z_index;
        self
    }
}

/// Stack renders all children into the same area in insertion order.
pub struct Stack<M = ()> {
    children: Vec<Box<dyn Widget<M>>>,
    widget_key: Option<String>,
    focus_scope: Option<FocusScope>,
}

impl<M> std::fmt::Debug for Stack<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stack")
            .field("children", &self.children.len())
            .finish()
    }
}

impl<M> Stack<M> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            widget_key: None,
            focus_scope: None,
        }
    }

    pub fn child(mut self, widget: impl IntoWidget<M>) -> Self {
        self.children.push(widget.into_widget());
        self
    }

    pub fn children<I>(mut self, widgets: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoWidget<M>,
    {
        self.children
            .extend(widgets.into_iter().map(IntoWidget::into_widget));
        self
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn focus_scope(mut self, scope: FocusScope) -> Self {
        self.focus_scope = Some(scope);
        self
    }

    pub fn trap_focus(self) -> Self {
        self.focus_scope(
            FocusScope::new()
                .trap_tab(true)
                .entry(crate::focus::ScopeEntry::LastFocused),
        )
    }
}

impl<M> Default for Stack<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> Widget<M> for Stack<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        ctx.record_bounds(area);

        ctx.render_children(chunk, self);
    }

    fn handle_event(&self, _event: &Event, _ctx: &mut EventCtx<M>) {}

    fn constraints(&self) -> Constraints {
        let min_width = self
            .children
            .iter()
            .map(|child| child.constraints().min_width)
            .max()
            .unwrap_or(0);
        let min_height = self
            .children
            .iter()
            .map(|child| child.constraints().min_height)
            .max()
            .unwrap_or(0);

        Constraints {
            min_width,
            max_width: None,
            min_height,
            max_height: None,
            flex: Some(1.0),
        }
    }

    fn measure(&self, proposal: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
        let mut measured = MeasuredSize::ZERO;
        for (index, child) in self.children.iter().enumerate() {
            let child_ctx = ctx.child_ctx(WidgetKey::for_child(index, child.as_ref()));
            let child_size = child.measure(proposal, &child_ctx);
            measured.width = measured.width.max(child_size.width);
            measured.height = measured.height.max(child_size.height);
        }

        self.layout_style().clamp_size(measured)
    }

    fn children(&self) -> &[Box<dyn Widget<M>>] {
        &self.children
    }

    fn focus_config(&self) -> FocusConfig {
        self.focus_scope
            .clone()
            .map(FocusConfig::Scope)
            .unwrap_or(FocusConfig::None)
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

/// Overlay renders a base widget and any number of popup layers above it.
pub struct Overlay<M = ()> {
    children: Vec<Box<dyn Widget<M>>>,
    placements: Vec<OverlayPlacement>,
    z_indices: Vec<i16>,
    widget_key: Option<String>,
    focus_scope: Option<FocusScope>,
}

impl<M> std::fmt::Debug for Overlay<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Overlay")
            .field("children", &self.children.len())
            .field("placements", &self.placements)
            .finish()
    }
}

impl<M> Overlay<M> {
    pub fn new(base: impl IntoWidget<M>) -> Self {
        Self {
            children: vec![base.into_widget()],
            placements: vec![OverlayPlacement::Fill],
            z_indices: vec![i16::MIN],
            widget_key: None,
            focus_scope: None,
        }
    }

    pub fn layer(mut self, layer: OverlayLayer<M>) -> Self {
        self.children.push(layer.widget);
        self.placements.push(layer.placement);
        self.z_indices.push(layer.z_index);
        self
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn focus_scope(mut self, scope: FocusScope) -> Self {
        self.focus_scope = Some(scope);
        self
    }

    pub fn trap_focus(self) -> Self {
        self.focus_scope(
            FocusScope::new()
                .trap_tab(true)
                .entry(crate::focus::ScopeEntry::LastFocused),
        )
    }

    fn anchor_point(area: Area, anchor: OverlayAnchor) -> (i32, i32) {
        let x0 = i32::from(area.x());
        let y0 = i32::from(area.y());
        let x1 = x0 + i32::from(area.width());
        let y1 = y0 + i32::from(area.height());
        let cx = x0 + i32::from(area.width()) / 2;
        let cy = y0 + i32::from(area.height()) / 2;

        match anchor {
            OverlayAnchor::TopLeft => (x0, y0),
            OverlayAnchor::Top => (cx, y0),
            OverlayAnchor::TopRight => (x1, y0),
            OverlayAnchor::Left => (x0, cy),
            OverlayAnchor::Center => (cx, cy),
            OverlayAnchor::Right => (x1, cy),
            OverlayAnchor::BottomLeft => (x0, y1),
            OverlayAnchor::Bottom => (cx, y1),
            OverlayAnchor::BottomRight => (x1, y1),
        }
    }

    fn anchored_area(
        anchor_area: Area,
        anchor: OverlayAnchor,
        popup_anchor: OverlayAnchor,
        width: u16,
        height: u16,
        offset_x: i16,
        offset_y: i16,
    ) -> Area {
        let (anchor_x, anchor_y) = Self::anchor_point(anchor_area, anchor);
        let popup_x = match popup_anchor {
            OverlayAnchor::TopLeft | OverlayAnchor::Left | OverlayAnchor::BottomLeft => anchor_x,
            OverlayAnchor::Top | OverlayAnchor::Center | OverlayAnchor::Bottom => {
                anchor_x - i32::from(width) / 2
            }
            OverlayAnchor::TopRight | OverlayAnchor::Right | OverlayAnchor::BottomRight => {
                anchor_x - i32::from(width)
            }
        } + i32::from(offset_x);
        let popup_y = match popup_anchor {
            OverlayAnchor::TopLeft | OverlayAnchor::Top | OverlayAnchor::TopRight => anchor_y,
            OverlayAnchor::Left | OverlayAnchor::Center | OverlayAnchor::Right => {
                anchor_y - i32::from(height) / 2
            }
            OverlayAnchor::BottomLeft | OverlayAnchor::Bottom | OverlayAnchor::BottomRight => {
                anchor_y - i32::from(height)
            }
        } + i32::from(offset_y);

        Area::new(
            (popup_x.max(0) as u16, popup_y.max(0) as u16).into(),
            (width, height).into(),
        )
    }

    fn placement_area(&self, container: Area, index: usize, ctx: &MeasureCtx) -> Area {
        let child = &self.children[index];
        let placement = self.placements[index];

        match placement {
            OverlayPlacement::Fill => container,
            OverlayPlacement::Floating {
                anchor,
                popup_anchor,
                offset_x,
                offset_y,
                width,
                height,
            } => {
                let measured =
                    measure_overlay_child(index, child.as_ref(), width, height, container, ctx);
                let width = width.unwrap_or(measured.width).min(container.width());
                let height = height.unwrap_or(measured.height).min(container.height());
                clamp_overlay_area(
                    Self::anchored_area(
                        container,
                        anchor,
                        popup_anchor,
                        width,
                        height,
                        offset_x,
                        offset_y,
                    ),
                    container,
                )
            }
            OverlayPlacement::Anchored {
                rect,
                anchor,
                popup_anchor,
                offset_x,
                offset_y,
                width,
                height,
            } => {
                let measured =
                    measure_overlay_child(index, child.as_ref(), width, height, container, ctx);
                let width = width.unwrap_or(measured.width).min(container.width());
                let height = height.unwrap_or(measured.height).min(container.height());
                clamp_overlay_area(
                    Self::anchored_area(
                        rect.as_area(container),
                        anchor,
                        popup_anchor,
                        width,
                        height,
                        offset_x,
                        offset_y,
                    ),
                    container,
                )
            }
        }
    }
}

impl<M> Widget<M> for Overlay<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 || self.children.is_empty() {
            return;
        }

        ctx.record_bounds(area);

        ctx.render_children(chunk, self);
    }

    fn layout(&self, area: Area, ctx: &MeasureCtx) -> Layout {
        if self.children.is_empty() {
            return Layout::default();
        }
        let mut indices: Vec<_> = (1..self.children.len()).collect();
        indices.sort_by_key(|i| self.z_indices[*i]);
        let mut children = vec![ChildLayout::new(0, area)];
        children.extend(indices.into_iter().filter_map(|i| {
            let layer = self.placement_area(area, i, ctx);
            layer
                .intersects(&area)
                .then_some(ChildLayout::new(i, layer))
        }));
        Layout::new(children)
    }

    fn handle_event(&self, _event: &Event, _ctx: &mut EventCtx<M>) {}

    fn constraints(&self) -> Constraints {
        self.children
            .first()
            .map(|child| child.constraints())
            .unwrap_or_else(Constraints::content)
    }

    fn measure(&self, proposal: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
        self.children
            .first()
            .map(|child| {
                let child_ctx = ctx.child_ctx(WidgetKey::for_child(0, child.as_ref()));
                child.measure(proposal, &child_ctx)
            })
            .unwrap_or(MeasuredSize::ZERO)
    }

    fn children(&self) -> &[Box<dyn Widget<M>>] {
        &self.children
    }

    fn focus_config(&self) -> FocusConfig {
        self.focus_scope
            .clone()
            .map(FocusConfig::Scope)
            .unwrap_or(FocusConfig::None)
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

/// Create a full-area stack container.
pub fn stack<M>() -> Stack<M> {
    Stack::new()
}

/// Create an overlay container with a base widget.
pub fn overlay<M>(base: impl IntoWidget<M>) -> Overlay<M> {
    Overlay::new(base)
}

fn measure_overlay_child<M>(
    index: usize,
    child: &dyn Widget<M>,
    width: Option<u16>,
    height: Option<u16>,
    container: Area,
    ctx: &MeasureCtx,
) -> MeasuredSize {
    let proposal = SizeProposal {
        width: width
            .map(AxisLimit::Exact)
            .unwrap_or(AxisLimit::AtMost(container.width())),
        height: height
            .map(AxisLimit::Exact)
            .unwrap_or(AxisLimit::AtMost(container.height())),
    };
    let child_ctx = ctx.child_ctx(WidgetKey::for_child(index, child));
    child
        .layout_style()
        .clamp_size(child.measure(proposal, &child_ctx))
}

fn clamp_overlay_area(area: Area, container: Area) -> Area {
    let max_x = container
        .x()
        .saturating_add(container.width().saturating_sub(area.width()));
    let max_y = container
        .y()
        .saturating_add(container.height().saturating_sub(area.height()));
    let x = area.x().clamp(container.x(), max_x);
    let y = area.y().clamp(container.y(), max_y);
    Area::new((x, y).into(), (area.width(), area.height()).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::AnimationStore;
    use crate::style::Theme;
    use crate::widget::{RenderCtx, WidgetPath, WidgetStore};
    use render::buffer::Buffer;
    use render::chunk::Chunk;
    use std::cell::RefCell;
    use std::collections::HashMap;

    struct MeasuredWidget {
        size: MeasuredSize,
    }

    impl Widget<()> for MeasuredWidget {
        fn render(&self, _chunk: &mut render::chunk::Chunk, _ctx: &RenderCtx) {}

        fn constraints(&self) -> Constraints {
            Constraints::min(1, 1)
        }

        fn measure(&self, _proposal: SizeProposal, _ctx: &MeasureCtx) -> MeasuredSize {
            self.size
        }
    }

    #[test]
    fn floating_overlay_uses_measured_size_when_size_is_implicit() {
        let overlay =
            overlay(crate::widgets::label::<()>("base")).layer(OverlayLayer::new(MeasuredWidget {
                size: MeasuredSize::new(8, 3),
            }));
        let geometry = render_overlay(&overlay, 20, 10);
        let layer_area = geometry
            .get(&WidgetPath::root().child(1usize))
            .copied()
            .expect("overlay layer geometry");

        assert_eq!(layer_area.width(), 8);
        assert_eq!(layer_area.height(), 3);
    }

    #[test]
    fn floating_overlay_clamps_measured_size_to_container() {
        let overlay =
            overlay(crate::widgets::label::<()>("base")).layer(OverlayLayer::new(MeasuredWidget {
                size: MeasuredSize::new(50, 20),
            }));
        let geometry = render_overlay(&overlay, 10, 5);
        let layer_area = geometry
            .get(&WidgetPath::root().child(1usize))
            .copied()
            .expect("overlay layer geometry");

        assert_eq!(layer_area, Area::new((0, 0).into(), (10, 5).into()));
    }

    fn render_overlay(widget: &Overlay<()>, width: u16, height: u16) -> HashMap<WidgetPath, Area> {
        let mut buffer = Buffer::new((width, height).into());
        let area = Area::new((0, 0).into(), (width, height).into());
        let mut chunk = Chunk::new(&mut buffer, area).unwrap();
        let store = WidgetStore::new();
        let animation_store = AnimationStore::new();
        let theme = Theme::dark();
        let geometry = RefCell::new(HashMap::<WidgetPath, Area>::new());
        let ctx = RenderCtx::new(&store, &animation_store, &theme, None, &geometry);

        widget.render(&mut chunk, &ctx);
        drop(chunk);
        geometry.into_inner()
    }
}
