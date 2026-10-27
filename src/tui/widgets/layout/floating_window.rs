//! Non-modal floating windows. Hidden and collapsed content remains mounted,
//! using the same scopes and retained-subtree lifecycle as other containers.
use crate::render::{area::Area, chunk::Chunk};
use crate::tui::{
    animation::{AnimationSpec, Easing, HitTestMode, LayoutTransition},
    event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind},
    focus::{FocusConfig, FocusScope, ScopeEntry},
    gesture::{
        CellPoint, DragAxis, DragCancelReason, DragEvent, DragState, ResizeEdge, WindowRect,
    },
    layout::{border_renderer, ChildLayout, Constraints, Layout},
    style::{BorderStyle, Style},
    widget::{
        EventCtx, EventPhase, IntoWidget, Lifecycle, MeasureCtx, RenderCtx, StateCtx, Widget,
        WidgetKey,
    },
};
use std::{cell::Cell, rc::Rc, time::Duration};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WindowVisibility {
    #[default]
    Expanded,
    Collapsed,
    Hidden,
}
#[derive(Debug, Clone, Default)]
pub struct FloatingWindowState {
    pub rect: WindowRect,
    pub visibility: WindowVisibility,
    drag: DragState,
    start_rect: WindowRect,
    start_host: Option<Area>,
    resizing: bool,
    authoritative_rect: Option<WindowRect>,
    authoritative_visibility: Option<WindowVisibility>,
}

/// Cached projections of persistent state for tree traversal and window chrome.
/// WidgetStore remains authoritative; these cells never survive descriptions.
#[derive(Clone)]
pub(crate) struct WindowProjection {
    visibility: Rc<Cell<WindowVisibility>>,
    dragging: Rc<Cell<bool>>,
    pub(crate) compact: Rc<Cell<bool>>,
    pub active: Rc<Cell<bool>>,
    pub host: Rc<Cell<Area>>,
    pub snap: Rc<Cell<u16>>,
}
impl Default for WindowProjection {
    fn default() -> Self {
        Self {
            compact: Rc::new(Cell::new(false)),
            visibility: Rc::new(Cell::new(WindowVisibility::Expanded)),
            dragging: Rc::new(Cell::new(false)),
            active: Rc::new(Cell::new(false)),
            host: Rc::new(Cell::new(Area::new((0, 0).into(), (0, 0).into()))),
            snap: Rc::new(Cell::new(0)),
        }
    }
}

pub struct FloatingWindow<M = ()> {
    children: Vec<Box<dyn Widget<M>>>,
    key: Option<String>,
    rect: Option<WindowRect>,
    default_rect: WindowRect,
    visibility: Option<WindowVisibility>,
    default_visibility: WindowVisibility,
    pub(crate) min: (u16, u16),
    pub(crate) max: (u16, u16),
    movable: bool,
    resizable: bool,
    collapsible: bool,
    hideable: bool,
    border: BorderStyle,
    style: Option<Style>,
    active_style: Option<Style>,
    transition: LayoutTransition,
    pub(crate) projection: WindowProjection,
    title: String,
    title_actions: Rc<Vec<Box<dyn Widget<M>>>>,
    actions_width: u16,
    on_rect: Option<Box<dyn Fn(WindowRect) -> M>>,
    on_visibility: Option<Box<dyn Fn(WindowVisibility) -> M>>,
    on_close: Option<Box<dyn Fn() -> M>>,
}

