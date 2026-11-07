//! A non-modal viewport and stacking host for arbitrary floating windows.
use crate::render::{area::Area, chunk::Chunk};
use crate::tui::{
    event::{Event, MouseButton, MouseEventKind},
    layout::{ChildLayout, Constraints, Layout},
    style::Padding,
    widget::{
        EventCtx, EventPhase, IntoWidget, Lifecycle, MeasureCtx, RenderCtx, StateCtx, Widget,
        WidgetKey,
    },
    widgets::layout::floating_window::{
        FloatingWindow, FloatingWindowState, WindowProjection, WindowVisibility,
    },
};
/// Persistent active-window and stacking state stored under the host's widget identity.
///
/// The default has no active window and an empty ordering.
#[derive(Debug, Clone, Default)]
pub struct WindowHostState {
    /// Stable key of the active window, or `None` when the base is active.
    pub active_window: Option<String>,
    /// Back to front logical ids. Reordering preserves keyed child identities.
    pub order: Vec<String>,
    authoritative_active: Option<Option<String>>,
}
struct WindowMeta {
    id: String,
    min: (u16, u16),
    max: (u16, u16),
    projection: WindowProjection,
}
/// Non-modal host that constrains floating windows to a shared viewport.
///
/// The base fills the allocation. Windows use the padded viewport, paint
/// above the base, and retain identity through their unique keys. The active
/// window paints last. Pointer presses and keyboard focus update activation.
///
/// Create a host with [`window_host`] and add windows with [`Self::window`].
pub struct WindowHost<M = ()> {
    children: Vec<Box<dyn Widget<M>>>,
    windows: Vec<WindowMeta>,
    key: Option<String>,
    active: Option<Option<String>>,
    default_active: Option<String>,
    padding: Padding,
    snap: u16,
    on_active: Option<Box<dyn Fn(Option<String>) -> M>>,
}
/// Creates a host around a base widget, with no windows, padding, or edge snapping.
///
/// # Examples
///
/// ```
/// use rsille::tui::{layout::window_host, widgets::{floating_window, label}};
///
/// let view = window_host(label::<()>("Desktop"))
///     .key("desktop")
///     .window(floating_window(label("Notes")).key("notes"))
///     .default_active_window(Some("notes".into()))
///     .snap_distance(2);
/// ```
pub fn window_host<M>(base: impl IntoWidget<M>) -> WindowHost<M> {
    WindowHost {
        children: vec![base.into_widget()],
        windows: vec![],
        key: None,
        active: None,
        default_active: None,
        padding: Padding::ZERO,
        snap: 0,
        on_active: None,
    }
}
impl<M: 'static> WindowHost<M> {
    /// Sets a stable key for widget identity and persistent state.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }
    /// Appends a floating window to the host.
    ///
    /// Its stable key is also its logical ID for activation and stacking.
    ///
    /// # Panics
    ///
    /// Panics if the window has no key or another window already uses that key.
    pub fn window(mut self, window: FloatingWindow<M>) -> Self {
        let id = Widget::key(&window)
            .expect("FloatingWindow in WindowHost requires a stable key")
            .to_owned();
        assert!(
            !self.windows.iter().any(|w| w.id == id),
            "Duplicate floating window key {id}"
        );
        window.projection.snap.set(self.snap);
        self.windows.push(WindowMeta {
            id,
            min: window.min,
            max: window.max,
            projection: window.projection.clone(),
        });
        self.children.push(Box::new(window));
        self
    }
    /// Sets the authoritative active-window key for this view.
    ///
    /// `None` explicitly selects no window; omitting this method leaves
    /// activation managed by host state. Pair with [`Self::on_active_window_change`]
    /// to keep application state synchronized with interactions.
    pub fn active_window(mut self, id: Option<String>) -> Self {
        self.active = Some(id);
        self
    }
    /// Sets the initial active-window key, applied only when the host mounts.
    ///
    /// An explicit [`Self::active_window`] value takes precedence.
    pub fn default_active_window(mut self, id: Option<String>) -> Self {
        self.default_active = id;
        self
    }
    /// Emits a notification when interaction changes the active window.
    ///
    /// The argument is the window's stable key, or `None` for the base. Use the
    /// message to update application state when controlling activation.
    pub fn on_active_window_change(
        mut self,
        handler: impl Fn(Option<String>) -> M + 'static,
    ) -> Self {
        self.on_active = Some(Box::new(handler));
        self
    }
    /// Reserves viewport space in cells for application-composed docks or toolbars.
    ///
    /// Only windows use the inset viewport; the base still fills the host area.
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }
    /// Sets the edge snapping distance in cells, for all existing and future windows.
    ///
    /// Snapping occurs on release; zero (the default) disables it.
    pub fn snap_distance(mut self, distance: u16) -> Self {
        self.snap = distance;
        for w in &self.windows {
            w.projection.snap.set(distance);
        }
        self
    }
    fn ordered(&self, state: Option<&WindowHostState>) -> Vec<usize> {
        let mut indices = (0..self.windows.len()).collect::<Vec<_>>();
        if let Some(state) = state {
            indices.sort_by_key(|i| {
                (
                    state.active_window.as_ref() == Some(&self.windows[*i].id),
                    state.order.iter().position(|id| id == &self.windows[*i].id),
                )
            });
        }
        indices
    }
}
impl<M: 'static> Widget<M> for WindowHost<M> {
    fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
    fn children(&self) -> &[Box<dyn Widget<M>>] {
        &self.children
    }
    fn constraints(&self) -> Constraints {
        self.children[0].constraints()
    }
    fn lifecycle(&self, ctx: &mut StateCtx) {
        if ctx.lifecycle() == Lifecycle::Unmount {
            return;
        }
        let mount = ctx.lifecycle() == Lifecycle::Mount;
        let state = ctx.state_mut::<WindowHostState>();
        let before = state.active_window.clone();
        if mount {
            state.active_window = self.default_active.clone();
        }
        if let Some(active) = &self.active {
            state.active_window = active.clone();
        } else if let Some(active) = state.authoritative_active.take() {
            state.active_window = active;
        }
        state.authoritative_active = self.active.clone();
        state
            .order
            .retain(|id| self.windows.iter().any(|w| &w.id == id));
        for window in &self.windows {
            if !state.order.contains(&window.id) {
                state.order.push(window.id.clone());
            }
        }
        if before != state.active_window {
            ctx.request_layout();
        }
    }
    fn layout(&self, area: Area, ctx: &MeasureCtx) -> Layout {
        let viewport = area.shrink_saturating(
            self.padding.top,
            self.padding.bottom,
            self.padding.left,
            self.padding.right,
        );
        let mut allocations = vec![ChildLayout::new(0, area)];
        for i in self.ordered(ctx.state::<WindowHostState>()) {
            let window = &self.windows[i];
            window.projection.host.set(viewport);
            let child = self.children[i + 1].as_ref();
            let child_ctx = ctx.child_ctx(WidgetKey::for_child(i + 1, child));
            let Some(state) = child_ctx.state::<FloatingWindowState>() else {
                continue;
            };
            if state.visibility == WindowVisibility::Hidden {
                continue;
            }
            let rect = state.rect.constrained(
                (viewport.width(), viewport.height()),
                window.min,
                window.max,
            );
            window
                .projection
                .compact
                .set(rect.height < 5 || rect.width < 3);
            let height = if state.visibility == WindowVisibility::Collapsed {
                rect.height.min(3)
            } else {
                rect.height
            };
            allocations.push(ChildLayout::new(
                i + 1,
                Area::new(
                    (
                        viewport.x().saturating_add(rect.x as u16),
                        viewport.y().saturating_add(rect.y as u16),
                    )
                        .into(),
                    (rect.width, height).into(),
                ),
            ));
        }
        Layout::new(allocations)
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let active = ctx
            .state::<WindowHostState>()
            .and_then(|s| s.active_window.as_ref());
        for window in &self.windows {
            window.projection.active.set(active == Some(&window.id));
        }
        ctx.render_children(chunk, self);
    }
    fn focus_enter(&self, ctx: &mut EventCtx<M>) {
        self.handle_event(&Event::FocusGained, ctx);
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if ctx.phase() == EventPhase::Bubble {
            return;
        }
        let interested = matches!(event, Event::FocusGained | Event::Key(_))
            || matches!(event,Event::Mouse(m) if m.kind==MouseEventKind::Down(MouseButton::Left));
        if !interested {
            return;
        }
        let path = if matches!(event, Event::Mouse(_)) {
            ctx.target_path()
        } else {
            ctx.focused_path()
        };
        let next = path.and_then(|path| {
            self.windows
                .iter()
                .enumerate()
                .find(|(i, _)| {
                    path.starts_with(
                        &ctx.path()
                            .child(WidgetKey::for_child(i + 1, self.children[i + 1].as_ref())),
                    )
                })
                .map(|(_, w)| w.id.clone())
        });
        let state = ctx.state_mut::<WindowHostState>();
        if state.active_window != next {
            state.active_window = next.clone();
            if let Some(id) = &next {
                state.order.retain(|w| w != id);
                state.order.push(id.clone());
            }
            ctx.request_layout();
            if let Some(handler) = &self.on_active {
                ctx.emit_notification(handler(next));
            }
        }
    }
}
