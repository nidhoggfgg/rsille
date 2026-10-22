mod composition;
pub use composition::{MappedWidget, ScopedWidget, WidgetExt};

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

use render::area::Area;
use render::chunk::Chunk;
use smallvec::SmallVec;

use crate::animation::{
    AnimationCtx, AnimationStore, ClipMode, HitTestMode, LayoutSnapshot, LayoutTransition,
    MotionPolicy, Presence, SharedTransition, Timeline, TimelineFrame,
};
use crate::event::Event;
use crate::focus::FocusConfig;
use crate::layout::{Constraints, LayoutStyle, MeasuredSize, SizeProposal};
use crate::style::Style;
use crate::style::Theme;
use crate::widgets::visual::TerminalVisualCapabilities;

// ---------------------------------------------------------------------------
// WidgetKey, WidgetPath & WidgetId
// ---------------------------------------------------------------------------

/// A single segment in a widget path.
///
/// Widgets can be identified by their positional index (default) or by a
/// user-assigned stable name. Named keys survive reordering of siblings,
/// making persistent state (scroll offsets, cursor positions, …) robust
/// against dynamic list changes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WidgetKey {
    Index(usize),
    Named(String),
}

impl WidgetKey {
    /// Determine the key for a child widget: use the widget's own key if set,
    /// otherwise fall back to its positional index.
    pub fn for_child<M>(index: usize, widget: &dyn Widget<M>) -> Self {
        match widget.key() {
            Some(name) => WidgetKey::Named(name.to_owned()),
            None => WidgetKey::Index(index),
        }
    }
}

impl From<usize> for WidgetKey {
    fn from(i: usize) -> Self {
        WidgetKey::Index(i)
    }
}

impl From<&str> for WidgetKey {
    fn from(s: &str) -> Self {
        WidgetKey::Named(s.to_owned())
    }
}

impl From<String> for WidgetKey {
    fn from(s: String) -> Self {
        WidgetKey::Named(s)
    }
}

impl fmt::Display for WidgetKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WidgetKey::Index(i) => write!(f, "{i}"),
            WidgetKey::Named(s) => write!(f, "\"{s}\""),
        }
    }
}

/// Path from the root of the current widget tree to a specific widget.
///
/// Each segment is a [`WidgetKey`] identifying the child at that level. This is
/// a frame-local address used for traversal, event routing, hit testing, and
/// geometry. Cross-frame state should use [`WidgetId`] instead.
/// Uses `SmallVec` to avoid heap allocation for typical tree depths (≤ 8).
#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub struct WidgetPath(SmallVec<[WidgetKey; 8]>);

impl WidgetPath {
    pub fn root() -> Self {
        Self(SmallVec::new())
    }

    pub fn child(&self, key: impl Into<WidgetKey>) -> Self {
        let mut p = self.0.clone();
        p.push(key.into());
        Self(p)
    }

    pub fn push(&mut self, key: impl Into<WidgetKey>) {
        self.0.push(key.into());
    }

    pub fn pop(&mut self) -> Option<WidgetKey> {
        self.0.pop()
    }

    pub fn as_slice(&self) -> &[WidgetKey] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn starts_with(&self, prefix: &WidgetPath) -> bool {
        self.0.starts_with(prefix.as_slice())
    }

    pub fn ancestors_inclusive(&self) -> Vec<WidgetPath> {
        let mut ancestors = Vec::with_capacity(self.len() + 1);
        ancestors.push(WidgetPath::root());

        let mut path = WidgetPath::root();
        for key in self.as_slice() {
            path.push(key.clone());
            ancestors.push(path.clone());
        }

        ancestors
    }
}

impl fmt::Debug for WidgetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WidgetPath[")?;
        for (i, key) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, " → ")?;
            }
            write!(f, "{key}")?;
        }
        write!(f, "]")
    }
}

impl fmt::Display for WidgetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, key) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, "/")?;
            }
            write!(f, "{key}")?;
        }
        Ok(())
    }
}

/// Stable widget identity used for focus, widget-local state, and animation.
///
/// Named widget keys act as stable anchors. A keyed descendant keeps the same
/// identity even when intermediate unkeyed layout wrappers move or are rebuilt.
/// Unkeyed widgets still receive positional identities scoped to the nearest
/// named ancestor, so they preserve the existing path-like behavior.
#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub struct WidgetId(SmallVec<[WidgetKey; 8]>);

impl WidgetId {
    pub fn root() -> Self {
        Self(SmallVec::new())
    }

    pub fn child(&self, key: impl Into<WidgetKey>) -> Self {
        let mut id = self.0.clone();
        id.push(key.into());
        Self(id)
    }

    pub fn as_slice(&self) -> &[WidgetKey] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn starts_with(&self, prefix: &WidgetId) -> bool {
        self.0.starts_with(prefix.as_slice())
    }

    pub fn from_path(path: &WidgetPath) -> Self {
        Self(path.as_slice().iter().cloned().collect())
    }

    pub(crate) fn for_child(
        parent_id: &WidgetId,
        stable_scope_id: &WidgetId,
        key: &WidgetKey,
    ) -> (WidgetId, WidgetId) {
        match key {
            WidgetKey::Named(_) => {
                let child_id = stable_scope_id.child(key.clone());
                (child_id.clone(), child_id)
            }
            WidgetKey::Index(_) => (parent_id.child(key.clone()), stable_scope_id.clone()),
        }
    }
}

impl From<&WidgetPath> for WidgetId {
    fn from(path: &WidgetPath) -> Self {
        WidgetId::from_path(path)
    }
}

impl From<&WidgetId> for WidgetId {
    fn from(id: &WidgetId) -> Self {
        id.clone()
    }
}

impl fmt::Debug for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WidgetId[")?;
        for (i, key) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, " → ")?;
            }
            write!(f, "{key}")?;
        }
        write!(f, "]")
    }
}

impl fmt::Display for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, key) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, "/")?;
            }
            write!(f, "{key}")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Widget trait