pub fn floating_window<M: 'static>(content: impl IntoWidget<M>) -> FloatingWindow<M> {
    let projection = WindowProjection::default();
    FloatingWindow {
        children: vec![
            Box::new(Chrome::<M> {
                actions: Rc::new(vec![]),
                actions_width: 0,
                key: "window-title".into(),
                text: String::new(),
                grip: false,
                controls: TitleControls {
                    collapsible: true,
                    hideable: false,
                    closable: false,
                },
                projection: projection.clone(),
            }),
            Box::new(Content {
                children: vec![content.into_widget()],
            }),
            Box::new(Chrome::<M> {
                actions: Rc::new(vec![]),
                actions_width: 0,
                key: "window-resize".into(),
                text: "drag title · resize ↘".into(),
                grip: true,
                controls: TitleControls::default(),
                projection: projection.clone(),
            }),
        ],
        key: None,
        rect: None,
        default_rect: WindowRect::default(),
        visibility: None,
        default_visibility: WindowVisibility::Expanded,
        min: (12, 5),
        max: (u16::MAX, u16::MAX),
        movable: true,
        resizable: true,
        collapsible: true,
        hideable: false,
        border: BorderStyle::Rounded,
        style: None,
        active_style: None,
        transition: LayoutTransition::size_and_position(AnimationSpec::new(
            Duration::from_millis(180),
            Easing::EaseOut,
        )),
        projection,
        title: String::new(),
        title_actions: Rc::new(vec![]),
        actions_width: 0,
        on_rect: None,
        on_visibility: None,
        on_close: None,
    }
}
impl<M: 'static> FloatingWindow<M> {
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self.refresh_title();
        self
    }
    fn refresh_title(&mut self) {
        self.children[0] = Box::new(Chrome::<M> {
            actions: self.title_actions.clone(),
            actions_width: self.actions_width,
            key: self.children[0].key().unwrap_or("window-title").into(),
            text: self.title.clone(),
            grip: false,
            controls: TitleControls {
                collapsible: self.collapsible,
                hideable: self.hideable,
                closable: self.on_close.is_some(),
            },
            projection: self.projection.clone(),
        });
    }
    pub fn title_key(mut self, key: impl Into<String>) -> Self {
        self.children[0] = Box::new(Chrome::<M> {
            actions: self.title_actions.clone(),
            actions_width: self.actions_width,
            key: key.into(),
            text: self.title.clone(),
            grip: false,
            controls: TitleControls {
                collapsible: self.collapsible,
                hideable: self.hideable,
                closable: self.on_close.is_some(),
            },
            projection: self.projection.clone(),
        });
        self
    }
    /// Replace the built-in title buttons with ordinary application controls.
    /// Action children keep their own focus, clicks, and pointer capture.
    pub fn title_actions(mut self, width: u16, actions: impl IntoWidget<M>) -> Self {
        self.title_actions = Rc::new(vec![actions.into_widget()]);
        self.actions_width = width;
        self.refresh_title();
        self
    }
    pub fn resize_key(mut self, key: impl Into<String>) -> Self {
        self.children[2] = Box::new(Chrome::<M> {
            actions: Rc::new(vec![]),
            actions_width: 0,
            key: key.into(),
            text: "drag title · resize ↘".into(),
            grip: true,
            controls: TitleControls::default(),
            projection: self.projection.clone(),
        });
        self
    }
    pub fn rect(mut self, rect: WindowRect) -> Self {
        self.rect = Some(rect);
        self
    }
    pub fn default_rect(mut self, rect: WindowRect) -> Self {
        self.default_rect = rect;
        self
    }
    pub fn visibility(mut self, value: WindowVisibility) -> Self {
        self.visibility = Some(value);
        self
    }
    pub fn default_visibility(mut self, value: WindowVisibility) -> Self {
        self.default_visibility = value;
        self
    }
    pub fn min_size(mut self, w: u16, h: u16) -> Self {
        self.min = (w, h);
        self
    }
    pub fn max_size(mut self, w: u16, h: u16) -> Self {
        self.max = (w, h);
        self
    }
    pub fn movable(mut self, value: bool) -> Self {
        self.movable = value;
        self
    }
    pub fn resizable(mut self, value: bool) -> Self {
        self.resizable = value;
        self
    }
    pub fn collapsible(mut self, value: bool) -> Self {
        self.collapsible = value;
        self.refresh_title();
        self
    }
    pub fn hideable(mut self, value: bool) -> Self {
        self.hideable = value;
        self.refresh_title();
        self
    }
    pub fn border(mut self, value: BorderStyle) -> Self {
        self.border = value;
        self
    }
    pub fn style(mut self, value: Style) -> Self {
        self.style = Some(value);
        self
    }
    pub fn active_style(mut self, value: Style) -> Self {
        self.active_style = Some(value);
        self
    }
    pub fn layout_transition(mut self, value: LayoutTransition) -> Self {
        self.transition = value;
        self
    }
    pub fn on_rect_change(mut self, handler: impl Fn(WindowRect) -> M + 'static) -> Self {
        self.on_rect = Some(Box::new(handler));
        self
    }
    pub fn on_visibility_change(
        mut self,
        handler: impl Fn(WindowVisibility) -> M + 'static,
    ) -> Self {
        self.on_visibility = Some(Box::new(handler));
        self
    }
    pub fn on_close(mut self, handler: impl Fn() -> M + 'static) -> Self {
        self.on_close = Some(Box::new(handler));
        self.refresh_title();
        self
    }
    fn constrain(&self, rect: WindowRect) -> WindowRect {
        let host = self.projection.host.get();
        rect.constrained((host.width(), host.height()), self.min, self.max)
    }
    fn propose_rect(&self, rect: WindowRect, ctx: &mut EventCtx<M>) {
        let rect = self.constrain(rect);
        if ctx.state_mut::<FloatingWindowState>().rect != rect {
            ctx.state_mut::<FloatingWindowState>().rect = rect;
            ctx.request_layout();
            if let Some(handler) = &self.on_rect {
                ctx.emit(handler(rect));
            }
        }
    }
    fn propose_visibility(&self, visibility: WindowVisibility, ctx: &mut EventCtx<M>) {
        ctx.state_mut::<FloatingWindowState>().visibility = visibility;
        self.projection.visibility.set(visibility);
        ctx.request_layout();
        if let Some(handler) = &self.on_visibility {
            ctx.emit(handler(visibility));
        }
        if visibility == WindowVisibility::Collapsed {
            ctx.request_focus(
                ctx.path()
                    .child(WidgetKey::for_child(0, self.children[0].as_ref())),
            );
        }
    }
    fn apply_drag(&self, events: impl IntoIterator<Item = DragEvent>, ctx: &mut EventCtx<M>) {
        for event in events {
            let state = ctx.state_mut::<FloatingWindowState>().clone();
            match event {
                DragEvent::Update(update) | DragEvent::End(update) => {
                    let mut rect = if state.resizing {
                        state
                            .start_rect
                            .resized(update.total_delta, ResizeEdge::BottomRight)
                    } else {
                        state.start_rect.translated(update.total_delta)
                    };
                    if matches!(event, DragEvent::End(_)) {
                        self.projection.dragging.set(false);
                        if !state.resizing && self.projection.snap.get() > 0 {
                            let host = self.projection.host.get();
                            rect = self
                                .constrain(rect)
                                .snapped((host.width(), host.height()), self.projection.snap.get());
                        }
                    }
                    self.propose_rect(rect, ctx);
                }
                DragEvent::Cancel(DragCancelReason::Escape) => {
                    self.propose_rect(state.start_rect, ctx)
                }
                _ => {}
            }
        }
    }
}
impl<M: 'static> Widget<M> for FloatingWindow<M> {
    fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
    fn lifecycle(&self, ctx: &mut StateCtx) {
        if ctx.lifecycle() == Lifecycle::Unmount {
            return;
        }
        let mount = ctx.lifecycle() == Lifecycle::Mount;
        let state = ctx.state_mut::<FloatingWindowState>();
        let previous = (state.rect, state.visibility);
        if mount {
            state.rect = self.default_rect;
            state.visibility = self.default_visibility;
        }
        let changed_rect =
            self.rect.is_some_and(|rect| rect != state.rect) && state.drag.is_pending();
        if let Some(rect) = self.rect {
            state.rect = rect;
        } else if let Some(rect) = state.authoritative_rect.take() {
            state.rect = rect;
        }
        if let Some(value) = self.visibility {
            state.visibility = value;
        } else if let Some(value) = state.authoritative_visibility.take() {
            state.visibility = value;
        }
        state.authoritative_rect = self.rect;
        state.authoritative_visibility = self.visibility;
        self.projection.visibility.set(state.visibility);
        self.projection.dragging.set(state.drag.is_pending());
        let changed_visibility = previous.1 != state.visibility && state.drag.is_pending();
        let changed = previous != (state.rect, state.visibility);
        if changed_visibility {
            ctx.cancel_pointer(DragCancelReason::Hidden);
        }
        if changed {
            ctx.request_layout();
        }
        if changed_rect {
            ctx.cancel_pointer(DragCancelReason::ControlledValueChanged);
        }
    }
    fn constraints(&self) -> Constraints {
        Constraints::min(1, 1)
    }
    fn children(&self) -> &[Box<dyn Widget<M>>] {
        let count = match self.projection.visibility.get() {
            WindowVisibility::Expanded if !self.projection.compact.get() => {
                if self.resizable {
                    3
                } else {
                    2
                }
            }
            WindowVisibility::Expanded | WindowVisibility::Collapsed => 1,
            WindowVisibility::Hidden => 0,
        };
        &self.children[..count]
    }
    fn retained_children(&self) -> &[Box<dyn Widget<M>>] {
        &self.children[self.children().len()..]
    }
    fn focus_config(&self) -> FocusConfig {
        FocusConfig::Scope(FocusScope::new().entry(ScopeEntry::LastFocused))
    }
    fn layout_transition(&self) -> Option<LayoutTransition> {
        Some(
            (if self.projection.dragging.get() {
                LayoutTransition::none()
            } else {
                self.transition
            })
            .hit_test(HitTestMode::Display),
        )
    }
    fn hit_test_mode(&self) -> HitTestMode {
        HitTestMode::Display
    }
    fn layout(&self, area: Area, _: &MeasureCtx) -> Layout {
        if area.width() < 3 || area.height() < 3 {
            return Layout::new([ChildLayout::new(0, area)]);
        }
        let x = area.x().saturating_add(1);
        let y = area.y().saturating_add(1);
        let w = area.width().saturating_sub(2);
        let h = area.height().saturating_sub(2);
        let mut children = vec![ChildLayout::new(
            0,
            Area::new((x, y).into(), (w, h.min(1)).into()),
        )];
        if self.projection.visibility.get() == WindowVisibility::Expanded
            && !self.projection.compact.get()
            && h > 2
        {
            children.push(ChildLayout::new(
                1,
                Area::new((x, y.saturating_add(2)).into(), (w, h - 3).into()),
            ));
            if self.resizable {
                children.push(ChildLayout::new(
                    2,
                    Area::new((x, y + h - 1).into(), (w, 1).into()),
                ));
            }
        }
        Layout::new(children)
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        let base = self.style.unwrap_or(ctx.theme().styles.surface_elevated);
        let border = if self.projection.active.get() {
            self.active_style
                .unwrap_or(ctx.theme().styles.border_focused)
        } else {
            ctx.theme().styles.border
        };
        let _ = chunk.fill(
            0,
            0,
            area.width(),
            area.height(),
            ' ',
            base.to_render_style(),
        );
        if area.width() > 1 && area.height() > 1 {
            border_renderer::render_border(chunk, self.border, border.to_render_style());
        }
        ctx.render_children(chunk, self);
    }

    fn pointer_enabled(&self) -> bool {
        self.projection.visibility.get() != WindowVisibility::Hidden
            && (self.movable || self.resizable)
    }
    fn pointer_cancel(&self, reason: DragCancelReason, ctx: &mut EventCtx<M>) {
        let active = ctx.state_mut::<FloatingWindowState>().drag.is_active();
        let event = ctx.state_mut::<FloatingWindowState>().drag.cancel(reason);
        if active && reason == DragCancelReason::Escape {
            ctx.set_handled();
        }
        self.projection.dragging.set(false);
        self.apply_drag(event, ctx);
        ctx.request_layout();
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if ctx.phase() == EventPhase::Bubble
            || ctx.was_handled()
            || self.projection.visibility.get() == WindowVisibility::Hidden
        {
            return;
        }
        let host_changed = {
            let state = ctx.state_mut::<FloatingWindowState>();
            state.drag.is_pending() && state.start_host != Some(self.projection.host.get())
        };
        if host_changed {
            self.pointer_cancel(DragCancelReason::CoordinateSpaceChanged, ctx);
            ctx.release_pointer();
            return;
        }
        // Only explicit chrome handles initiate a gesture. Content editors keep
        // their own cursor, scroll and pointer-capture behavior.
        let title_path = ctx
            .path()
            .child(WidgetKey::for_child(0, self.children[0].as_ref()));
        let grip_path = ctx
            .path()
            .child(WidgetKey::for_child(2, self.children[2].as_ref()));
        let target = if matches!(event, Event::Mouse(_)) {
            ctx.target_path()
        } else {
            ctx.focused_path()
        };
        let is_title = target == Some(&title_path);
        let is_grip = target == Some(&grip_path);
        match event {
            Event::Mouse(mouse) => {
                let Some(position) = ctx.cell_mouse_position(event) else {
                    return;
                };
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) if is_title || is_grip => {
                        if is_title && self.title_actions.is_empty() {
                            if let Some(bounds) =
                                ctx.bounds_for(&title_path).filter(|b| b.width() >= 7)
                            {
                                let x = position.x - i32::from(bounds.x());
                                let w = i32::from(bounds.width());
                                if let Some(handler) = self.on_close.as_ref().filter(|_| x >= w - 3)
                                {
                                    ctx.emit(handler());
                                    ctx.stop_propagation();
                                    return;
                                }
                                if x >= w - 5 && self.hideable {
                                    self.propose_visibility(WindowVisibility::Hidden, ctx);
                                    ctx.stop_propagation();
                                    return;
                                }
                                if x >= w - 7 && self.collapsible {
                                    let visibility = if self.projection.visibility.get()
                                        == WindowVisibility::Collapsed
                                    {
                                        WindowVisibility::Expanded
                                    } else {
                                        WindowVisibility::Collapsed
                                    };
                                    self.propose_visibility(visibility, ctx);
                                    ctx.stop_propagation();
                                    return;
                                }
                            }
                        }
                        if (is_title && !self.movable) || (is_grip && !self.resizable) {
                            return;
                        }
                        let Some(bounds) = ctx.bounds() else {
                            return;
                        };
                        let host = self.projection.host.get();
                        let mut rect = ctx.state_mut::<FloatingWindowState>().rect;
                        rect.x = i32::from(bounds.x()) - i32::from(host.x());
                        rect.y = i32::from(bounds.y()) - i32::from(host.y());
                        rect.width = bounds.width();
                        if self.projection.visibility.get() != WindowVisibility::Collapsed {
                            rect.height = bounds.height();
                        }
                        self.projection.dragging.set(true);
                        self.propose_rect(rect, ctx);
                        let state = ctx.state_mut::<FloatingWindowState>();
                        state.start_rect = rect;
                        state.start_host = Some(host);
                        state.resizing = is_grip;
                        state.drag.begin(position, mouse.modifiers, 0);
                        ctx.capture_pointer();
                        ctx.set_handled();
                        ctx.stop_propagation();
                        ctx.request_layout();
                    }
                    MouseEventKind::Drag(MouseButton::Left)
                        if ctx.state_mut::<FloatingWindowState>().drag.is_pending() =>
                    {
                        let events = ctx.state_mut::<FloatingWindowState>().drag.motion(
                            position,
                            mouse.modifiers,
                            0,
                            DragAxis::Both,
                        );
                        self.apply_drag(events, ctx);
                        ctx.set_handled();
                        ctx.stop_propagation();
                    }
                    MouseEventKind::Up(MouseButton::Left)
                        if ctx.state_mut::<FloatingWindowState>().drag.is_pending() =>
                    {
                        let event = ctx.state_mut::<FloatingWindowState>().drag.end(
                            position,
                            mouse.modifiers,
                            DragAxis::Both,
                        );
                        self.apply_drag(event, ctx);
                        ctx.release_pointer();
                        ctx.set_handled();
                        ctx.stop_propagation();
                    }
                    _ => {}
                }
            }
            Event::Key(key) if key.kind != KeyEventKind::Release && (is_title || is_grip) => {
                let delta = match key.code {
                    KeyCode::Left => CellPoint::new(-2, 0),
                    KeyCode::Right => CellPoint::new(2, 0),
                    KeyCode::Up => CellPoint::new(0, -1),
                    KeyCode::Down => CellPoint::new(0, 1),
                    KeyCode::Enter if is_title && self.collapsible => {
                        self.propose_visibility(
                            if self.projection.visibility.get() == WindowVisibility::Collapsed {
                                WindowVisibility::Expanded
                            } else {
                                WindowVisibility::Collapsed
                            },
                            ctx,
                        );
                        ctx.stop_propagation();
                        return;
                    }
                    KeyCode::Delete if is_title && self.on_close.is_some() => {
                        ctx.emit(self.on_close.as_ref().unwrap()());
                        ctx.stop_propagation();
                        return;
                    }
                    _ => return,
                };
                let rect = ctx.state_mut::<FloatingWindowState>().rect;
                let resize = is_grip || key.modifiers.contains(KeyModifiers::SHIFT);
                if (resize && !self.resizable) || (!resize && !self.movable) {
                    return;
                }
                self.propose_rect(
                    if resize {
                        rect.resized(delta, ResizeEdge::BottomRight)
                    } else {
                        rect.translated(delta)
                    },
                    ctx,
                );
                ctx.set_handled();
                ctx.stop_propagation();
            }
            _ => {}
        }
    }
}
struct Content<M> {
    children: Vec<Box<dyn Widget<M>>>,
}
impl<M> Widget<M> for Content<M> {
    fn constraints(&self) -> Constraints {
        Constraints::min(1, 1)
    }
    fn key(&self) -> Option<&str> {
        Some("window-content")
    }
    fn children(&self) -> &[Box<dyn Widget<M>>] {
        &self.children
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.render_children(chunk, self);
    }
}
#[derive(Clone, Copy, Default)]
struct TitleControls {
    collapsible: bool,
    hideable: bool,
    closable: bool,
}

