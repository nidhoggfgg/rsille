//! Root-hosted floating surfaces anchored to displayed widget identities.
use crate::render::{area::Area, chunk::Chunk};
use crate::tui::{
    event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind},
    focus::{FocusConfig, FocusScope},
    layout::{ChildLayout, Constraints, Layout, MeasuredSize, SizeProposal},
    widget::{
        EventCtx, EventPhase, IntoWidget, MeasureCtx, RenderCtx, Widget, WidgetId, WidgetKey,
    },
};

/// Put a Portal at the viewport root. Layer content can then cross the anchor's
/// container/scroll clipping boundaries, while retaining ordinary widget identity.
pub struct Portal<M = ()> {
    children: Vec<Box<dyn Widget<M>>>,
}
impl<M> Portal<M> {
    pub fn new(base: impl IntoWidget<M>) -> Self {
        Self {
            children: vec![base.into_widget()],
        }
    }
    /// Layers paint in insertion order. The last modal layer owns input.
    pub fn layer(mut self, layer: PortalLayer<M>) -> Self
    where
        M: 'static,
    {
        self.children.push(Box::new(layer));
        self
    }
}
impl<M> Widget<M> for Portal<M> {
    fn constraints(&self) -> Constraints {
        self.children[0].constraints()
    }
    fn measure(&self, proposal: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
        let child = self.children[0].as_ref();
        child.measure(proposal, &ctx.child_ctx(WidgetKey::for_child(0, child)))
    }
    fn children(&self) -> &[Box<dyn Widget<M>>] {
        &self.children
    }
    fn layout(&self, area: Area, _: &MeasureCtx) -> Layout {
        Layout::new([ChildLayout::new(0, area)])
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.render_children(chunk, self);
        for (i, layer) in self.children.iter().enumerate().skip(1) {
            let child_ctx = ctx.child_ctx(WidgetKey::for_child(i, layer.as_ref()));
            child_ctx.record_bounds(chunk.area());
            layer.render(chunk, &child_ctx);
        }
    }
}

/// A floating surface. By default it is centered. An anchored surface prefers
/// below its anchor, flips above if it fits, then clamps to the host viewport.
/// A missing/offscreen anchor falls back to centered placement.
pub struct PortalLayer<M = ()> {
    children: Vec<Box<dyn Widget<M>>>,
    anchor: Option<WidgetId>,
    size: Option<(u16, u16)>,
    modal: bool,
    on_dismiss: Option<Box<dyn Fn() -> M>>,
    key: Option<String>,
}
impl<M> PortalLayer<M> {
    pub fn new(content: impl IntoWidget<M>) -> Self {
        Self {
            children: vec![content.into_widget()],
            anchor: None,
            size: None,
            modal: false,
            on_dismiss: None,
            key: None,
        }
    }
    pub fn anchor(mut self, id: WidgetId) -> Self {
        self.anchor = Some(id);
        self
    }
    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.size = Some((width, height));
        self
    }
    pub fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }
    /// Escape and a left click outside the content request dismissal. The click
    /// is consumed; it never activates the background control beneath it.
    pub fn on_dismiss(mut self, handler: impl Fn() -> M + 'static) -> Self {
        self.on_dismiss = Some(Box::new(handler));
        self
    }
}
impl<M> Widget<M> for PortalLayer<M> {
    fn constraints(&self) -> Constraints {
        Constraints::content()
    }
    fn children(&self) -> &[Box<dyn Widget<M>>] {
        &self.children
    }
    fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
    fn is_modal(&self) -> bool {
        self.modal
    }
    fn focus_config(&self) -> FocusConfig {
        if self.modal {
            FocusConfig::Scope(FocusScope::new().trap_tab(true))
        } else {
            FocusConfig::None
        }
    }
    // Placement uses this frame's displayed/clipped anchor, after the base paints.
    // The actual child allocation still passes through the standard layout cache.
    fn layout(&self, host: Area, ctx: &MeasureCtx) -> Layout {
        let child = self.children[0].as_ref();
        let measured = child.measure(
            SizeProposal::at_most(host.width(), host.height()),
            &ctx.child_ctx(WidgetKey::for_child(0, child)),
        );
        let (width, height) = self.size.unwrap_or((measured.width, measured.height));
        Layout::default().with_data((width.min(host.width()), height.min(host.height())))
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let host = chunk.area();
        if host.width() == 0 || host.height() == 0 {
            return;
        }
        if self.modal || self.on_dismiss.is_some() {
            ctx.record_hit_bounds(host, host, crate::tui::animation::HitTestMode::Display);
        }
        let child = self.children[0].as_ref();
        let child_key = WidgetKey::for_child(0, child);
        let layout = ctx.layout(self, host);
        let &(w, h) = layout.data::<(u16, u16)>().expect("portal dimensions");
        let anchor = self.anchor.as_ref().and_then(|id| ctx.bounds_for_id(id));
        let (x, y) = if let Some(anchor) = anchor {
            let below = anchor.y().saturating_add(anchor.height());
            let y = if u32::from(below) + u32::from(h)
                > u32::from(host.y()) + u32::from(host.height())
                && anchor.y().saturating_sub(host.y()) >= h
            {
                anchor.y() - h
            } else {
                below
            };
            (anchor.x(), y)
        } else {
            (
                host.x() + (host.width() - w) / 2,
                host.y() + (host.height() - h) / 2,
            )
        };
        let x = x.clamp(host.x(), host.x().saturating_add(host.width() - w));
        let y = y.clamp(host.y(), host.y().saturating_add(host.height() - h));
        let allocation = Area::new((x, y).into(), (w, h).into());
        ctx.child_ctx(child_key.clone()).layout(child, allocation);
        ctx.render_child_at(chunk, child_key, child, allocation);
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        let Some(handler) = &self.on_dismiss else {
            return;
        };
        if ctx.was_handled() {
            return;
        }
        let dismiss = match event {
            Event::Key(key) => {
                key.kind == KeyEventKind::Press
                    && key.code == KeyCode::Esc
                    && ctx.phase() != EventPhase::Capture
            }
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
                let path = ctx
                    .path()
                    .child(WidgetKey::for_child(0, self.children[0].as_ref()));
                !ctx.bounds_for(&path).is_some_and(|area| {
                    mouse.column >= area.x()
                        && mouse.row >= area.y()
                        && u32::from(mouse.column) < u32::from(area.x()) + u32::from(area.width())
                        && u32::from(mouse.row) < u32::from(area.y()) + u32::from(area.height())
                })
            }
            _ => false,
        };
        if dismiss {
            ctx.emit(handler());
            ctx.set_handled();
            ctx.stop_propagation();
        }
    }
}
pub fn portal<M>(base: impl IntoWidget<M>) -> Portal<M> {
    Portal::new(base)
}