// ---------------------------------------------------------------------------

/// Lifecycle of a mounted description. Hidden, retained children remain mounted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    Mount,
    Sync,
    Unmount,
}

/// Mutable widget-local state access during reconciliation, before measurement.
/// Unmount runs once at logical removal, before exit rendering retains resources.
pub struct StateCtx<'a> {
    store: &'a mut WidgetStore,
    path: WidgetPath,
    id: WidgetId,
    lifecycle: Lifecycle,
}

impl<'a> StateCtx<'a> {
    pub(crate) fn new(
        store: &'a mut WidgetStore,
        path: WidgetPath,
        id: WidgetId,
        lifecycle: Lifecycle,
    ) -> Self {
        Self {
            store,
            path,
            id,
            lifecycle,
        }
    }

    pub fn lifecycle(&self) -> Lifecycle {
        self.lifecycle
    }
    pub fn path(&self) -> &WidgetPath {
        &self.path
    }
    pub fn id(&self) -> &WidgetId {
        &self.id
    }
    pub fn state<T: 'static>(&self) -> Option<&T> {
        self.store.get(&self.id)
    }
    pub fn state_mut<T: Default + 'static>(&mut self) -> &mut T {
        self.store.get_or_default(&self.id)
    }
}

/// Read-only context passed during widget measurement.
///
/// Measurement is intentionally side-effect free: widgets may inspect theme,
/// persistent state, identity, and frame metadata, but cannot emit messages or
/// mutate application state.
pub struct MeasureCtx<'a> {
    store: &'a WidgetStore,
    theme: &'a Theme,
    current_path: WidgetPath,
    current_id: WidgetId,
    stable_scope_id: WidgetId,
    now: std::time::Instant,
    frame: u64,
    visual_capabilities: TerminalVisualCapabilities,
}

impl<'a> MeasureCtx<'a> {
    pub fn new(store: &'a WidgetStore, theme: &'a Theme) -> Self {
        Self {
            store,
            theme,
            current_path: WidgetPath::root(),
            current_id: WidgetId::root(),
            stable_scope_id: WidgetId::root(),
            now: std::time::Instant::now(),
            frame: 0,
            visual_capabilities: TerminalVisualCapabilities::default(),
        }
    }

    pub(crate) fn with_runtime(
        store: &'a WidgetStore,
        theme: &'a Theme,
        now: std::time::Instant,
        frame: u64,
        visual_capabilities: TerminalVisualCapabilities,
    ) -> Self {
        Self {
            store,
            theme,
            current_path: WidgetPath::root(),
            current_id: WidgetId::root(),
            stable_scope_id: WidgetId::root(),
            now,
            frame,
            visual_capabilities,
        }
    }

    pub fn child_ctx(&self, key: impl Into<WidgetKey>) -> Self {
        let key = key.into();
        let (current_id, stable_scope_id) =
            WidgetId::for_child(&self.current_id, &self.stable_scope_id, &key);

        Self {
            store: self.store,
            theme: self.theme,
            current_path: self.current_path.child(key),
            current_id,
            stable_scope_id,
            now: self.now,
            frame: self.frame,
            visual_capabilities: self.visual_capabilities,
        }
    }

    pub fn theme(&self) -> &Theme {
        self.theme
    }

    pub fn now(&self) -> std::time::Instant {
        self.now
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    pub fn visual_capabilities(&self) -> TerminalVisualCapabilities {
        self.visual_capabilities
    }

    pub fn path(&self) -> &WidgetPath {
        &self.current_path
    }

    pub fn id(&self) -> &WidgetId {
        &self.current_id
    }

    pub fn state<T: Default + 'static>(&self) -> Option<&T> {
        self.store.get::<T>(&self.current_id)
    }
}

/// Core widget trait that all TUI components implement.
///
/// Widgets are **immutable descriptions** of UI. All mutable state lives in
/// [`WidgetStore`] (accessed via contexts), not inside the widget itself.
///
/// The generic parameter `M` is the message type for the application.
pub trait Widget<M> {
    /// Concrete component type used together with identity to match state.
    /// Transparent boxing must forward this method. The default distinguishes
    /// concrete Rust types, including generic parameters, using `TypeId`.
    fn widget_type(&self) -> TypeId
    where
        Self: 'static,
    {
        TypeId::of::<Self>()
    }

    /// Initialize state on Mount, synchronize controlled properties on Sync,
    /// and release interactive resources on Unmount. Mount also performs the
    /// initial property synchronization. Sync may also run before input batches
    /// and drawing; implementations must make it idempotent.
    fn lifecycle(&self, _ctx: &mut StateCtx) {}

