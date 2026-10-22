use std::rc::Rc;

use super::*;

/// Message conversion and lifetime namespaces for any widget description.
pub trait WidgetExt<M>: Widget<M> + Sized + 'static {
    /// Convert all messages in the subtree without adding a layout/identity node.
    fn map<N: 'static>(self, map: impl Fn(M) -> N + 'static) -> MappedWidget<M, N>
    where
        M: 'static,
    {
        MappedWidget::build(Rc::new(self), Vec::new(), Rc::new(map))
    }

    /// A named identity boundary and owner for scoped effects/subscriptions.
    fn scope(self, name: impl Into<String>) -> ScopedWidget<M> {
        ScopedWidget {
            name: name.into(),
            children: vec![Box::new(self)],
        }
    }
}

impl<M, W: Widget<M> + Sized + 'static> WidgetExt<M> for W {}

type MessageMap<M, N> = Rc<dyn Fn(M) -> N>;

/// Transparent adapter. Nodes share ownership of the original immutable tree;
/// each exposes the corresponding mapped children for framework traversal.
pub struct MappedWidget<M, N> {
    root: Rc<dyn Widget<M>>,
    route: Vec<(bool, usize)>,
    map: MessageMap<M, N>,
    children: Vec<Box<dyn Widget<N>>>,
    retained: Vec<Box<dyn Widget<N>>>,
}

impl<M: 'static, N: 'static> MappedWidget<M, N> {
    fn build(root: Rc<dyn Widget<M>>, route: Vec<(bool, usize)>, map: MessageMap<M, N>) -> Self {
        let mut result = Self {
            root,
            route,
            map,
            children: Vec::new(),
            retained: Vec::new(),
        };
        for retained in [false, true] {
            let count = if retained {
                result.inner().retained_children().len()
            } else {
                result.inner().children().len()
            };
            for index in 0..count {
                let mut route = result.route.clone();
                route.push((retained, index));
                let child = Box::new(Self::build(result.root.clone(), route, result.map.clone()));
                if retained {
                    result.retained.push(child);
                } else {
                    result.children.push(child);
                }
            }
        }
        result
    }

    fn inner(&self) -> &(dyn Widget<M> + 'static) {
        let mut widget = self.root.as_ref();
        for &(retained, index) in &self.route {
            widget = if retained {
                widget.retained_children()[index].as_ref()
            } else {
                widget.children()[index].as_ref()
            };
        }
        widget
    }
}

impl<M: 'static, N: 'static> Widget<N> for MappedWidget<M, N> {
    fn widget_type(&self) -> TypeId {
        self.inner().widget_type()
    }
    fn lifecycle(&self, ctx: &mut StateCtx) {
        self.inner().lifecycle(ctx);
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        self.inner().render(chunk, ctx);
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<N>) {
        ctx.map_messages(
            |child| self.inner().handle_event(event, child),
            |m| (self.map)(m),
        );
    }
    fn animate(&self, ctx: &mut AnimationCtx) -> bool {
        self.inner().animate(ctx)
    }
    fn presence(&self) -> Option<&Presence> {
        self.inner().presence()
    }
    fn layout_transition(&self) -> Option<LayoutTransition> {
        self.inner().layout_transition()
    }
    fn shared_transition(&self) -> Option<&SharedTransition> {
        self.inner().shared_transition()
    }
    fn hit_test_mode(&self) -> HitTestMode {
        self.inner().hit_test_mode()
    }
    fn constraints(&self) -> Constraints {
        self.inner().constraints()
    }
    fn layout_style(&self) -> LayoutStyle {
        self.inner().layout_style()
    }
    fn layout_style_with(&self, ctx: &MeasureCtx) -> LayoutStyle {
        self.inner().layout_style_with(ctx)
    }
    fn measure(&self, proposal: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
        self.inner().measure(proposal, ctx)
    }
    fn focus_config(&self) -> FocusConfig {
        self.inner().focus_config()
    }
    fn children(&self) -> &[Box<dyn Widget<N>>] {
        &self.children
    }
    fn retained_children(&self) -> &[Box<dyn Widget<N>>] {
        &self.retained
    }
    fn key(&self) -> Option<&str> {
        self.inner().key()
    }
    fn resource_scope(&self) -> Option<&str> {
        self.inner().resource_scope()
    }
}

/// A scope adds one named container without changing its child's layout intent.
pub struct ScopedWidget<M> {
    name: String,
    children: Vec<Box<dyn Widget<M>>>,
}

impl<M> Widget<M> for ScopedWidget<M> {
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.record_bounds(chunk.area());
        let child = self.children[0].as_ref();
        ctx.render_child_at(chunk, WidgetKey::for_child(0, child), child, chunk.area());
    }
    fn constraints(&self) -> Constraints {
        self.children[0].constraints()
    }
    fn layout_style(&self) -> LayoutStyle {
        self.children[0].layout_style()
    }
    fn layout_style_with(&self, ctx: &MeasureCtx) -> LayoutStyle {
        self.children[0]
            .layout_style_with(&ctx.child_ctx(WidgetKey::for_child(0, self.children[0].as_ref())))
    }
    fn measure(&self, proposal: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
        self.children[0].measure(
            proposal,
            &ctx.child_ctx(WidgetKey::for_child(0, self.children[0].as_ref())),
        )
    }
    fn children(&self) -> &[Box<dyn Widget<M>>] {
        &self.children
    }
    fn key(&self) -> Option<&str> {
        Some(&self.name)
    }
    fn resource_scope(&self) -> Option<&str> {
        Some(&self.name)
    }
}
