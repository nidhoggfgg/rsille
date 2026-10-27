//! A composable pointer handle. Child-handled inputs never start a parent drag.
use crate::render::{area::Area, chunk::Chunk};
use crate::tui::{
    event::{Event, MouseButton, MouseEventKind},
    gesture::{DragAxis, DragCancelReason, DragEvent, DragState},
    layout::{Constraints, Layout, LayoutStyle, MeasuredSize, SizeProposal},
    widget::{EventCtx, EventPhase, IntoWidget, MeasureCtx, RenderCtx, Widget},
};
pub struct DragRegion<M = ()> {
    children: Vec<Box<dyn Widget<M>>>,
    key: Option<String>,
    axis: DragAxis,
    threshold: u16,
    disabled: bool,
    on_drag: Option<Box<dyn Fn(DragEvent) -> M>>,
}
pub fn drag_region<M>(content: impl IntoWidget<M>) -> DragRegion<M> {
    DragRegion {
        children: vec![content.into_widget()],
        key: None,
        axis: DragAxis::Both,
        threshold: 1,
        disabled: false,
        on_drag: None,
    }
}
impl<M> DragRegion<M> {
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }
    pub fn axis(mut self, axis: DragAxis) -> Self {
        self.axis = axis;
        self
    }
    pub fn threshold_cells(mut self, threshold: u16) -> Self {
        self.threshold = threshold;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn on_drag(mut self, handler: impl Fn(DragEvent) -> M + 'static) -> Self {
        self.on_drag = Some(Box::new(handler));
        self
    }
    fn emit(&self, events: impl IntoIterator<Item = DragEvent>, ctx: &mut EventCtx<M>) {
        for event in events {
            if let Some(handler) = &self.on_drag {
                ctx.emit(handler(event));
            }
        }
    }
}
impl<M> Widget<M> for DragRegion<M> {
    fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
    fn children(&self) -> &[Box<dyn Widget<M>>] {
        &self.children
    }
    fn constraints(&self) -> Constraints {
        self.children[0].constraints()
    }
    fn layout_style(&self) -> LayoutStyle {
        self.children[0].layout_style()
    }
    fn measure(&self, proposal: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
        self.children[0].measure(
            proposal,
            &ctx.child_ctx(crate::tui::widget::WidgetKey::for_child(
                0,
                self.children[0].as_ref(),
            )),
        )
    }
    fn layout(&self, area: Area, _: &MeasureCtx) -> Layout {
        Layout::new([crate::tui::layout::ChildLayout::new(0, area)])
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.render_children(chunk, self);
    }
    fn pointer_enabled(&self) -> bool {
        !self.disabled
    }
    fn pointer_cancel(&self, reason: DragCancelReason, ctx: &mut EventCtx<M>) {
        let active = ctx.state_mut::<DragState>().is_active();
        let event = ctx.state_mut::<DragState>().cancel(reason);
        if active && reason == DragCancelReason::Escape {
            ctx.set_handled();
        }
        self.emit(event, ctx);
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if self.disabled || ctx.phase() == EventPhase::Capture || ctx.was_handled() {
            return;
        }
        let Event::Mouse(mouse) = event else {
            return;
        };
        let Some(position) = ctx.cell_mouse_position(event) else {
            return;
        };
        let events = match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                ctx.capture_pointer();
                ctx.state_mut::<DragState>()
                    .begin(position, mouse.modifiers, self.threshold)
            }
            MouseEventKind::Drag(MouseButton::Left)
                if ctx.state_mut::<DragState>().is_pending() =>
            {
                ctx.state_mut::<DragState>().motion(
                    position,
                    mouse.modifiers,
                    self.threshold,
                    self.axis,
                )
            }
            MouseEventKind::Up(MouseButton::Left) if ctx.state_mut::<DragState>().is_pending() => {
                ctx.release_pointer();
                ctx.state_mut::<DragState>()
                    .end(position, mouse.modifiers, self.axis)
                    .into_iter()
                    .collect()
            }
            _ => return,
        };
        ctx.set_handled();
        self.emit(events, ctx);
    }
}