    /// Render the widget into the provided chunk.
    ///
    /// Read persistent state from `ctx.state::<T>()` and focus info from
    /// `ctx.is_focused()`. The widget draws at relative coordinates within the chunk.
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx);

    /// Handle an event routed to this widget by the framework.
    ///
    /// Widgets receive events during capture, target, and bubble phases.
    /// Write persistent state via `ctx.state_mut::<T>()`, emit messages via
    /// `ctx.emit(msg)`, and request focus changes via the focus helpers.
    fn handle_event(&self, _event: &Event, _ctx: &mut EventCtx<M>) {}

    /// Advance component-level animation state for this widget.
    ///
    /// Return `true` when the animation changed a value or needs another frame.
    fn animate(&self, _ctx: &mut AnimationCtx) -> bool {
        false
    }

    /// Presence declaration used by the runtime for enter/exit lifecycle animation.
    fn presence(&self) -> Option<&Presence> {
        None
    }

    /// Layout transition declaration used by parent layout containers.
    fn layout_transition(&self) -> Option<LayoutTransition> {
        None
    }

    /// Shared layout transition declaration used across widget paths.
    fn shared_transition(&self) -> Option<&SharedTransition> {
        None
    }

    /// Pointer hit-testing strategy for animated geometry.
    fn hit_test_mode(&self) -> HitTestMode {
        HitTestMode::Target
    }

    /// Return size constraints for layout computation.
    fn constraints(&self) -> Constraints;

    /// Return rich layout intent for measurement-aware containers.
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::from_constraints(self.constraints())
    }

    /// Resolve intrinsic layout constraints using the active theme/text policy.
    fn layout_style_with(&self, _ctx: &MeasureCtx) -> LayoutStyle {
        self.layout_style()
    }

    /// Measure the widget's preferred size for a proposed parent allocation.
    fn measure(&self, proposal: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
        self.layout_style_with(ctx).resolve_fallback_size(proposal)
    }

    /// How this widget participates in keyboard focus.
    fn focus_config(&self) -> FocusConfig {
        FocusConfig::None
    }

    /// Return child widgets (for containers like Flex/Grid).
    /// Leaf widgets return the default empty slice.
    fn children(&self) -> &[Box<dyn Widget<M>>] {
        &[]
    }

    /// Hidden children explicitly retained by this description. They receive
    /// lifecycle synchronization but no focus, input, animation updates or draw.
    /// These must be disjoint from `children()`, with the same local key rules.
    /// Retention ends when the owning description drops them or is unmounted.
    fn retained_children(&self) -> &[Box<dyn Widget<M>>] {
        &[]
    }

    /// Return a stable key for this widget.
    ///
    /// When set, the framework uses this key as a stable identity anchor. This
    /// makes focus, widget-local state, and animation survive sibling reordering
    /// and intermediate unkeyed layout wrapper changes. Keys should be unique
    /// within the nearest keyed ancestor.
    fn key(&self) -> Option<&str> {
        None
    }

    /// Optional resource namespace owned by this node's mounted lifetime.
    /// Usually supplied through `WidgetExt::scope`.
    fn resource_scope(&self) -> Option<&str> {
        None
    }
}

impl<M> Widget<M> for Box<dyn Widget<M>> {
    fn widget_type(&self) -> TypeId
    where
        Self: 'static,
    {
        (**self).widget_type()
    }

    fn lifecycle(&self, ctx: &mut StateCtx) {
        (**self).lifecycle(ctx)
    }

    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        (**self).render(chunk, ctx)
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        (**self).handle_event(event, ctx)
    }

    fn animate(&self, ctx: &mut AnimationCtx) -> bool {
        (**self).animate(ctx)
    }

    fn presence(&self) -> Option<&Presence> {
        (**self).presence()
    }

    fn layout_transition(&self) -> Option<LayoutTransition> {
        (**self).layout_transition()
    }

    fn shared_transition(&self) -> Option<&SharedTransition> {
        (**self).shared_transition()
    }

    fn hit_test_mode(&self) -> HitTestMode {
        (**self).hit_test_mode()
    }

    fn constraints(&self) -> Constraints {
        (**self).constraints()
    }

    fn layout_style(&self) -> LayoutStyle {
        (**self).layout_style()
    }

    fn layout_style_with(&self, ctx: &MeasureCtx) -> LayoutStyle {
        (**self).layout_style_with(ctx)
    }

    fn measure(&self, proposal: SizeProposal, ctx: &MeasureCtx) -> MeasuredSize {
        (**self).measure(proposal, ctx)
    }

    fn focus_config(&self) -> FocusConfig {
        (**self).focus_config()
    }

    fn children(&self) -> &[Box<dyn Widget<M>>] {
        (**self).children()
    }

    fn retained_children(&self) -> &[Box<dyn Widget<M>>] {
        (**self).retained_children()
    }

    fn key(&self) -> Option<&str> {
        (**self).key()
    }

    fn resource_scope(&self) -> Option<&str> {
        (**self).resource_scope()
    }
}

// ---------------------------------------------------------------------------
// RenderCtx – immutable context passed during rendering
// ---------------------------------------------------------------------------