struct Chrome<M> {
    key: String,
    text: String,
    grip: bool,
    controls: TitleControls,
    projection: WindowProjection,
    actions: Rc<Vec<Box<dyn Widget<M>>>>,
    actions_width: u16,
}
impl<M> Widget<M> for Chrome<M> {
    fn key(&self) -> Option<&str> {
        Some(&self.key)
    }
    fn focus_config(&self) -> FocusConfig {
        FocusConfig::Leaf
    }
    fn constraints(&self) -> Constraints {
        Constraints::min(1, 1)
    }
    fn children(&self) -> &[Box<dyn Widget<M>>] {
        &self.actions
    }
    fn layout(&self, area: Area, _: &MeasureCtx) -> Layout {
        let width = self.actions_width.min(area.width());
        Layout::new(self.actions.iter().enumerate().map(|(i, _)| {
            ChildLayout::new(
                i,
                Area::new(
                    (area.x() + area.width() - width, area.y()).into(),
                    (width, area.height()).into(),
                ),
            )
        }))
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let w = chunk.area().width();
        let style = if self.projection.active.get() || ctx.is_focused() {
            ctx.theme().styles.interactive_focused
        } else {
            ctx.theme().styles.surface_header
        }
        .to_render_style();
        let _ = chunk.fill(0, 0, w, 1, ' ', style);
        let reserve = if w < 8 {
            0
        } else if !self.actions.is_empty() {
            self.actions_width
        } else if self.grip {
            2
        } else {
            8
        };
        let text = chunk
            .text_metrics()
            .clip(&self.text, 0, w.saturating_sub(reserve) as usize);
        let _ = chunk.set_string(if w > 2 { 1 } else { 0 }, 0, &text, style);
        ctx.render_children(chunk, self);
        if !self.grip && self.actions.is_empty() && w >= 7 {
            let metrics = chunk.text_metrics();
            let glyph = |symbol, fallback| {
                if metrics.width(symbol) == 1 {
                    symbol
                } else {
                    fallback
                }
            };
            if self.controls.collapsible {
                let symbol = if self.projection.visibility.get() == WindowVisibility::Collapsed {
                    "+"
                } else {
                    glyph("−", "-")
                };
                let _ = chunk.set_string(w - 7, 0, symbol, style);
            }
            if self.controls.hideable {
                let _ = chunk.set_string(w - 5, 0, glyph("↓", "v"), style);
            }
            if self.controls.closable {
                let _ = chunk.set_string(w - 3, 0, glyph("×", "x"), style);
            }
        }
        if self.grip && w > 2 {
            let _ = chunk.set_string(w - 2, 0, "◢", style);
        }
    }
}