/// Context passed to [`Widget::render`].
///
/// Provides read-only access to the [`WidgetStore`], focus information,
/// and the current render theme.
#[derive(Clone, Copy)]
enum AnimationStoreView<'a> {
    ReadOnly(&'a AnimationStore),
    Mutable(&'a RefCell<AnimationStore>),
}

impl AnimationStoreView<'_> {
    fn value(self, id: &WidgetId, channel: &str) -> Option<f64> {
        match self {
            Self::ReadOnly(store) => store.value(id, channel),
            Self::Mutable(store) => store.borrow().value(id, channel),
        }
    }

    fn style(self, id: &WidgetId, channel: &str) -> Option<Style> {
        match self {
            Self::ReadOnly(store) => store.style(id, channel),
            Self::Mutable(store) => store.borrow().style(id, channel),
        }
    }

    fn layout_snapshot(self, id: &WidgetId, channel: &str) -> Option<LayoutSnapshot> {
        match self {
            Self::ReadOnly(store) => store.layout_snapshot(id, channel),
            Self::Mutable(store) => store.borrow().layout_snapshot(id, channel),
        }
    }

    fn shared_layout_snapshot(self, id: &str) -> Option<LayoutSnapshot> {
        match self {
            Self::ReadOnly(store) => store.shared_layout_snapshot(id),
            Self::Mutable(store) => store.borrow().shared_layout_snapshot(id),
        }
    }

    fn track_timeline(
        self,
        path: &WidgetPath,
        channel: &str,
        timeline: Timeline,
        restart: bool,
        now: std::time::Instant,
        motion_policy: MotionPolicy,
    ) -> Vec<TimelineFrame> {
        match self {
            Self::ReadOnly(store) => store.timeline_frames(path, channel, now),
            Self::Mutable(store) => {
                store
                    .borrow_mut()
                    .track_timeline(path, channel, timeline, restart, now, motion_policy)
                    .0
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HitRegion {
    pub path: WidgetPath,
    pub area: Area,
}

impl HitRegion {
    fn new(path: WidgetPath, area: Area) -> Option<Self> {
        if area.width() == 0 || area.height() == 0 {
            return None;
        }

        Some(Self { path, area })
    }

    pub(crate) fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.area.x()
            && x < self.area.x().saturating_add(self.area.width())
            && y >= self.area.y()
            && y < self.area.y().saturating_add(self.area.height())
    }
}

pub struct RenderCtx<'a> {
    store: &'a WidgetStore,
    animation_store: AnimationStoreView<'a>,
    theme: &'a Theme,
    focused_path: Option<&'a WidgetPath>,
    focused_id: Option<WidgetId>,
    current_path: WidgetPath,
    current_id: WidgetId,
    stable_scope_id: WidgetId,
    geometry: &'a RefCell<HashMap<WidgetPath, Area>>,
    hit_regions: Option<&'a RefCell<Vec<HitRegion>>>,
    hit_clip: Option<Area>,
    now: std::time::Instant,
    frame: u64,
    motion_policy: MotionPolicy,
    visual_capabilities: TerminalVisualCapabilities,
    layout_target: Option<Area>,
    layout_managed: bool,
    exit_render: bool,
}

impl<'a> RenderCtx<'a> {
    pub fn new(
        store: &'a WidgetStore,
        animation_store: &'a AnimationStore,
        theme: &'a Theme,
        focused_path: Option<&'a WidgetPath>,
        geometry: &'a RefCell<HashMap<WidgetPath, Area>>,
    ) -> Self {
        Self {
            store,
            animation_store: AnimationStoreView::ReadOnly(animation_store),
            theme,
            focused_path,
            focused_id: focused_path.map(WidgetId::from_path),
            current_path: WidgetPath::root(),
            current_id: WidgetId::root(),
            stable_scope_id: WidgetId::root(),
            geometry,
            hit_regions: None,
            hit_clip: None,
            now: std::time::Instant::now(),
            frame: 0,
            motion_policy: MotionPolicy::default(),
            visual_capabilities: TerminalVisualCapabilities::default(),
            layout_target: None,
            layout_managed: false,
            exit_render: false,
        }
    }

    pub(crate) fn with_runtime(
        store: &'a WidgetStore,
        animation_store: &'a RefCell<AnimationStore>,
        theme: &'a Theme,
        focused_path: Option<&'a WidgetPath>,
        focused_id: Option<WidgetId>,
        geometry: &'a RefCell<HashMap<WidgetPath, Area>>,
        hit_regions: &'a RefCell<Vec<HitRegion>>,
        now: std::time::Instant,
        frame: u64,
        motion_policy: MotionPolicy,
        visual_capabilities: TerminalVisualCapabilities,
    ) -> Self {
        Self {
            store,
            animation_store: AnimationStoreView::Mutable(animation_store),
            theme,
            focused_path,
            focused_id,
            current_path: WidgetPath::root(),
            current_id: WidgetId::root(),
            stable_scope_id: WidgetId::root(),
            geometry,
            hit_regions: Some(hit_regions),
            hit_clip: None,
            now,
            frame,
            motion_policy,
            visual_capabilities,
            layout_target: None,
            layout_managed: false,
            exit_render: false,
        }
    }

    /// The theme for the current render pass.
    pub fn theme(&self) -> &Theme {
        self.theme
    }

    /// The render instant supplied by the runtime for this frame.
    pub fn now(&self) -> std::time::Instant {
        self.now
    }

    /// Monotonic render frame number supplied by the runtime.
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// Create a read-only measurement context at the current widget path.
    pub fn measure_ctx(&self) -> MeasureCtx<'a> {
        MeasureCtx {
            store: self.store,
            theme: self.theme,
            current_path: self.current_path.clone(),
            current_id: self.current_id.clone(),
            stable_scope_id: self.stable_scope_id.clone(),
            now: self.now,
            frame: self.frame,
            visual_capabilities: self.visual_capabilities,
        }
    }

    /// The global motion policy active for this render pass.
    pub fn motion_policy(&self) -> MotionPolicy {
        self.motion_policy
    }

    /// Terminal feature hints active for visual post-processing effects.
    pub fn visual_capabilities(&self) -> TerminalVisualCapabilities {
        self.visual_capabilities
    }

    /// Whether this render pass is drawing a retained exiting subtree.
    pub fn is_exit_render(&self) -> bool {
        self.exit_render
    }

    /// Whether the current widget has keyboard focus.
    pub fn is_focused(&self) -> bool {
        self.focused_id
            .as_ref()
            .map(|id| id == &self.current_id)
            .unwrap_or(false)
    }

    /// Whether the currently focused widget is inside this subtree.
    pub fn is_focus_within(&self) -> bool {
        self.focused_path
            .map(|fp| fp.starts_with(&self.current_path))
            .unwrap_or(false)
    }

    /// The first descendant key along the focused path, relative to this widget.
    pub fn focused_descendant_key(&self) -> Option<&WidgetKey> {
        let focused = self.focused_path?;
        if !focused.starts_with(&self.current_path) || focused.len() <= self.current_path.len() {
            return None;
        }
        focused.as_slice().get(self.current_path.len())
    }

    /// The globally focused path.
    pub fn focused_path(&self) -> Option<&WidgetPath> {
        self.focused_path
    }

    /// Read persistent state stored for this widget.
    /// Returns `None` if no state has been stored yet.
    pub fn state<T: Default + 'static>(&self) -> Option<&T> {
        self.store.get::<T>(&self.current_id)
    }

    /// Read the current animation value for this widget and channel.
    pub fn animation_value(&self, channel: &str) -> Option<f64> {
        self.animation_store.value(&self.current_id, channel)
    }

    /// Read the current layout animation snapshot for this widget and channel.
    pub fn layout_animation(&self, channel: &str) -> Option<LayoutSnapshot> {
        self.animation_store
            .layout_snapshot(&self.current_id, channel)
    }

    /// Read the current shared layout animation snapshot by shared id.
    pub fn shared_layout_animation(&self, id: &str) -> Option<LayoutSnapshot> {
        self.animation_store.shared_layout_snapshot(id)
    }

    /// Read the current style animation value for this widget and channel.
    pub fn animation_style(&self, channel: &str) -> Option<Style> {
        self.animation_store.style(&self.current_id, channel)
    }

    /// Track a target area and return the animated display area.
    pub fn track_layout(&self, channel: &str, target: Area, transition: LayoutTransition) -> Area {
        let AnimationStoreView::Mutable(store) = self.animation_store else {
            return target;
        };

        let (displayed, _) = store.borrow_mut().track_layout(
            &self.current_id,
            channel,
            target,
            transition,
            self.now,
            self.motion_policy,
        );
        displayed
    }

    /// Track a shared target area and return the animated display area.
    pub fn track_shared_layout(
        &self,
        id: &str,
        target: Area,
        transition: LayoutTransition,
    ) -> Area {
        let AnimationStoreView::Mutable(store) = self.animation_store else {
            return target;
        };

        let (displayed, _) = store.borrow_mut().track_shared_layout(
            id,
            &self.current_path,
            target,
            transition,
            self.now,
            self.motion_policy,
        );
        displayed
    }

    /// Track a timeline and return transition frames active at the current render instant.
    pub fn track_timeline(
        &self,
        channel: &str,
        timeline: Timeline,
        restart: bool,
    ) -> Vec<TimelineFrame> {
        self.animation_store.track_timeline(
            &self.current_path,
            channel,
            timeline,
            restart,
            self.now,
            self.motion_policy,
        )
    }

    /// Read persistent state, or return a default reference if absent.
    /// This is a convenience that avoids `unwrap_or` at every call-site
    /// by falling back to a leaked static default. Use sparingly.
    pub fn state_or_default<T: Default + 'static>(&self) -> &T {
        self.store.get::<T>(&self.current_id).unwrap_or_else(|| {
            // Safe: Default is computed once per type via OnceLock-like pattern
            // We use a thread-local to avoid leaking.
            // For rendering purposes, returning a stack reference is fine because
            // we only need it for the duration of the render call.
            // We use a small trick: store the default in a thread-local.
            thread_local! {
                static DEFAULTS: std::cell::RefCell<HashMap<std::any::TypeId, Box<dyn Any>>> =
                    std::cell::RefCell::new(HashMap::new());
            }
            DEFAULTS.with(|defaults| {
                let mut map = defaults.borrow_mut();
                let entry = map
                    .entry(std::any::TypeId::of::<T>())
                    .or_insert_with(|| Box::new(T::default()));
                // SAFETY: the borrow_mut guard is dropped but the Box lives in
                // thread-local storage for the entire thread lifetime. We extend
                // the lifetime to 'a which is bounded by the render call.
                let ptr: *const T = entry.downcast_ref::<T>().unwrap();
                unsafe { &*ptr }
            })
        })
    }

    /// Create a child context for rendering a child widget.
    ///
    /// The `key` identifies the child — use [`WidgetKey::for_child`] to
    /// automatically pick a named key when the child widget provides one,
    /// falling back to the positional index.
    pub fn child_ctx(&self, key: impl Into<WidgetKey>) -> RenderCtx<'a> {
        let key = key.into();
        let (current_id, stable_scope_id) =
            WidgetId::for_child(&self.current_id, &self.stable_scope_id, &key);
        RenderCtx {
            store: self.store,
            animation_store: self.animation_store,
            theme: self.theme,
            focused_path: self.focused_path,
            focused_id: self.focused_id.clone(),
            current_path: self.current_path.child(key),
            current_id,
            stable_scope_id,
            geometry: self.geometry,
            hit_regions: self.hit_regions,
            hit_clip: self.hit_clip,
            now: self.now,
            frame: self.frame,
            motion_policy: self.motion_policy,
            visual_capabilities: self.visual_capabilities,
            layout_target: None,
            layout_managed: false,
            exit_render: self.exit_render,
        }
    }

    pub(crate) fn child_ctx_with_layout(
        &self,
        key: impl Into<WidgetKey>,
        target: Area,
    ) -> RenderCtx<'a> {
        let key = key.into();
        let (current_id, stable_scope_id) =
            WidgetId::for_child(&self.current_id, &self.stable_scope_id, &key);
        RenderCtx {
            store: self.store,
            animation_store: self.animation_store,
            theme: self.theme,
            focused_path: self.focused_path,
            focused_id: self.focused_id.clone(),
            current_path: self.current_path.child(key),
            current_id,
            stable_scope_id,
            geometry: self.geometry,
            hit_regions: self.hit_regions,
            hit_clip: self.hit_clip,
            now: self.now,
            frame: self.frame,
            motion_policy: self.motion_policy,
            visual_capabilities: self.visual_capabilities,
            layout_target: Some(target),
            layout_managed: true,
            exit_render: self.exit_render,
        }
    }

    pub(crate) fn exit_render_ctx<'b>(
        &'b self,
        store: &'b WidgetStore,
        animations: &'b AnimationStore,
        geometry: &'b RefCell<HashMap<WidgetPath, Area>>,
        hit_regions: &'b RefCell<Vec<HitRegion>>,
    ) -> RenderCtx<'b> {
        RenderCtx {
            store,
            animation_store: AnimationStoreView::ReadOnly(animations),
            theme: self.theme,
            focused_path: None,
            focused_id: None,
            current_path: self.current_path.clone(),
            current_id: self.current_id.clone(),
            stable_scope_id: self.stable_scope_id.clone(),
            geometry,
            hit_regions: Some(hit_regions),
            hit_clip: self.hit_clip,
            now: self.now,
            frame: self.frame,
            motion_policy: self.motion_policy,
            visual_capabilities: self.visual_capabilities,
            layout_target: self.layout_target,
            layout_managed: self.layout_managed,
            exit_render: true,
        }
    }

    /// Return the logical target area when a parent container has already
    /// applied a layout transition for this widget.
    pub fn layout_target(&self) -> Option<Area> {
        self.layout_target
    }

    pub(crate) fn layout_is_managed(&self) -> bool {
        self.layout_managed
    }

    pub(crate) fn prepare_child_layout<M>(
        &self,
        key: WidgetKey,
        child: &dyn Widget<M>,
        target: Area,
    ) -> (Area, RenderCtx<'a>, ClipMode) {
        let path = self.current_path.child(key.clone());
        let (id, _) = WidgetId::for_child(&self.current_id, &self.stable_scope_id, &key);
        self.geometry.borrow_mut().insert(path.clone(), target);

        if let Some(shared) = child.shared_transition() {
            let display = match self.animation_store {
                AnimationStoreView::ReadOnly(_) => target,
                AnimationStoreView::Mutable(store) => {
                    store
                        .borrow_mut()
                        .track_shared_layout(
                            &shared.id,
                            &path,
                            target,
                            shared.layout,
                            self.now,
                            self.motion_policy,
                        )
                        .0
                }
            };
            let hit_area = hit_area_for(shared.layout.hit_test, target, display);
            self.record_child_hit_area(&path, hit_area);
            return (
                display,
                self.child_ctx_with_layout(key, target)
                    .with_hit_area(hit_area),
                shared.layout.clip,
            );
        }

        if let Some(transition) = child.layout_transition() {
            let display = match self.animation_store {
                AnimationStoreView::ReadOnly(_) => target,
                AnimationStoreView::Mutable(store) => {
                    store
                        .borrow_mut()
                        .track_layout(
                            &id,
                            "layout",
                            target,
                            transition,
                            self.now,
                            self.motion_policy,
                        )
                        .0
                }
            };
            let hit_area = hit_area_for(transition.hit_test, target, display);
            self.record_child_hit_area(&path, hit_area);
            return (
                display,
                self.child_ctx_with_layout(key, target)
                    .with_hit_area(hit_area),
                transition.clip,
            );
        }

        let hit_area = hit_area_for(child.hit_test_mode(), target, target);
        self.record_child_hit_area(&path, hit_area);
        (
            target,
            self.child_ctx(key).with_hit_area(hit_area),
            ClipMode::None,
        )
    }

    pub(crate) fn render_child_at<M>(
        &self,
        chunk: &mut Chunk,
        key: WidgetKey,
        child: &dyn Widget<M>,
        target: Area,
    ) {
        let (display, child_ctx, clip) = self.prepare_child_layout(key, child, target);
        if display.width() == 0 || display.height() == 0 {
            return;
        }

        let render_area = match clip {
            ClipMode::None | ClipMode::ClipToAnimatedBounds => display,
            ClipMode::ClipToTargetBounds => {
                let Some(clipped) = display.clamp_to(&target) else {
                    return;
                };
                clipped
            }
        };

        let _ = chunk.with_clip(render_area, |child_chunk| {
            child.render(child_chunk, &child_ctx);
        });
    }

    /// The current widget path in the tree.
    pub fn path(&self) -> &WidgetPath {
        &self.current_path
    }

    /// The stable identity for the current widget.
    pub fn id(&self) -> &WidgetId {
        &self.current_id
    }

    /// Record the rendered bounds for the current widget.
    pub fn record_bounds(&self, area: Area) {
        self.geometry
            .borrow_mut()
            .insert(self.current_path.clone(), area);
    }

    /// Record explicit pointer hit-test bounds for the current widget.
    pub fn record_hit_bounds(&self, target: Area, display: Area, mode: HitTestMode) {
        self.record_child_hit_area(&self.current_path, hit_area_for(mode, target, display));
    }

    pub(crate) fn with_hit_area(mut self, area: Option<Area>) -> Self {
        self.hit_clip = Some(
            area.and_then(|area| self.constrain_hit_area(area))
                .unwrap_or_else(zero_area),
        );
        self
    }

    fn record_child_hit_area(&self, path: &WidgetPath, area: Option<Area>) {
        let Some(regions) = self.hit_regions else {
            return;
        };

        let area = area.and_then(|area| self.constrain_hit_area(area));

        if let Some(region) = area.and_then(|area| HitRegion::new(path.clone(), area)) {
            regions.borrow_mut().push(region);
        }
    }

    fn constrain_hit_area(&self, area: Area) -> Option<Area> {
        match self.hit_clip {
            Some(clip) => area.clamp_to(&clip),
            None => Some(area),
        }
    }
}

pub(crate) fn hit_area_for(mode: HitTestMode, target: Area, display: Area) -> Option<Area> {
    match mode {
        HitTestMode::Target => Some(target),
        HitTestMode::Display => Some(display),
        HitTestMode::TargetAndDisplay => Some(union_area(target, display)),
        HitTestMode::None => None,
    }
}

fn union_area(a: Area, b: Area) -> Area {
    let left = a.x().min(b.x());
    let top = a.y().min(b.y());
    let right = a
        .x()
        .saturating_add(a.width())
        .max(b.x().saturating_add(b.width()));
    let bottom = a
        .y()
        .saturating_add(a.height())
        .max(b.y().saturating_add(b.height()));

    Area::new(
        (left, top).into(),
        (right.saturating_sub(left), bottom.saturating_sub(top)).into(),
    )
}

fn zero_area() -> Area {
    Area::new((0, 0).into(), (0, 0).into())
}

// ---------------------------------------------------------------------------
// EventCtx – mutable context passed during event handling
// ---------------------------------------------------------------------------

/// The current event routing phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPhase {
    Capture,
    Target,
    Bubble,
}

/// Requested focus change emitted by event handlers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusRequest {
    Set(WidgetPath),
    Clear,
    Next,
    Prev,
    NextInScope(Option<WidgetPath>),
    PrevInScope(Option<WidgetPath>),
}

/// Final event outcome collected after a widget has handled an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventOutcome {
    pub handled: bool,
    pub stop_propagation: bool,
    pub focus_request: Option<FocusRequest>,
}

/// Context passed to [`Widget::handle_event`].
///
/// Provides mutable access to the [`WidgetStore`] and message collection.
pub struct EventCtx<'a, M> {
    store: &'a mut WidgetStore,
    messages: &'a mut Vec<M>,
    path: WidgetPath,
    id: WidgetId,
    stable_scope_id: WidgetId,
    focused_path: Option<WidgetPath>,
    geometry: &'a HashMap<WidgetPath, Area>,
    phase: EventPhase,
    already_handled: bool,
    handled: bool,
    stop_propagation: bool,
    focus_request: Option<FocusRequest>,
    theme: Option<&'a Theme>,
    now: std::time::Instant,
    frame: u64,
    visual_capabilities: TerminalVisualCapabilities,
}

impl<'a, M> EventCtx<'a, M> {
    pub fn new(
        store: &'a mut WidgetStore,
        messages: &'a mut Vec<M>,
        path: WidgetPath,
        id: WidgetId,
        focused_path: Option<WidgetPath>,
        geometry: &'a HashMap<WidgetPath, Area>,
        phase: EventPhase,
        already_handled: bool,
    ) -> Self {
        Self {
            store,
            messages,
            path,
            id,
            stable_scope_id: WidgetId::root(),
            focused_path,
            geometry,
            phase,
            already_handled,
            handled: false,
            stop_propagation: false,
            focus_request: None,
            theme: None,
            now: std::time::Instant::now(),
            frame: 0,
            visual_capabilities: TerminalVisualCapabilities::default(),
        }
    }

    pub(crate) fn with_runtime(
        store: &'a mut WidgetStore,
        messages: &'a mut Vec<M>,
        path: WidgetPath,
        id: WidgetId,
        stable_scope_id: WidgetId,
        focused_path: Option<WidgetPath>,
        geometry: &'a HashMap<WidgetPath, Area>,
        phase: EventPhase,
        already_handled: bool,
        theme: &'a Theme,
        now: std::time::Instant,
        frame: u64,
        visual_capabilities: TerminalVisualCapabilities,
    ) -> Self {
        Self {
            store,
            messages,
            path,
            id,
            stable_scope_id,
            focused_path,
            geometry,
            phase,
            already_handled,
            handled: false,
            stop_propagation: false,
            focus_request: None,
            theme: Some(theme),
            now,
            frame,
            visual_capabilities,
        }
    }

    fn map_messages<C>(&mut self, handle: impl FnOnce(&mut EventCtx<C>), map: impl Fn(C) -> M) {
        let mut messages = Vec::new();
        let mut child = EventCtx {
            store: &mut *self.store,
            messages: &mut messages,
            path: self.path.clone(),
            id: self.id.clone(),
            stable_scope_id: self.stable_scope_id.clone(),
            focused_path: self.focused_path.clone(),
            geometry: self.geometry,
            phase: self.phase,
            already_handled: self.already_handled,
            handled: self.handled,
            stop_propagation: self.stop_propagation,
            focus_request: self.focus_request.clone(),
            theme: self.theme,
            now: self.now,
            frame: self.frame,
            visual_capabilities: self.visual_capabilities,
        };
        handle(&mut child);
        let outcome = child.finish();
        self.handled = outcome.handled;
        self.stop_propagation = outcome.stop_propagation;
        self.focus_request = outcome.focus_request;
        self.messages.extend(messages.into_iter().map(map));
    }

    pub fn text_metrics(&self) -> render::text::TextMetrics {
        self.theme
            .map(|theme| theme.text_metrics)
            .unwrap_or_default()
    }

    /// Get or create mutable persistent state for this widget.
    pub fn state_mut<T: Default + 'static>(&mut self) -> &mut T {
        self.store.get_or_default::<T>(self.id.clone())
    }

    /// Emit a message that will be delivered to the application's `update` function.
    pub fn emit(&mut self, message: M) {
        self.messages.push(message);
        self.handled = true;
    }

    /// The current widget path in the tree.
    pub fn path(&self) -> &WidgetPath {
        &self.path
    }

    /// The stable identity for the current widget.
    pub fn id(&self) -> &WidgetId {
        &self.id
    }

    /// Create a read-only measurement context at the current event target path.
    ///
    /// This is available when events are dispatched by the runtime. Manually
    /// constructed contexts may not have theme/runtime metadata attached.
    pub fn measure_ctx(&self) -> Option<MeasureCtx<'_>> {
        Some(MeasureCtx {
            store: &*self.store,
            theme: self.theme?,
            current_path: self.path.clone(),
            current_id: self.id.clone(),
            stable_scope_id: self.stable_scope_id.clone(),
            now: self.now,
            frame: self.frame,
            visual_capabilities: self.visual_capabilities,
        })
    }

    /// The globally focused path.
    pub fn focused_path(&self) -> Option<&WidgetPath> {
        self.focused_path.as_ref()
    }

    /// The current routing phase.
    pub fn phase(&self) -> EventPhase {
        self.phase
    }

    /// Whether an earlier handler in the current routing pass already handled this event.
    pub fn was_handled(&self) -> bool {
        self.already_handled
    }

    /// The last rendered bounds for the current widget.
    pub fn bounds(&self) -> Option<Area> {
        self.geometry.get(&self.path).copied()
    }

    /// The last rendered bounds for an arbitrary widget path.
    pub fn bounds_for(&self, path: &WidgetPath) -> Option<Area> {
        self.geometry.get(path).copied()
    }

    /// Absolute terminal mouse coordinates for a mouse event.
    pub fn mouse_position(&self, event: &Event) -> Option<(u16, u16)> {
        match event {
            Event::Mouse(mouse) => Some((mouse.column, mouse.row)),
            _ => None,
        }
    }

    /// Mouse coordinates relative to the current widget bounds.
    pub fn local_mouse_position(&self, event: &Event) -> Option<(u16, u16)> {
        let (x, y) = self.mouse_position(event)?;
        let bounds = self.bounds()?;

        if x < bounds.x()
            || x >= bounds.x().saturating_add(bounds.width())
            || y < bounds.y()
            || y >= bounds.y().saturating_add(bounds.height())
        {
            return None;
        }

        Some((x.saturating_sub(bounds.x()), y.saturating_sub(bounds.y())))
    }

    /// Mark the event as handled.
    pub fn set_handled(&mut self) {
        self.handled = true;
    }

    /// Stop event propagation after the current handler returns.
    pub fn stop_propagation(&mut self) {
        self.handled = true;
        self.stop_propagation = true;
    }

    /// Request focus for the current widget.
    pub fn request_focus_self(&mut self) {
        self.request_focus(self.path.clone());
    }

    /// Request focus for a specific widget path.
    pub fn request_focus(&mut self, path: WidgetPath) {
        self.handled = true;
        self.focus_request = Some(FocusRequest::Set(path));
    }

    /// Clear the current focus.
    pub fn clear_focus(&mut self) {
        self.handled = true;
        self.focus_request = Some(FocusRequest::Clear);
    }

    /// Move focus to the next global focus target.
    pub fn focus_next(&mut self) {
        self.handled = true;
        self.focus_request = Some(FocusRequest::Next);
    }

    /// Move focus to the previous global focus target.
    pub fn focus_prev(&mut self) {
        self.handled = true;
        self.focus_request = Some(FocusRequest::Prev);
    }

    /// Move focus to the next target within the given scope.
    pub fn focus_next_in_scope(&mut self, scope: Option<WidgetPath>) {
        self.handled = true;
        self.focus_request = Some(FocusRequest::NextInScope(scope));
    }

    /// Move focus to the previous target within the given scope.
    pub fn focus_prev_in_scope(&mut self, scope: Option<WidgetPath>) {
        self.handled = true;
        self.focus_request = Some(FocusRequest::PrevInScope(scope));
    }

    pub fn finish(self) -> EventOutcome {
        EventOutcome {
            handled: self.handled,
            stop_propagation: self.stop_propagation,
            focus_request: self.focus_request,
        }
    }
}

// ---------------------------------------------------------------------------
// WidgetStore – persistent state store across view() rebuilds
// ---------------------------------------------------------------------------

/// Stores widget-internal persistent state (cursor positions, scroll offsets, etc.)
/// keyed by a widget's stable [`WidgetId`].
///
/// State survives `view()` rebuilds because the store lives in the [`AppRuntime`],
/// not inside widget instances.
#[derive(Default)]
pub struct WidgetStore {
    states: HashMap<WidgetId, Box<dyn Any>>,
    component_types: HashMap<WidgetId, TypeId>,
}

impl WidgetStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Type of the mounted component, even when it has not allocated state.
    pub fn component_type(&self, id: impl Into<WidgetId>) -> Option<TypeId> {
        self.component_types.get(&id.into()).copied()
    }

    pub(crate) fn mount(&mut self, id: &WidgetId, component_type: TypeId) -> Lifecycle {
        if self.component_type(id) == Some(component_type) {
            Lifecycle::Sync
        } else {
            self.states.remove(id);
            self.component_types.insert(id.clone(), component_type);
            Lifecycle::Mount
        }
    }

    pub(crate) fn extract(&mut self, mut predicate: impl FnMut(&WidgetId) -> bool) -> Self {
        let ids: Vec<_> = self
            .component_types
            .keys()
            .filter(|id| predicate(id))
            .cloned()
            .collect();
        let mut extracted = Self::new();
        for id in ids {
            if let Some(state) = self.states.remove(&id) {
                extracted.states.insert(id.clone(), state);
            }
            if let Some(component_type) = self.component_types.remove(&id) {
                extracted.component_types.insert(id, component_type);
            }
        }
        extracted
    }

    /// Read state for the given widget id. Returns `None` if no state is stored.
    pub fn get<T: 'static>(&self, id: impl Into<WidgetId>) -> Option<&T> {
        let id = id.into();
        self.states.get(&id)?.downcast_ref()
    }

    /// Get or insert a default state for the given widget id.
    pub fn get_or_default<T: Default + 'static>(&mut self, id: impl Into<WidgetId>) -> &mut T {
        let id = id.into();
        self.states
            .entry(id.clone())
            .or_insert_with(|| Box::new(T::default()))
            .downcast_mut()
            .unwrap_or_else(|| panic!("WidgetStore state type mismatch at {id:?}: requested {}. A component currently has one state slot; use an aggregate state type.", std::any::type_name::<T>()))
    }

    /// Apply a function to all states of type T. Used for frame-level resets.
    pub fn for_each_state_mut<T: 'static, F>(&mut self, mut f: F)
    where
        F: FnMut(&WidgetId, &mut T),
    {
        for (id, state) in self.states.iter_mut() {
            if let Some(s) = state.downcast_mut::<T>() {
                f(id, s);
            }
        }
    }

    /// Remove entries whose widget ids are not in the active set.
    /// Called after building the widget tree to clean up stale state.
    pub fn retain_active<F>(&mut self, mut is_active: F)
    where
        F: FnMut(&WidgetId) -> bool,
    {
        self.states.retain(|id, _| is_active(id));
        self.component_types.retain(|id, _| is_active(id));
    }
}

// ---------------------------------------------------------------------------
// IntoWidget – ergonomic conversion for the builder API
// ---------------------------------------------------------------------------

/// Trait for converting types into boxed widgets, used by container `.child()` methods.
pub trait IntoWidget<M> {
    fn into_widget(self) -> Box<dyn Widget<M>>;
}

impl<M, W: Widget<M> + 'static> IntoWidget<M> for W {
    fn into_widget(self) -> Box<dyn Widget<M>> {
        Box::new(self)
    }
}
