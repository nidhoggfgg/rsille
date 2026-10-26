use std::cell::RefCell;
use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::render::area::Size;
use crate::render::chunk::Chunk;
use crate::render::{Draw, DrawErr, InlineMouseMode, InputCapabilities, KeyboardMode, Update};
use crossterm::event::Event;

use super::{Clock, Executor};
use crate::tui::animation::{
    AnimationCtx, AnimationStore, MotionPolicy, TimelineFrame, TransitionEffect,
};
use crate::tui::effect::{
    run_task_attempt, CancellationToken, Effect, Task, TaskContext, TaskEvent, TaskEventSender,
    TaskId, TaskState, TaskStatus, UpdateCtx,
};
use crate::tui::event::{
    InputEvent, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use crate::tui::focus::FocusManager;
use crate::tui::layout::{
    AxisLimit, Invalidation, LayoutCache, LayoutGeometry, LayoutStats, SizeProposal,
};
use crate::tui::scope::{ResourceKey, Scope};
use crate::tui::shell::{CommandMatch, CommandRouter, HotkeyRegistry};
use crate::tui::style::Theme;
use crate::tui::subscription::Subscription;
use crate::tui::widget::{
    EventCtx, EventPhase, FocusRequest, HitRegion, Lifecycle, MeasureCtx, RenderCtx, StateCtx,
    Widget, WidgetId, WidgetKey, WidgetPath, WidgetStore,
};
use crate::tui::widgets::visual::TerminalVisualCapabilities;
use crate::tui::WidgetResult;

use crate::tui::app::{
    App, AppExit, EventHandler, ExitReason, FrameHandler, FrameInfo, InputHandler, Mode,
    QuitBehavior, SubscriptionResolver, ThemeResolver,
};

struct TickRuntime<M> {
    interval: Duration,
    next_fire_at: Instant,
    handler: EventHandler<M>,
}

#[derive(Debug)]
struct FrameRuntime {
    started_at: Option<Instant>,
    last_frame_at: Option<Instant>,
    frame: u64,
}

impl FrameRuntime {
    fn new() -> Self {
        Self {
            started_at: None,
            last_frame_at: None,
            frame: 0,
        }
    }

    fn next(&mut self, now: Instant) -> FrameInfo {
        let started_at = *self.started_at.get_or_insert(now);
        let delta = self
            .last_frame_at
            .map(|last| now.saturating_duration_since(last))
            .unwrap_or(Duration::ZERO);
        let info = FrameInfo {
            now,
            delta,
            since_start: now.saturating_duration_since(started_at),
            frame: self.frame,
        };

        self.last_frame_at = Some(now);
        self.frame = self.frame.wrapping_add(1);
        info
    }
}

pub(crate) trait UpdateDriver<State, M> {
    fn handle(&mut self, state: &mut State, message: M, ctx: &mut UpdateCtx<M>);
}

pub(crate) struct SimpleUpdate<F>(pub(crate) F);

impl<State, M, F> UpdateDriver<State, M> for SimpleUpdate<F>
where
    F: Fn(&mut State, M),
{
    fn handle(&mut self, state: &mut State, message: M, _ctx: &mut UpdateCtx<M>) {
        (self.0)(state, message);
    }
}

impl<State, M, F> UpdateDriver<State, M> for F
where
    F: Fn(&mut State, M, &mut UpdateCtx<M>),
{
    fn handle(&mut self, state: &mut State, message: M, ctx: &mut UpdateCtx<M>) {
        (self)(state, message, ctx);
    }
}

// ---------------------------------------------------------------------------
// AppRuntime – implements Draw + Update for the render event loop
// ---------------------------------------------------------------------------

/// Thread-safe application ingress. Messages are FIFO and wake the UI immediately.
/// The queue is unbounded; each runtime turn processes at most 256 messages.
pub struct AppHandle<M> {
    sender: TaskEventSender<M>,
}

impl<M> Clone for AppHandle<M> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<M> AppHandle<M> {
    /// Returns the original message if the runtime has been dropped.
    pub fn send(&self, message: M) -> Result<(), mpsc::SendError<M>> {
        self.sender.send_message(message)
    }
}

type RuntimeUpdate<State, M> = Box<dyn Fn(&mut State, M, &mut UpdateCtx<M>)>;
type RuntimeView<State, M> = Box<dyn Fn(&State) -> Box<dyn Widget<M>>>;

/// A single-step application using the same core as the terminal adapter.
/// Building and stepping do not access a TTY. `run` enters the terminal adapter.
pub struct Runtime<State, M> {
    core: AppRuntime<State, RuntimeUpdate<State, M>, RuntimeView<State, M>, M>,
    buffer: crate::render::buffer::Buffer,
    needs_draw: bool,
    mouse_capture: bool,
    inline_mouse_mode: InlineMouseMode,
}

#[derive(Clone, Copy, Debug)]
pub struct StepResult {
    pub rendered: bool,
    pub should_quit: bool,
    /// Deadline in the injected clock's time domain, or `None` when idle.
    pub next_update_at: Option<Instant>,
}

impl<State, M: Send + 'static> Runtime<State, M> {
    pub(crate) fn from_app<F, V, W>(app: App<State, M>, update: F, view: V) -> Self
    where
        F: Fn(&mut State, M, &mut UpdateCtx<M>) + 'static,
        V: Fn(&State) -> W + 'static,
        W: Widget<M> + 'static,
    {
        let update: RuntimeUpdate<State, M> = Box::new(update);
        let view: RuntimeView<State, M> = Box::new(move |state| Box::new(view(state)));
        let inline = app.mode == Mode::Inline;
        Runtime {
            mouse_capture: app.mouse_capture,
            inline_mouse_mode: app.inline_mouse_mode,
            core: AppRuntime::new(app, update, view, inline, 50),
            buffer: crate::render::buffer::Buffer::new((80, 24).into()),
            needs_draw: true,
        }
    }

    /// Enter the terminal adapter after obtaining a handle with `handle()`.
    pub fn run(self) -> WidgetResult<AppExit<State>> {
        self.core
            .run_terminal(self.mouse_capture, self.inline_mouse_mode)
    }

    /// Enter the inline terminal adapter using this runtime and its message queue.
    pub fn run_inline(mut self) -> WidgetResult<AppExit<State>> {
        self.core.inline_mode = true;
        self.core
            .run_terminal(self.mouse_capture, self.inline_mouse_mode)
    }

    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.resize((width, height).into());
        self
    }

    pub fn resize(&mut self, size: Size) {
        self.buffer = crate::render::buffer::Buffer::new(size);
        self.needs_draw = true;
    }

    pub fn handle(&self) -> AppHandle<M> {
        AppHandle {
            sender: self.core.task_sender.clone(),
        }
    }

    pub fn state(&self) -> &State {
        self.core.state.as_ref().expect("runtime state")
    }

    pub fn frame(&self) -> &crate::render::buffer::Buffer {
        &self.buffer
    }

    pub fn focused_path(&self) -> Option<&WidgetPath> {
        self.core.focus.current_path()
    }

    pub fn focused_key(&self) -> Option<&str> {
        let path = self.focused_path()?;
        match path.as_slice().last() {
            Some(WidgetKey::Named(key)) => Some(key),
            None => self.core.cached_tree.as_ref()?.key(),
            _ => None,
        }
    }

    pub fn bounds(&self, path: &WidgetPath) -> Option<crate::render::area::Area> {
        self.core.geometry.borrow().get(path).copied()
    }

    /// Target, displayed allocation, visible clip and hit area from the last paint.
    pub fn layout_geometry(&self, path: &WidgetPath) -> Option<LayoutGeometry> {
        self.core.store.layout_geometry.borrow().get(path).copied()
    }

    /// Cumulative layout computations and cache hits for profiling/tests.
    pub fn layout_stats(&self) -> LayoutStats {
        self.core.layouts.stats()
    }

    /// Refresh layout (and paint) on the next step, including custom external dependencies.
    pub fn request_layout(&mut self) {
        self.core.layouts.clear();
        self.needs_draw = true;
        self.core.wake.wake();
    }

    /// Paint the existing view and reuse its layout on the next step.
    pub fn request_paint(&mut self) {
        self.needs_draw = true;
        self.core.wake.wake();
    }

    pub fn widget_state<T: 'static>(&self, id: &WidgetId) -> Option<&T> {
        self.core.store.get(id.clone())
    }

    pub fn tasks(&self) -> Vec<TaskStatus> {
        self.core.task_statuses()
    }

    pub fn next_update_at(&self) -> Option<Instant> {
        self.core.next_update_at()
    }

    pub fn wake_handle(&self) -> crate::render::WakeHandle {
        self.core.wake.clone()
    }

    /// Supply a simulated input backend's capabilities for headless testing.
    /// Delivers the same startup notification as terminal setup, on the next step.
    pub fn set_input_capabilities(&mut self, capabilities: InputCapabilities) {
        self.core.on_input_capabilities(capabilities);
    }

    /// Stop timers and signal cancellation without waiting for blocking jobs.
    pub fn shutdown(&mut self) {
        self.core.shutdown();
    }

    /// Cancel resources and recover state without cloning it.
    pub fn into_exit(mut self) -> AppExit<State> {
        self.core.take_exit()
    }

    pub fn exit_reason(&self) -> Option<ExitReason> {
        self.core.exit_reason
    }

    /// Use the existing Tokio runtime rather than creating a nested runtime.
    pub async fn run_async(self) -> WidgetResult<AppExit<State>> {
        self.core
            .run_terminal_async(self.mouse_capture, self.inline_mouse_mode)
            .await
    }

    /// Dispatch input, drain messages, advance time-dependent work and draw.
    /// Pass an empty slice to drive messages or an explicitly advanced clock.
    pub fn step(&mut self, events: &[Event]) -> Result<StepResult, DrawErr> {
        if self.core.should_quit {
            return Ok(StepResult {
                rendered: false,
                should_quit: true,
                next_update_at: None,
            });
        }
        // Establish initial focus and hit geometry before dispatching input.
        let mut rendered = false;
        if self.needs_draw {
            self.draw_frame()?;
            rendered = true;
        }
        for event in events {
            if matches!(event, Event::Key(key) if key.kind == crate::tui::event::KeyEventKind::Release)
            {
                continue;
            }
            if let Event::Resize(width, height) = event {
                self.resize((*width, *height).into());
            }
        }
        self.core.on_events(events)?;
        let changed = self.core.update()?;
        if !self.core.should_quit && (changed || !events.is_empty() || self.needs_draw) {
            self.draw_frame()?;
            rendered = true;
        }
        Ok(StepResult {
            rendered,
            should_quit: self.core.should_quit,
            next_update_at: self.core.next_update_at(),
        })
    }

    fn draw_frame(&mut self) -> Result<(), DrawErr> {
        self.buffer.clear();
        let area = crate::render::area::Area::new((0, 0).into(), self.buffer.size());
        let chunk = Chunk::new(&mut self.buffer, area)?;
        self.core.draw(chunk)?;
        self.needs_draw = false;
        Ok(())
    }
}

struct TaskRecord<M> {
    status: TaskStatus,
    lease: ScopeLease,
    accept_results: bool,
    cancellation: CancellationToken,
    task: Task<M>,
    deadline: Option<Instant>,
    retry_at: Option<Instant>,
}

#[derive(Clone)]
struct ScopeLease {
    scope: Scope,
    generation: u64,
}

struct MountedScope {
    owner: WidgetId,
    generation: u64,
}

enum QueuedMessage<M> {
    Plain(M),
    Scoped {
        lease: ScopeLease,
        message: M,
    },
    Task {
        id: TaskId,
        attempt: u32,
        message: M,
    },
}

struct MessageQueue<M>(VecDeque<QueuedMessage<M>>);

impl<M> MessageQueue<M> {
    fn new() -> Self {
        Self(VecDeque::new())
    }
    fn push_back(&mut self, message: M) {
        self.0.push_back(QueuedMessage::Plain(message));
    }
    fn push_scoped(&mut self, lease: ScopeLease, message: M) {
        self.0.push_back(QueuedMessage::Scoped { lease, message });
    }
    fn extend(&mut self, messages: impl IntoIterator<Item = M>) {
        self.0
            .extend(messages.into_iter().map(QueuedMessage::Plain));
    }
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    fn pop_front(&mut self) -> Option<QueuedMessage<M>> {
        self.0.pop_front()
    }
    fn contains_task(&self, task_id: TaskId) -> bool {
        self.0
            .iter()
            .any(|message| matches!(message, QueuedMessage::Task { id, .. } if *id == task_id))
    }
}

pub(crate) struct AppRuntime<State, U, V, M> {
    state: Option<State>,
    theme: Theme,
    theme_resolver: Option<ThemeResolver<State>>,
    subscriptions: Option<SubscriptionResolver<State, M>>,
    subscription_tasks: HashMap<ResourceKey, TaskId>,
    scopes: HashMap<Scope, MountedScope>,
    next_scope_generation: u64,
    update_fn: U,
    view_fn: V,
    store: WidgetStore,
    animation_store: RefCell<AnimationStore>,
    geometry: RefCell<HashMap<WidgetPath, crate::render::area::Area>>,
    hit_regions: RefCell<Vec<HitRegion>>,
    layouts: LayoutCache,
    focus: FocusManager,
    cached_tree: Option<Box<dyn Widget<M>>>,
    previous_tree: Option<Box<dyn Widget<M>>>,
    previous_geometry: HashMap<WidgetPath, crate::render::area::Area>,
    exiting_visuals: Vec<ExitingVisual<M>>,
    messages: MessageQueue<M>,
    should_quit: bool,
    exit_reason: Option<ExitReason>,
    quit_behavior: QuitBehavior,
    global_key_handlers: HashMap<KeyCode, Box<dyn Fn() -> M>>,
    input_handler: Option<InputHandler<M>>,
    keyboard_mode: KeyboardMode,
    focus_events: bool,
    hotkeys: HotkeyRegistry<M>,
    command_router: Option<CommandRouter<M>>,
    command_resolver: Option<crate::tui::app::CommandResolver<State, M>>,
    tick_handlers: Vec<TickRuntime<M>>,
    frame_handlers: Vec<FrameHandler<M>>,
    frame_runtime: FrameRuntime,
    animation_epoch: Instant,
    animation_frame: u64,
    render_frame: u64,
    pointer_capture: Option<WidgetPath>,
    motion_policy: MotionPolicy,
    visual_capabilities: TerminalVisualCapabilities,
    inline_mode: bool,
    inline_max_height: u16,
    task_sender: TaskEventSender<M>,
    task_receiver: Receiver<TaskEvent<M>>,
    debounces: HashMap<ResourceKey, (Instant, ScopeLease, M)>,
    timers: Vec<(Instant, ScopeLease, M)>,
    wake: crate::render::WakeHandle,
    clock: Arc<dyn Clock>,
    executor: Arc<dyn Executor>,
    next_animation_at: Option<Instant>,
    next_task_id: u64,
    tasks: HashMap<TaskId, TaskRecord<M>>,
    task_keys: HashMap<ResourceKey, TaskId>,
    task_order: Vec<TaskId>,
}

struct ExitingVisual<M> {
    store: WidgetStore,
    animations: AnimationStore,
    root: Box<dyn Widget<M>>,
    nodes: Vec<ExitingNode>,
}

struct ExitingNode {
    path: WidgetPath,
    bounds: crate::render::area::Area,
}

/// The same identity walk is used for visible and explicitly retained children.
/// Focus, event routing and animation continue to traverse visible children only.
fn visit_widgets<M>(
    widget: &(dyn Widget<M> + 'static),
    visitor: &mut impl FnMut(&(dyn Widget<M> + 'static), &WidgetPath, &WidgetId),
) {
    fn walk<M>(
        widget: &(dyn Widget<M> + 'static),
        path: &WidgetPath,
        id: &WidgetId,
        scope: &WidgetId,
        visitor: &mut impl FnMut(&(dyn Widget<M> + 'static), &WidgetPath, &WidgetId),
    ) {
        visitor(widget, path, id);
        for (index, child) in widget
            .children()
            .iter()
            .chain(widget.retained_children())
            .enumerate()
        {
            let key = WidgetKey::for_child(index, child.as_ref());
            let (child_id, child_scope) = WidgetId::for_child(id, scope, &key);
            walk(
                child.as_ref(),
                &path.child(key),
                &child_id,
                &child_scope,
                visitor,
            );
        }
    }
    walk(
        widget,
        &WidgetPath::root(),
        &WidgetId::root(),
        &WidgetId::root(),
        visitor,
    );
}

fn sync_widget_states<M: 'static>(widget: &(dyn Widget<M> + 'static), store: &mut WidgetStore) {
    reconcile_widget_states(widget, store, true);
}

fn reconcile_widget_states<M: 'static>(
    widget: &(dyn Widget<M> + 'static),
    store: &mut WidgetStore,
    sync_existing: bool,
) {
    visit_widgets(widget, &mut |widget, path, id| {
        let lifecycle = store.mount(id, widget.widget_type());
        if lifecycle == Lifecycle::Sync && !sync_existing {
            return;
        }
        widget.lifecycle(&mut StateCtx::new(
            store,
            path.clone(),
            id.clone(),
            lifecycle,
        ));
    });
}

fn unmount_widget_states<M>(widget: &(dyn Widget<M> + 'static), store: &mut WidgetStore) {
    visit_widgets(widget, &mut |widget, path, id| {
        if store.component_type(id).is_some() {
            widget.lifecycle(&mut StateCtx::new(
                store,
                path.clone(),
                id.clone(),
                Lifecycle::Unmount,
            ));
        }
    });
    store.retain_active(|_| false);
}

impl<State, U, V, M> Drop for AppRuntime<State, U, V, M> {
    fn drop(&mut self) {
        if let Some(tree) = self.cached_tree.as_ref().or(self.previous_tree.as_ref()) {
            unmount_widget_states(tree.as_ref(), &mut self.store);
        }
        // Blocking jobs may outlive the UI. Signal cancellation without joining
        // them; dropping the receiver rejects all later messages.
        for record in self.tasks.values() {
            record.cancellation.cancel();
        }
    }
}

fn exit_display_area(
    area: crate::render::area::Area,
    frames: &[TimelineFrame],
) -> crate::render::area::Area {
    frames
        .iter()
        .fold(area, |display_area, frame| match frame.transition.effect {
            TransitionEffect::Collapse | TransitionEffect::Expand => {
                vertical_area_for_exit(display_area, 1.0 - frame.progress)
            }
            TransitionEffect::ScaleFromCenter => {
                scale_area_for_exit(display_area, 1.0 - frame.progress)
            }
            TransitionEffect::Layout(_)
            | TransitionEffect::Fade
            | TransitionEffect::BorderEmphasis => display_area,
        })
}

fn vertical_area_for_exit(
    area: crate::render::area::Area,
    progress: f64,
) -> crate::render::area::Area {
    let height = ((area.height() as f64) * progress.clamp(0.0, 1.0)).round() as u16;
    crate::render::area::Area::new(area.pos(), (area.width(), height).into())
}

fn scale_area_for_exit(
    area: crate::render::area::Area,
    progress: f64,
) -> crate::render::area::Area {
    let progress = progress.clamp(0.0, 1.0);
    let width = ((area.width() as f64) * progress).round() as u16;
    let height = ((area.height() as f64) * progress).round() as u16;
    let x = area.x() + area.width().saturating_sub(width) / 2;
    let y = area.y() + area.height().saturating_sub(height) / 2;

    crate::render::area::Area::new((x, y).into(), (width, height).into())
}

fn inline_available_height(terminal_height: u16, force_follow_viewport: bool) -> u16 {
    if force_follow_viewport {
        terminal_height.saturating_sub(1).max(1)
    } else {
        terminal_height
    }
}

fn mouse_position(event: &Event) -> Option<(u16, u16)> {
    match event {
        Event::Mouse(mouse) => Some((mouse.column, mouse.row)),
        _ => None,
    }
}

fn hit_test_path(hit_regions: &[HitRegion], x: u16, y: u16) -> Option<WidgetPath> {
    hit_regions
        .iter()
        .rev()
        .find(|region| region.contains(x, y))
        .map(|region| region.path.clone())
}

fn mouse_target_path(
    event: &Event,
    hit_regions: &[HitRegion],
    pointer_capture: Option<&WidgetPath>,
) -> Option<WidgetPath> {
    let Event::Mouse(mouse) = event else {
        return None;
    };

    let hit_path = hit_test_path(hit_regions, mouse.column, mouse.row);
    match mouse.kind {
        MouseEventKind::Drag(_) | MouseEventKind::Up(_) => pointer_capture.cloned().or(hit_path),
        _ => hit_path,
    }
}

impl<State, U, V, M: Send + 'static> AppRuntime<State, U, V, M> {
    pub(crate) fn new(
        app: App<State, M>,
        update: U,
        view: V,
        inline_mode: bool,
        inline_max_height: u16,
    ) -> Self {
        let App {
            state,
            mode: _,
            theme,
            theme_resolver,
            subscriptions,
            global_key_handlers,
            input_handler,
            keyboard_mode,
            focus_events,
            hotkeys,
            command_router,
            command_resolver,
            tick_handlers,
            frame_handlers,
            quit_behavior,
            mouse_capture: _,
            inline_mouse_mode: _,
            motion_policy,
            visual_capabilities,
            clock,
            executor,
        } = app;

        let now = clock.now();
        let (sender, task_receiver) = mpsc::channel();
        let wake = crate::render::WakeHandle::new();
        let task_sender = TaskEventSender::new(sender, wake.clone());

        AppRuntime {
            state: Some(state),
            theme,
            theme_resolver,
            subscriptions,
            subscription_tasks: HashMap::new(),
            scopes: HashMap::new(),
            next_scope_generation: 0,
            update_fn: update,
            view_fn: view,
            store: WidgetStore::new(),
            animation_store: RefCell::new(AnimationStore::new()),
            geometry: RefCell::new(HashMap::new()),
            hit_regions: RefCell::new(Vec::new()),
            layouts: LayoutCache::default(),
            focus: FocusManager::new(),
            cached_tree: None,
            previous_tree: None,
            previous_geometry: HashMap::new(),
            exiting_visuals: Vec::new(),
            messages: MessageQueue::new(),
            should_quit: false,
            exit_reason: None,
            quit_behavior,
            global_key_handlers,
            input_handler,
            keyboard_mode,
            focus_events,
            hotkeys,
            command_router,
            command_resolver,
            tick_handlers: tick_handlers
                .into_iter()
                .map(|tick| TickRuntime {
                    interval: tick.interval,
                    next_fire_at: now + tick.interval,
                    handler: tick.handler,
                })
                .collect(),
            frame_handlers: frame_handlers
                .into_iter()
                .map(|frame| frame.handler)
                .collect(),
            frame_runtime: FrameRuntime::new(),
            animation_epoch: now,
            animation_frame: 0,
            render_frame: 0,
            pointer_capture: None,
            motion_policy,
            visual_capabilities,
            inline_mode,
            inline_max_height,
            task_sender,
            task_receiver,
            debounces: HashMap::new(),
            timers: Vec::new(),
            wake,
            clock,
            executor,
            next_animation_at: None,
            next_task_id: 0,
            tasks: HashMap::new(),
            task_keys: HashMap::new(),
            task_order: Vec::new(),
        }
    }

    const MAX_TRACKED_TASKS: usize = 256;

    fn ensure_tree(&mut self)
    where
        V: Fn(&State) -> Box<dyn Widget<M>>,
    {
        self.ensure_tree_with_sync(true);
    }

    fn ensure_tree_with_sync(&mut self, sync_existing: bool)
    where
        V: Fn(&State) -> Box<dyn Widget<M>>,
    {
        if self.cached_tree.is_some() {
            return;
        }
        self.layouts.clear();
        if let Some(resolver) = &self.command_resolver {
            self.command_router = Some(resolver(self.state.as_ref().expect("runtime state")));
        }
        let tree = (self.view_fn)(self.state.as_ref().expect("runtime state"));
        // Validate before modifying state: even keys below separate unkeyed
        // wrappers share the nearest named ancestor's namespace.
        let mut mounted = HashMap::new();
        visit_widgets(tree.as_ref(), &mut |widget, path, id| {
            if let Some((previous, _)) =
                mounted.insert(id.clone(), (path.clone(), widget.widget_type()))
            {
                panic!("Duplicate widget key/identity {id:?}: paths {previous:?} and {path:?}. Keys must be unique within the nearest keyed ancestor (including retained children).");
            }
        });

        let mut removed = HashSet::new();
        let mut root_key_changed = false;
        if let Some(previous) = self.previous_tree.take() {
            // The root always uses WidgetId::root(); its explicit key still
            // defines a lifetime boundary for the entire application view.
            root_key_changed = previous.key() != tree.key();
            visit_widgets(previous.as_ref(), &mut |widget, path, id| {
                if root_key_changed
                    || mounted.get(id).map(|(_, ty)| *ty) != Some(widget.widget_type())
                {
                    removed.insert(id.clone());
                    widget.lifecycle(&mut StateCtx::new(
                        &mut self.store,
                        path.clone(),
                        id.clone(),
                        Lifecycle::Unmount,
                    ));
                }
            });
            for id in &removed {
                self.focus.forget(id);
            }
            if let Some(captured) = self.pointer_capture.as_ref() {
                // Capture follows identity across reordering, but never crosses
                // an unmount/type change into a replacement at the same path.
                let mut captured_id = None;
                visit_widgets(previous.as_ref(), &mut |_, path, id| {
                    if path == captured {
                        captured_id = Some(id.clone());
                    }
                });
                self.pointer_capture = captured_id
                    .filter(|id| !removed.contains(id))
                    .and_then(|id| mounted.get(&id).map(|(path, _)| path.clone()));
            }
            let geometry = std::mem::take(&mut self.previous_geometry);
            self.start_exit_visuals(previous, geometry, &removed, self.animation_time());
        }
        self.reconcile_scopes(tree.as_ref(), &removed, root_key_changed);
        self.store.retain_active(|id| mounted.contains_key(id));
        reconcile_widget_states(tree.as_ref(), &mut self.store, sync_existing);
        self.focus.rebuild(tree.as_ref());
        if let Some(router) = self.command_router.as_mut() {
            router.sync_focus(self.focus.current_id());
        }
        if self
            .pointer_capture
            .as_ref()
            .is_some_and(|path| !self.focus.live_paths().contains(path))
        {
            self.pointer_capture = None;
        }
        self.cached_tree = Some(tree);
        self.reconcile_subscriptions();
        self.prune_live_animations();
    }

    fn lease(&self, scope: &Scope) -> Option<ScopeLease> {
        if *scope == Scope::root() {
            Some(ScopeLease {
                scope: scope.clone(),
                generation: 0,
            })
        } else {
            self.scopes.get(scope).map(|s| ScopeLease {
                scope: scope.clone(),
                generation: s.generation,
            })
        }
    }

    fn lease_is_live(&self, lease: &ScopeLease) -> bool {
        lease.scope == Scope::root()
            || self
                .scopes
                .get(&lease.scope)
                .is_some_and(|live| live.generation == lease.generation)
    }

    fn reconcile_scopes(
        &mut self,
        tree: &(dyn Widget<M> + 'static),
        removed: &HashSet<WidgetId>,
        reset: bool,
    ) {
        let mut paths = HashMap::<WidgetPath, Scope>::new();
        let mut next = HashMap::new();
        visit_widgets(tree, &mut |widget, path, id| {
            let mut parent = path.clone();
            parent.pop();
            let mut scope = if path.is_empty() {
                Scope::root()
            } else {
                paths[&parent].clone()
            };
            if let Some(name) = widget.resource_scope() {
                let parent_continues = self.scopes.get(&scope).map(|s| s.generation)
                    == next.get(&scope).map(|s: &MountedScope| s.generation);
                scope = scope.child(name);
                let generation = self
                    .scopes
                    .get(&scope)
                    .filter(|s| {
                        !reset && parent_continues && s.owner == *id && !removed.contains(id)
                    })
                    .map(|s| s.generation)
                    .unwrap_or_else(|| {
                        self.next_scope_generation += 1;
                        self.next_scope_generation
                    });
                if next
                    .insert(
                        scope.clone(),
                        MountedScope {
                            owner: id.clone(),
                            generation,
                        },
                    )
                    .is_some()
                {
                    panic!("Duplicate resource scope {scope} at widget path {path:?}");
                }
            }
            paths.insert(path.clone(), scope);
        });
        self.scopes = next;
        let stale: Vec<_> = self
            .tasks
            .iter()
            .filter(|(_, r)| !self.lease_is_live(&r.lease))
            .map(|(&id, _)| id)
            .collect();
        for id in stale {
            self.cancel_task(id);
        }
        let scopes = &self.scopes;
        let live = |lease: &ScopeLease| {
            lease.scope == Scope::root()
                || scopes
                    .get(&lease.scope)
                    .is_some_and(|s| s.generation == lease.generation)
        };
        self.timers.retain(|(_, lease, _)| live(lease));
        self.debounces.retain(|_, (_, lease, _)| live(lease));
    }

    fn reconcile_subscriptions(&mut self) {
        fn collect<M>(
            source: Subscription<M>,
            scope: Scope,
            out: &mut Vec<(ResourceKey, Task<M>)>,
        ) {
            match source {
                Subscription::None => {}
                Subscription::Batch(sources) => {
                    for source in sources {
                        collect(source, scope.clone(), out);
                    }
                }
                Subscription::Run { key, mut task } => {
                    // Subscription identity is tracked separately from task keys.
                    task.key = None;
                    out.push((ResourceKey::new(scope, key), task));
                }
                Subscription::Scoped(inner, source) => collect(*source, scope.join(&inner), out),
            }
        }
        let Some(resolver) = &self.subscriptions else {
            return;
        };
        let mut desired = Vec::new();
        collect(
            resolver(self.state.as_ref().expect("runtime state")),
            Scope::root(),
            &mut desired,
        );
        let mut keys = HashSet::new();
        for (key, _) in &desired {
            assert!(
                keys.insert(key.clone()),
                "Duplicate subscription key {:?} in scope {}",
                key.key,
                key.scope
            );
        }
        let stale: Vec<_> = self
            .subscription_tasks
            .iter()
            .filter(|(key, id)| {
                !keys.contains(*key)
                    || self
                        .tasks
                        .get(id)
                        .is_none_or(|r| !self.lease_is_live(&r.lease))
            })
            .map(|(key, id)| (key.clone(), *id))
            .collect();
        for (key, id) in stale {
            self.subscription_tasks.remove(&key);
            self.cancel_task(id);
        }
        for (key, task) in desired {
            if self.subscription_tasks.contains_key(&key) {
                continue;
            }
            if let Some(lease) = self.lease(&key.scope) {
                // Subscription IDs and one-shot task keys are separate namespaces.
                let id = self.spawn_task(task, lease);
                self.subscription_tasks.insert(key, id);
            }
        }
    }

    fn prune_live_animations(&mut self) {
        let mut live_shared_ids = HashSet::new();
        if let Some(tree) = self.cached_tree.as_ref() {
            let mut path = WidgetPath::root();
            Self::collect_shared_transition_ids(tree.as_ref(), &mut path, &mut live_shared_ids);
        }
        // Hidden children keep widget state only. Exiting state and animations
        // are owned by the exit renderer, never by the live tree.
        let mut animation_store = self.animation_store.borrow_mut();
        animation_store.retain_active(|id| self.focus.live_ids().contains(id));
        animation_store.retain_timeline_paths(|path| self.focus.live_paths().contains(path));
        animation_store.retain_shared_layouts(|id| live_shared_ids.contains(id));
    }

    fn collect_shared_transition_ids(
        widget: &dyn Widget<M>,
        path: &mut WidgetPath,
        ids: &mut HashSet<String>,
    ) {
        if let Some(shared) = widget.shared_transition() {
            ids.insert(shared.id.clone());
        }

        for (index, child) in widget.children().iter().enumerate() {
            path.push(WidgetKey::for_child(index, child.as_ref()));
            Self::collect_shared_transition_ids(child.as_ref(), path, ids);
            path.pop();
        }
    }

    fn start_exit_visuals(
        &mut self,
        previous_tree: Box<dyn Widget<M>>,
        previous_geometry: HashMap<WidgetPath, crate::render::area::Area>,
        removed: &HashSet<WidgetId>,
        now: Instant,
    ) {
        let mut candidates = Vec::new();
        let mut surviving_paths = Vec::new();
        visit_widgets(previous_tree.as_ref(), &mut |_, path, id| {
            if !removed.contains(id) {
                surviving_paths.push(path.clone());
            }
        });
        visit_widgets(previous_tree.as_ref(), &mut |widget, path, id| {
            if !removed.contains(id) {
                return;
            }
            // A keyed descendant that moved elsewhere remains mounted. Do not
            // duplicate it in an ancestor exit or move its live state away.
            if surviving_paths.iter().any(|live| live.starts_with(path)) {
                return;
            }
            if let Some(timeline) = widget.presence().and_then(|presence| presence.exit.clone()) {
                if let Some(bounds) = previous_geometry.get(path).copied() {
                    candidates.push((path.clone(), bounds, timeline));
                }
            }
        });

        candidates.sort_by_key(|(path, _, _)| path.len());

        let mut removed_paths = Vec::new();
        visit_widgets(previous_tree.as_ref(), &mut |_, path, id| {
            if removed.contains(id) {
                removed_paths.push(path.clone());
            }
        });
        let mut animations = self
            .animation_store
            .borrow_mut()
            .extract(removed, &removed_paths);
        let store = self.store.extract(|id| removed.contains(id));
        let mut nodes: Vec<ExitingNode> = Vec::new();
        for (path, bounds, timeline) in candidates {
            if nodes.iter().any(|node| path.starts_with(&node.path)) {
                continue;
            }

            animations.track_timeline(
                &path,
                "exit",
                timeline.clone(),
                true,
                now,
                self.motion_policy,
            );
            nodes.push(ExitingNode { path, bounds });
        }

        if !nodes.is_empty() {
            self.exiting_visuals.push(ExitingVisual {
                store,
                animations,
                root: previous_tree,
                nodes,
            });
        }
    }

    fn sync_theme(&mut self) {
        if let Some(resolver) = self.theme_resolver.as_ref() {
            let theme = resolver(self.state.as_ref().expect("runtime state"));
            if self.theme != theme {
                self.theme = theme;
                self.layouts.clear();
                self.store.invalidation.request_paint();
            }
        }
    }

    fn build_event_route<'a>(
        tree: &'a dyn Widget<M>,
        target_path: Option<&WidgetPath>,
    ) -> Vec<(WidgetPath, WidgetId, WidgetId, &'a dyn Widget<M>)> {
        let mut route = vec![(WidgetPath::root(), WidgetId::root(), WidgetId::root(), tree)];
        let Some(target_path) = target_path else {
            return route;
        };

        let mut current_widget = tree;
        let mut current_path = WidgetPath::root();
        let mut current_id = WidgetId::root();
        let mut stable_scope_id = WidgetId::root();

        for key in target_path.as_slice() {
            let children = current_widget.children();
            let next = match key {
                WidgetKey::Index(idx) => children.get(*idx).map(|child| child.as_ref()),
                WidgetKey::Named(name) => children
                    .iter()
                    .find(|child| child.key() == Some(name.as_str()))
                    .map(|child| child.as_ref()),
            };

            let Some(next_widget) = next else {
                break;
            };

            current_path.push(key.clone());
            let (next_id, next_stable_scope_id) =
                WidgetId::for_child(&current_id, &stable_scope_id, key);
            current_id = next_id;
            stable_scope_id = next_stable_scope_id;
            route.push((
                current_path.clone(),
                current_id.clone(),
                stable_scope_id.clone(),
                next_widget,
            ));
            current_widget = next_widget;
        }

        route
    }

    fn dispatch_to_widget(
        widget: &dyn Widget<M>,
        event: &Event,
        store: &mut WidgetStore,
        messages: &mut MessageQueue<M>,
        path: WidgetPath,
        id: WidgetId,
        stable_scope_id: WidgetId,
        focused_path: Option<WidgetPath>,
        geometry: &HashMap<WidgetPath, crate::render::area::Area>,
        phase: EventPhase,
        already_handled: bool,
        theme: &Theme,
        now: Instant,
        frame: u64,
        visual_capabilities: TerminalVisualCapabilities,
    ) -> crate::tui::widget::EventOutcome {
        let mut emitted = Vec::new();
        let mut ctx = EventCtx::with_runtime(
            store,
            &mut emitted,
            path,
            id,
            stable_scope_id,
            focused_path,
            geometry,
            phase,
            already_handled,
            theme,
            now,
            frame,
            visual_capabilities,
        );
        widget.handle_event(event, &mut ctx);
        let outcome = ctx.finish();
        messages.extend(emitted);
        outcome
    }

    fn apply_focus_request(focus: &mut FocusManager, request: Option<FocusRequest>) {
        match request {
            Some(FocusRequest::Set(path)) => {
                focus.request_focus(&path);
            }
            Some(FocusRequest::Clear) => {
                focus.clear();
            }
            Some(FocusRequest::Next) => {
                focus.next();
            }
            Some(FocusRequest::Prev) => {
                focus.prev();
            }
            Some(FocusRequest::NextInScope(scope)) => {
                focus.next_in_scope(scope.as_ref());
            }
            Some(FocusRequest::PrevInScope(scope)) => {
                focus.prev_in_scope(scope.as_ref());
            }
            None => {}
        }
    }

    fn dispatch_widget_event(
        tree: &dyn Widget<M>,
        event: &Event,
        store: &mut WidgetStore,
        messages: &mut MessageQueue<M>,
        focus: &mut FocusManager,
        geometry: &HashMap<WidgetPath, crate::render::area::Area>,
        hit_regions: &[HitRegion],
        pointer_capture: Option<&WidgetPath>,
        theme: &Theme,
        now: Instant,
        frame: u64,
        visual_capabilities: TerminalVisualCapabilities,
    ) -> bool {
        let mut target_path = match mouse_position(event) {
            Some(_) => mouse_target_path(event, hit_regions, pointer_capture),
            None => focus.current_path().cloned(),
        };
        if let Some(modal) = focus.modal_path() {
            if !target_path
                .as_ref()
                .is_some_and(|path| path.starts_with(modal))
            {
                target_path = Some(modal.clone());
            }
        }
        let mut route = Self::build_event_route(tree, target_path.as_ref());
        if let Some(modal) = focus.modal_path() {
            route.retain(|(path, _, _, _)| path.starts_with(modal));
        }
        let mut handled = false;
        let focused_snapshot = focus.current_path().cloned();
        let ancestor_len = route.len().saturating_sub(1);

        for (path, id, stable_scope_id, widget) in route.iter().take(ancestor_len) {
            let outcome = Self::dispatch_to_widget(
                *widget,
                event,
                store,
                messages,
                path.clone(),
                id.clone(),
                stable_scope_id.clone(),
                focused_snapshot.clone(),
                geometry,
                EventPhase::Capture,
                handled,
                theme,
                now,
                frame,
                visual_capabilities,
            );
            handled |= outcome.handled;
            Self::apply_focus_request(focus, outcome.focus_request);
            if outcome.stop_propagation {
                return handled;
            }
        }

        if let Some((path, id, stable_scope_id, widget)) = route.last() {
            let outcome = Self::dispatch_to_widget(
                *widget,
                event,
                store,
                messages,
                path.clone(),
                id.clone(),
                stable_scope_id.clone(),
                focused_snapshot.clone(),
                geometry,
                EventPhase::Target,
                handled,
                theme,
                now,
                frame,
                visual_capabilities,
            );
            handled |= outcome.handled;
            Self::apply_focus_request(focus, outcome.focus_request);
            if outcome.stop_propagation {
                return handled;
            }
        }

        for (path, id, stable_scope_id, widget) in route.iter().take(ancestor_len).rev() {
            let outcome = Self::dispatch_to_widget(
                *widget,
                event,
                store,
                messages,
                path.clone(),
                id.clone(),
                stable_scope_id.clone(),
                focused_snapshot.clone(),
                geometry,
                EventPhase::Bubble,
                handled,
                theme,
                now,
                frame,
                visual_capabilities,
            );
            handled |= outcome.handled;
            Self::apply_focus_request(focus, outcome.focus_request);
            if outcome.stop_propagation {
                return handled;
            }
        }

        handled
    }

    fn animate_widget_tree(
        widget: &dyn Widget<M>,
        path: &mut WidgetPath,
        id: WidgetId,
        stable_scope_id: WidgetId,
        store: &mut AnimationStore,
        focused_id: Option<WidgetId>,
        now: Instant,
        motion_policy: MotionPolicy,
        animation_theme: crate::tui::animation::AnimationTheme,
    ) -> Invalidation {
        let mut requests = {
            let mut ctx = AnimationCtx::with_identity(
                store,
                path.clone(),
                id.clone(),
                focused_id.clone(),
                now,
                motion_policy,
                animation_theme,
            );
            let legacy = widget.animate(&mut ctx);
            let mut requests = ctx.invalidation();
            if legacy {
                requests.request_paint();
                requests.request_next_frame();
            }
            requests
        };

        for (index, child) in widget.children().iter().enumerate() {
            let key = WidgetKey::for_child(index, child.as_ref());
            let (child_id, child_stable_scope_id) =
                WidgetId::for_child(&id, &stable_scope_id, &key);
            path.push(key);
            requests.merge(Self::animate_widget_tree(
                child.as_ref(),
                path,
                child_id,
                child_stable_scope_id,
                store,
                focused_id.clone(),
                now,
                motion_policy,
                animation_theme,
            ));
            path.pop();
        }

        requests
    }

    fn animation_time(&self) -> Instant {
        if !self.motion_policy.deterministic {
            return self.clock.now();
        }
        self.animation_epoch
            + self
                .motion_policy
                .deterministic_step
                .saturating_mul(self.animation_frame.min(u32::MAX as u64) as u32)
    }

    fn animation_now(&mut self) -> Instant {
        let now = self.animation_time();
        if self.motion_policy.deterministic {
            self.animation_frame = self.animation_frame.wrapping_add(1);
        }
        now
    }

    fn animate_widgets(&mut self) -> Invalidation {
        if self.cached_tree.is_none() {
            return Invalidation::default();
        }

        let focused_id = self.focus.current_id().cloned();
        let mut path = WidgetPath::root();
        let now = self.animation_now();
        let animation_theme = self.theme.animations;
        let tree = self.cached_tree.as_ref().unwrap();
        let mut store = self.animation_store.borrow_mut();
        let mut requests = Self::animate_widget_tree(
            tree.as_ref(),
            &mut path,
            WidgetId::root(),
            WidgetId::root(),
            &mut store,
            focused_id,
            now,
            self.motion_policy,
            animation_theme,
        );
        if store.advance(now) {
            requests.request_paint();
            requests.request_next_frame();
        }
        // A completed exit still needs one frame to remove its retained pixels
        // and resources, even if no live animation changed.
        if !self.exiting_visuals.is_empty() {
            requests.request_paint();
            requests.request_next_frame();
        }
        requests
    }

    fn render_exiting_visuals(&self, chunk: &mut Chunk, ctx: &RenderCtx, now: Instant) {
        let exit_geometry = RefCell::new(HashMap::new());
        let exit_hit_regions = RefCell::new(Vec::new());
        for visual in &self.exiting_visuals {
            let animations = &visual.animations;
            let exit_ctx =
                ctx.exit_render_ctx(&visual.store, animations, &exit_geometry, &exit_hit_regions);
            for node in &visual.nodes {
                let frames = animations.timeline_frames(&node.path, "exit", now);
                let display_area = exit_display_area(node.bounds, &frames);

                if display_area.width() == 0 || display_area.height() == 0 {
                    continue;
                }

                let _ = Self::render_widget_at_path(
                    visual.root.as_ref(),
                    node.path.as_slice(),
                    chunk,
                    &exit_ctx,
                    display_area,
                );
            }
        }
    }

    fn render_widget_at_path(
        widget: &dyn Widget<M>,
        path: &[WidgetKey],
        chunk: &mut Chunk,
        ctx: &RenderCtx,
        area: crate::render::area::Area,
    ) -> bool {
        let Some((key, rest)) = path.split_first() else {
            let _ = chunk.with_clip(area, |child_chunk| {
                widget.render(child_chunk, ctx);
            });
            return true;
        };

        let children = widget.children();
        let child = match key {
            WidgetKey::Index(index) => children.get(*index).map(|child| child.as_ref()),
            WidgetKey::Named(name) => children
                .iter()
                .find(|child| child.key() == Some(name.as_str()))
                .map(|child| child.as_ref()),
        };

        let Some(child) = child else {
            return false;
        };

        let child_ctx = ctx.child_ctx(key.clone());
        Self::render_widget_at_path(child, rest, chunk, &child_ctx, area)
    }

    fn prune_completed_exit_visuals(&mut self, now: Instant) {
        for visual in &mut self.exiting_visuals {
            visual.nodes.retain(|node| {
                visual
                    .animations
                    .timeline_is_active(&node.path, "exit", now)
            });
            // A single previous tree can contain exits of different durations.
            // Drop each completed subtree's resources independently.
            let mut retained_ids = HashSet::new();
            visit_widgets(visual.root.as_ref(), &mut |_, path, id| {
                if visual.nodes.iter().any(|node| path.starts_with(&node.path)) {
                    retained_ids.insert(id.clone());
                }
            });
            visual.store.retain_active(|id| retained_ids.contains(id));
            visual
                .animations
                .retain_active(|id| retained_ids.contains(id));
            visual.animations.retain_timeline_paths(|path| {
                visual.nodes.iter().any(|node| path.starts_with(&node.path))
            });
        }
        self.exiting_visuals
            .retain(|visual| !visual.nodes.is_empty());
    }

    fn queue_tick_messages(&mut self) {
        if self.tick_handlers.is_empty() {
            return;
        }

        let now = self.clock.now();

        for tick in &mut self.tick_handlers {
            if now < tick.next_fire_at {
                continue;
            }

            self.messages.push_back((tick.handler)());

            while tick.next_fire_at <= now {
                tick.next_fire_at += tick.interval;
            }
        }
    }

    fn queue_frame_messages(&mut self) {
        if self.frame_handlers.is_empty() {
            return;
        }

        let now = self.clock.now();
        if self
            .frame_runtime
            .last_frame_at
            .is_some_and(|last| now < last + Self::FRAME_INTERVAL)
        {
            return;
        }
        let info = self.frame_runtime.next(now);
        for handler in &self.frame_handlers {
            self.messages.push_back(handler(info));
        }
    }

    fn task_statuses(&self) -> Vec<TaskStatus> {
        self.task_order
            .iter()
            .filter_map(|task_id| self.tasks.get(task_id).map(|record| record.status.clone()))
            .collect()
    }

    fn record_task_status(&mut self, status: TaskStatus) {
        let Some(record) = self.tasks.get_mut(&status.id) else {
            return;
        };

        record.status = status.clone();

        if let Some(handler) = record.task.status_handler.as_ref() {
            if let Some(message) = handler(status) {
                self.messages.push_scoped(record.lease.clone(), message);
            }
        }
    }

    // Each turn has a bounded ingress and update budget so a producer cannot
    // starve terminal input or drawing. The queue itself is currently unbounded.
    const MESSAGE_BUDGET: usize = 256;
    const FRAME_INTERVAL: Duration = Duration::from_nanos(16_666_667);

    fn drain_task_events(&mut self) {
        for _ in 0..Self::MESSAGE_BUDGET {
            let Ok(event) = self.task_receiver.try_recv() else {
                return;
            };
            match event {
                TaskEvent::Message(message) => self.messages.push_back(message),
                TaskEvent::TaskMessage {
                    id,
                    attempt,
                    message,
                } => {
                    if self.accepts_attempt(id, attempt) {
                        self.messages.0.push_back(QueuedMessage::Task {
                            id,
                            attempt,
                            message,
                        });
                    }
                }
                TaskEvent::Finished {
                    id,
                    attempt,
                    mut state,
                } => {
                    if !self.accepts_attempt(id, attempt) {
                        continue;
                    }
                    let record = self.tasks.get_mut(&id).unwrap();
                    record.deadline = None;
                    if state == TaskState::RetryScheduled {
                        if record.task.retry.can_retry(attempt) {
                            record.retry_at = Some(self.clock.now() + record.task.retry.delay());
                        } else {
                            state = TaskState::Failed;
                        }
                    }
                    self.set_task_state(id, state);
                }
            }
        }
        self.wake.wake();
    }

    fn accepts_attempt(&self, id: TaskId, attempt: u32) -> bool {
        self.tasks.get(&id).is_some_and(|record| {
            record.accept_results
                && record.status.attempt == attempt
                && record.status.state == TaskState::Running
                && self.lease_is_live(&record.lease)
        })
    }

    fn accepts_task_message(&self, id: TaskId, attempt: u32) -> bool {
        self.tasks.get(&id).is_some_and(|record| {
            record.accept_results
                && self.lease_is_live(&record.lease)
                && record.status.attempt == attempt
                && !matches!(
                    record.status.state,
                    TaskState::Cancelled | TaskState::Cancelling | TaskState::TimedOut
                )
                && record.status.key.as_ref().is_none_or(|key| {
                    self.task_keys
                        .get(&ResourceKey::new(record.lease.scope.clone(), key))
                        == Some(&id)
                })
        })
    }

    fn take_exit(&mut self) -> AppExit<State> {
        self.shutdown();
        AppExit {
            state: self.state.take().expect("runtime state"),
            reason: self.exit_reason.expect("exit reason"),
        }
    }

    fn shutdown(&mut self) {
        self.should_quit = true;
        self.exit_reason.get_or_insert(ExitReason::Shutdown);
        if let Some(tree) = self
            .cached_tree
            .take()
            .or_else(|| self.previous_tree.take())
        {
            unmount_widget_states(tree.as_ref(), &mut self.store);
        }
        self.previous_tree = None;
        self.exiting_visuals.clear();
        *self.animation_store.borrow_mut() = AnimationStore::new();
        self.focus.clear();
        self.pointer_capture = None;
        self.next_animation_at = None;
        let active: Vec<_> = self
            .tasks
            .iter()
            .filter(|(_, record)| record.status.is_active())
            .map(|(&id, _)| id)
            .collect();
        for id in active {
            self.cancel_task(id);
        }
        self.timers.clear();
        self.debounces.clear();
        self.subscription_tasks.clear();
        self.scopes.clear();
        self.messages.0.clear();
        self.wake.wake();
    }

    fn set_task_state(&mut self, id: TaskId, state: TaskState) {
        let Some(record) = self.tasks.get_mut(&id) else {
            return;
        };
        let mut status = record.status.clone();
        status.state = state;
        status.updated_at = self.clock.now();
        if state.is_terminal() {
            record.deadline = None;
            record.retry_at = None;
        }
        self.record_task_status(status);
    }

    fn cancel_task(&mut self, id: TaskId) {
        let Some(record) = self.tasks.get_mut(&id) else {
            return;
        };
        record.accept_results = false;
        record.cancellation.cancel();
        if record.status.state.is_terminal() {
            return;
        }
        self.set_task_state(id, TaskState::Cancelling);
        // Cancellation stops accepting results immediately. Blocking jobs must
        // cooperate with their token; the UI never waits for an arbitrary thread.
        self.set_task_state(id, TaskState::Cancelled);
    }

    fn process_deadlines(&mut self) {
        let now = self.clock.now();
        let mut pending = Vec::new();
        let mut waiting = Vec::new();
        for (at, lease, message) in self.timers.drain(..) {
            if at <= now {
                self.messages.push_scoped(lease, message);
            } else {
                waiting.push((at, lease, message));
            }
        }
        self.timers = waiting;
        let keys: Vec<_> = self
            .debounces
            .iter()
            .filter(|(_, (at, _, _))| *at <= now)
            .map(|(key, _)| key.clone())
            .collect();
        for key in keys {
            if let Some((_, lease, message)) = self.debounces.remove(&key) {
                self.messages.push_scoped(lease, message);
            }
        }
        for (&id, record) in &self.tasks {
            if record.deadline.is_some_and(|at| at <= now) {
                pending.push((id, true));
            } else if record.retry_at.is_some_and(|at| at <= now) {
                pending.push((id, false));
            }
        }
        for (id, timed_out) in pending {
            if timed_out {
                self.tasks[&id].cancellation.cancel();
                self.set_task_state(id, TaskState::TimedOut);
            } else {
                self.tasks.get_mut(&id).unwrap().status.attempt += 1;
                self.start_task_attempt(id);
            }
        }
    }

    fn cancel_task_key(&mut self, key: &ResourceKey) {
        if let Some(task_id) = self.task_keys.get(key).copied() {
            self.cancel_task(task_id);
        }
    }

    fn prune_finished_tasks(&mut self) {
        while self.tasks.len() > Self::MAX_TRACKED_TASKS {
            let Some(index) = self.task_order.iter().position(|task_id| {
                self.tasks
                    .get(task_id)
                    .map(|record| {
                        record.status.state.is_terminal()
                            && !self.messages.contains_task(*task_id)
                            && !self.subscription_tasks.values().any(|id| id == task_id)
                    })
                    .unwrap_or(false)
            }) else {
                break;
            };

            let task_id = self.task_order.remove(index);
            if let Some(record) = self.tasks.remove(&task_id) {
                if let Some(key) = record.status.key {
                    let key = ResourceKey::new(record.lease.scope, key);
                    if self.task_keys.get(&key) == Some(&task_id) {
                        self.task_keys.remove(&key);
                    }
                }
            }
        }
    }

    fn spawn_task(&mut self, task: Task<M>, lease: ScopeLease) -> TaskId {
        if let Some(key) = task.key.as_deref() {
            self.cancel_task_key(&ResourceKey::new(lease.scope.clone(), key));
        }
        let id = TaskId(self.next_task_id);
        self.next_task_id = self.next_task_id.wrapping_add(1);
        let now = self.clock.now();
        let mut status = TaskStatus::new(
            id,
            task.key.clone(),
            task.label.clone(),
            TaskState::Queued,
            1,
            task.retry.max_attempts(),
            now,
        );
        status.updated_at = now;
        status.scope = lease.scope.clone();
        if let Some(key) = task.key.as_ref() {
            self.task_keys
                .insert(ResourceKey::new(lease.scope.clone(), key), id);
        }
        self.task_order.push(id);
        self.tasks.insert(
            id,
            TaskRecord {
                status: status.clone(),
                lease,
                accept_results: true,
                cancellation: CancellationToken::new(),
                task,
                deadline: None,
                retry_at: None,
            },
        );
        self.record_task_status(status);
        self.start_task_attempt(id);
        self.prune_finished_tasks();
        id
    }

    fn start_task_attempt(&mut self, id: TaskId) {
        let record = self.tasks.get_mut(&id).unwrap();
        record.retry_at = None;
        record.deadline = record
            .task
            .timeout
            .map(|timeout| self.clock.now() + timeout);
        let attempt = record.status.attempt;
        let runner = record.task.runner.clone();
        let sender = self.task_sender.for_attempt(id, attempt);
        let context = TaskContext::new(id, attempt, record.cancellation.clone(), sender.clone());
        self.set_task_state(id, TaskState::Running);
        self.executor.spawn(Box::new(move || {
            let state = run_task_attempt(context, runner);
            sender.send(TaskEvent::Finished { id, attempt, state });
        }));
    }

    fn apply_effect(&mut self, effect: Effect<M>) {
        self.apply_scoped_effect(effect, Scope::root());
    }

    fn apply_scoped_effect(&mut self, effect: Effect<M>, scope: Scope) {
        let Some(lease) = self.lease(&scope) else {
            return;
        };
        match effect {
            Effect::None => {}
            Effect::Exit => {
                self.exit_reason = Some(ExitReason::Requested);
                self.shutdown();
            }
            Effect::Message(message) => self.messages.push_scoped(lease, message),
            Effect::Batch(effects) => {
                for effect in effects {
                    self.apply_scoped_effect(effect, scope.clone());
                    if self.should_quit {
                        break;
                    }
                }
            }
            Effect::Scoped(inner, effect) => self.apply_scoped_effect(*effect, scope.join(&inner)),
            Effect::Spawn(task) => {
                self.spawn_task(task, lease);
            }
            Effect::CancelTask(id) => {
                if scope == Scope::root()
                    || self.tasks.get(&id).is_some_and(|r| r.lease.scope == scope)
                {
                    self.cancel_task(id);
                }
            }
            Effect::CancelTaskKey(key) => self.cancel_task_key(&ResourceKey::new(scope, key)),
            Effect::After(duration, message) => {
                self.timers
                    .push((self.clock.now() + duration, lease, message))
            }
            Effect::Debounce {
                key,
                duration,
                message,
            } => {
                self.debounces.insert(
                    ResourceKey::new(scope, key),
                    (self.clock.now() + duration, lease, message),
                );
            }
            Effect::CancelDebounce(key) => {
                self.debounces.remove(&ResourceKey::new(scope, key));
            }
        }
    }
}

impl<State, U, V, M> Draw for AppRuntime<State, U, V, M>
where
    U: UpdateDriver<State, M>,
    V: Fn(&State) -> Box<dyn Widget<M>>,
    M: Send + 'static,
{
    fn draw(&mut self, mut chunk: Chunk) -> Result<Size, DrawErr> {
        let size = chunk.area().size();

        self.sync_theme();
        chunk.set_text_metrics(self.theme.text_metrics);

        self.ensure_tree();
        let tree = self.cached_tree.as_ref().unwrap();
        sync_widget_states(tree.as_ref(), &mut self.store);
        if self.store.invalidation.layout {
            self.layouts.clear();
            self.store.invalidation.layout = false;
        }
        let render_now = self.animation_now();
        self.prune_completed_exit_visuals(render_now);
        let tree = self.cached_tree.as_ref().unwrap();

        // Resolve the target tree before painting. Geometry-changing animations
        // may refresh only the nodes whose displayed allocation differs.
        let measure_ctx = MeasureCtx::with_runtime(
            &self.store,
            &self.theme,
            render_now,
            self.render_frame,
            self.visual_capabilities,
        );
        self.layouts
            .resolve(tree.as_ref(), chunk.area(), &measure_ctx);

        // Paint and record displayed/clipped hit geometry from those allocations.
        self.store.layout_geometry.borrow_mut().clear();
        self.store.rendered_ids.borrow_mut().clear();
        let focused_path = self.focus.current_path();
        let focused_id = self.focus.current_id().cloned();
        self.geometry.borrow_mut().clear();
        self.hit_regions.borrow_mut().clear();
        let ctx = RenderCtx::with_runtime(
            &self.store,
            &self.animation_store,
            &self.theme,
            focused_path,
            focused_id,
            &self.geometry,
            &self.hit_regions,
            render_now,
            self.render_frame,
            self.motion_policy,
            self.visual_capabilities,
        )
        .with_layouts(&self.layouts)
        .with_viewport(chunk.area());
        ctx.record_bounds(chunk.area());
        ctx.record_hit_bounds(chunk.area(), chunk.area(), tree.hit_test_mode());
        tree.render(&mut chunk, &ctx);
        self.render_exiting_visuals(&mut chunk, &ctx, render_now);
        self.prune_live_animations();
        self.render_frame = self.render_frame.wrapping_add(1);

        Ok(size)
    }
}

impl<State, U, V, M> AppRuntime<State, U, V, M>
where
    U: UpdateDriver<State, M>,
    V: Fn(&State) -> Box<dyn Widget<M>>,
    M: Send + 'static,
{
    fn terminal_loop(
        mut self,
        mouse_capture: bool,
        inline_mouse_mode: InlineMouseMode,
    ) -> WidgetResult<crate::render::event_loop::EventLoop<Self>> {
        let (width, height) = crossterm::terminal::size()?;
        let inline_max_height = self.inline_max_height;
        let inline_mode = self.inline_mode;
        let inline_available_height =
            inline_available_height(height, inline_mouse_mode.force_follows_viewport());

        let (buffer_height, initial_used_height) = if inline_mode {
            self.sync_theme();
            self.ensure_tree();
            let layout = self.cached_tree.as_ref().unwrap();
            let buffer_height = inline_max_height.min(inline_available_height);
            let measure_ctx = MeasureCtx::with_runtime(
                &self.store,
                &self.theme,
                self.clock.now(),
                0,
                self.visual_capabilities,
            );
            let required = layout
                .measure(
                    SizeProposal {
                        width: AxisLimit::Exact(width),
                        height: AxisLimit::AtMost(buffer_height),
                    },
                    &measure_ctx,
                )
                .height;
            let used = required.min(buffer_height);
            (buffer_height, used)
        } else {
            (height, height)
        };

        let mut builder = crate::render::Builder::new();
        builder
            .enable_raw_mode()
            .keyboard_mode(self.keyboard_mode)
            .focus_events(self.focus_events)
            .clear(false)
            .append_newline(false)
            .enable_hide_cursor()
            .disable_exit_code();
        if mouse_capture {
            builder.enable_mouse_capture();
        }

        if inline_mode {
            builder
                .inline_mode(true)
                .inline_max_height(inline_max_height)
                .inline_mouse_mode(inline_mouse_mode)
                .size((width, buffer_height));
        } else {
            builder.enable_alt_screen().size((width, height));
        }

        let mut event_loop = builder.build_event_loop(self);

        if inline_mode {
            event_loop.set_initial_used_height(initial_used_height);
        }

        Ok(event_loop)
    }

    pub(crate) fn run_terminal(
        self,
        mouse: bool,
        inline_mouse: InlineMouseMode,
    ) -> WidgetResult<AppExit<State>> {
        crate::render::event_loop::EventLoop::<Self>::check_sync_runtime()
            .map_err(|e| crate::tui::WidgetError::render_error(e.to_string()))?;
        let mut core = self
            .terminal_loop(mouse, inline_mouse)?
            .run_returning()
            .map_err(|e| crate::tui::WidgetError::render_error(e.to_string()))?;
        core.exit_reason.get_or_insert(ExitReason::InputClosed);
        Ok(core.take_exit())
    }

    pub(crate) async fn run_terminal_async(
        self,
        mouse: bool,
        inline_mouse: InlineMouseMode,
    ) -> WidgetResult<AppExit<State>> {
        let mut core = self
            .terminal_loop(mouse, inline_mouse)?
            .run_async_returning()
            .await
            .map_err(|e| crate::tui::WidgetError::render_error(e.to_string()))?;
        core.exit_reason.get_or_insert(ExitReason::InputClosed);
        Ok(core.take_exit())
    }
}

impl<State, U, V, M> Update for AppRuntime<State, U, V, M>
where
    U: UpdateDriver<State, M>,
    V: Fn(&State) -> Box<dyn Widget<M>>,
    M: Send + 'static,
{
    fn on_input_capabilities(&mut self, capabilities: InputCapabilities) {
        if let Some(message) = self
            .input_handler
            .as_ref()
            .and_then(|handler| handler(InputEvent::Capabilities(capabilities)))
        {
            self.messages.push_back(message);
        }
    }

    fn wake_handle(&self) -> Option<crate::render::WakeHandle> {
        Some(self.wake.clone())
    }

    fn next_update_at(&self) -> Option<Instant> {
        if self.should_quit {
            return None;
        }
        let now = self.clock.now();
        if !self.messages.is_empty() {
            return Some(now);
        }
        let animation = self.next_animation_at.or_else(|| {
            self.animation_store
                .borrow()
                .has_active_animations()
                .then_some(now + Self::FRAME_INTERVAL)
        });
        let frame = if self.frame_handlers.is_empty() {
            None
        } else {
            Some(
                self.frame_runtime
                    .last_frame_at
                    .map_or(now, |last| last + Self::FRAME_INTERVAL),
            )
        };
        self.tick_handlers
            .iter()
            .map(|tick| tick.next_fire_at)
            .chain(self.timers.iter().map(|(at, _, _)| *at))
            .chain(self.debounces.values().map(|(at, _, _)| *at))
            .chain(
                self.tasks
                    .values()
                    .filter_map(|record| record.deadline.or(record.retry_at)),
            )
            .chain(animation)
            .chain(frame)
            .min()
    }

    fn on_events(&mut self, events: &[Event]) -> Result<(), DrawErr> {
        self.ensure_tree();
        sync_widget_states(self.cached_tree.as_ref().unwrap().as_ref(), &mut self.store);

        let tree = self.cached_tree.as_ref().unwrap();
        let geometry = self.geometry.borrow();
        let hit_regions = self.hit_regions.borrow();

        for event in events {
            if let Some(message) = self
                .input_handler
                .as_ref()
                .and_then(|handler| handler(InputEvent::Terminal(event.clone())))
            {
                self.messages.push_back(message);
            }
            if let Event::Resize(_, _) = event {
                continue;
            }

            let pointer_capture = self.pointer_capture.clone();
            let mouse_target = mouse_target_path(event, &hit_regions, pointer_capture.as_ref());
            if let Event::Mouse(mouse_event) = event {
                match mouse_event.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        self.pointer_capture = mouse_target.clone();
                        if let Some(path) = mouse_target.as_ref() {
                            self.focus.request_focus(path);
                        }
                    }
                    MouseEventKind::Up(_) => {
                        self.pointer_capture = None;
                    }
                    _ => {}
                }
            }

            if Self::dispatch_widget_event(
                tree.as_ref(),
                event,
                &mut self.store,
                &mut self.messages,
                &mut self.focus,
                &geometry,
                &hit_regions,
                pointer_capture.as_ref(),
                &self.theme,
                self.clock.now(),
                self.render_frame,
                self.visual_capabilities,
            ) {
                if matches!(event, Event::Key(_)) {
                    if let Some(router) = self.command_router.as_mut() {
                        router.cancel_pending();
                    }
                }
                continue;
            }

            // Only handle keyboard events after widget propagation.
            let key_event = match event {
                Event::Key(k) => k,
                _ => continue,
            };
            if key_event.kind == KeyEventKind::Release {
                continue;
            }

            // 1. Tab / Shift+Tab → default focus navigation
            match key_event.code {
                KeyCode::Tab if key_event.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.focus.prev();
                    if let Some(router) = self.command_router.as_mut() {
                        router.cancel_pending();
                    }
                    continue;
                }
                KeyCode::Tab => {
                    self.focus.next();
                    if let Some(router) = self.command_router.as_mut() {
                        router.cancel_pending();
                    }
                    continue;
                }
                _ => {}
            }

            // Scoped command sequences use the runtime's injected clock.
            if let Some(router) = self.command_router.as_mut() {
                let modal = self
                    .focus
                    .modal_path()
                    .and_then(|path| self.focus.analysis_id(path));
                match router.handle_key_in(
                    key_event,
                    self.focus.current_id(),
                    modal,
                    self.clock.now(),
                ) {
                    CommandMatch::Message(message) => {
                        self.messages.push_back(message);
                        continue;
                    }
                    CommandMatch::Pending => continue,
                    CommandMatch::Ignored => {}
                }
            }
            if self.focus.modal_path().is_some() {
                continue;
            }

            // 2. Quit key
            let should_quit = match &self.quit_behavior {
                QuitBehavior::Default => {
                    key_event.code == KeyCode::Esc && key_event.modifiers.is_empty()
                }
                QuitBehavior::CustomKey(k) => {
                    key_event.code == *k && key_event.modifiers.is_empty()
                }
                QuitBehavior::CustomKeyEvent(ke) => {
                    key_event.code == ke.code && key_event.modifiers == ke.modifiers
                }
                QuitBehavior::Disabled => false,
            };
            if should_quit {
                self.exit_reason = Some(ExitReason::QuitKey);
                self.should_quit = true;
                return Ok(());
            }

            // 4. Rich hotkey registry
            if let Some(message) = self.hotkeys.resolve(key_event) {
                self.messages.push_back(message);
                continue;
            }

            // 5. Legacy global key handlers
            if let Some(handler) = self.global_key_handlers.get(&key_event.code) {
                self.messages.push_back(handler());
            }
        }

        Ok(())
    }

    fn update(&mut self) -> Result<bool, DrawErr> {
        if self.should_quit {
            self.shutdown();
            return Ok(false);
        }
        self.process_deadlines();
        self.drain_task_events();
        self.queue_tick_messages();
        self.queue_frame_messages();

        let mut processed_messages = false;

        for _ in 0..Self::MESSAGE_BUDGET {
            let Some(message) = self.messages.pop_front() else {
                break;
            };
            let message = match message {
                QueuedMessage::Plain(message) => message,
                QueuedMessage::Scoped { lease, message } => {
                    if !self.lease_is_live(&lease) {
                        continue;
                    }
                    message
                }
                QueuedMessage::Task {
                    id,
                    attempt,
                    message,
                } => {
                    if !self.accepts_task_message(id, attempt) {
                        continue;
                    }
                    message
                }
            };
            processed_messages = true;
            let mut effects = Vec::new();
            let statuses = self.task_statuses();
            let task_keys = self.task_keys.clone();
            let now = self.clock.now();
            {
                let mut ctx = UpdateCtx::new(&mut effects, statuses, task_keys, now);
                self.update_fn.handle(
                    self.state.as_mut().expect("runtime state"),
                    message,
                    &mut ctx,
                );
            }
            // Reconcile each update before accepting the next queued result.
            // An unmount in this batch must invalidate messages immediately.
            if self.previous_tree.is_none() {
                self.previous_tree = self.cached_tree.take();
                self.previous_geometry = self.geometry.borrow().clone();
            } else {
                self.cached_tree = None;
            }
            self.ensure_tree_with_sync(false);
            for effect in effects {
                self.apply_effect(effect);
                if self.should_quit {
                    return Ok(false);
                }
            }
        }
        self.prune_finished_tasks();

        self.sync_theme();
        self.ensure_tree();
        if processed_messages {
            // Controlled values synchronize once per input batch, after its
            // final app value is known. Mount/unmount and resource cancellation
            // still occur between messages.
            sync_widget_states(self.cached_tree.as_ref().unwrap().as_ref(), &mut self.store);
        }
        let mut requests = self.animate_widgets();
        requests.merge(std::mem::take(&mut self.store.invalidation));
        if requests.layout {
            self.layouts.clear();
        }
        self.next_animation_at = requests
            .next_frame
            .then(|| self.clock.now() + Self::FRAME_INTERVAL);

        Ok(processed_messages || requests.paint || requests.layout)
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn required_size(&self, current_size: Size) -> Option<Size> {
        if !self.inline_mode {
            return None;
        }
        if let Some(ref tree) = self.cached_tree {
            let measure_ctx = MeasureCtx::with_runtime(
                &self.store,
                &self.theme,
                self.clock.now(),
                self.render_frame,
                self.visual_capabilities,
            );
            let required = tree
                .measure(
                    SizeProposal {
                        width: AxisLimit::Exact(current_size.width),
                        height: AxisLimit::AtMost(self.inline_max_height),
                    },
                    &measure_ctx,
                )
                .height;
            let height = required.min(self.inline_max_height);
            if height != current_size.height {
                return Some(Size {
                    width: current_size.width,
                    height,
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::animation::{AnimationSpec, Easing, Transition};

    #[test]
    fn exit_collapse_reduces_height_without_moving_origin() {
        let area = crate::render::area::Area::new((2, 3).into(), (10, 6).into());
        let frame = TimelineFrame {
            transition: Transition::new(
                TransitionEffect::Collapse,
                AnimationSpec::new(Duration::from_millis(100), Easing::Linear),
            ),
            progress: 0.5,
            complete: false,
        };

        let display = exit_display_area(area, &[frame]);

        assert_eq!(display.x(), 2);
        assert_eq!(display.y(), 3);
        assert_eq!(display.width(), 10);
        assert_eq!(display.height(), 3);
    }

    #[test]
    fn exit_scale_from_center_collapses_around_center() {
        let area = crate::render::area::Area::new((2, 4).into(), (10, 6).into());
        let frame = TimelineFrame {
            transition: Transition::new(
                TransitionEffect::ScaleFromCenter,
                AnimationSpec::new(Duration::from_millis(100), Easing::Linear),
            ),
            progress: 0.5,
            complete: false,
        };

        let display = exit_display_area(area, &[frame]);

        assert_eq!(display.x(), 4);
        assert_eq!(display.y(), 5);
        assert_eq!(display.width(), 5);
        assert_eq!(display.height(), 3);
    }

    #[test]
    fn hit_test_uses_latest_rendered_region_on_overlap() {
        let lower = WidgetPath::root().child("lower");
        let upper = WidgetPath::root().child("upper");
        let regions = vec![
            HitRegion {
                path: lower.clone(),
                area: crate::render::area::Area::new((0, 0).into(), (10, 5).into()),
            },
            HitRegion {
                path: upper.clone(),
                area: crate::render::area::Area::new((2, 1).into(), (10, 5).into()),
            },
        ];

        assert_eq!(hit_test_path(&regions, 3, 2), Some(upper));
        assert_eq!(hit_test_path(&regions, 1, 1), Some(lower));
        assert_eq!(hit_test_path(&regions, 20, 20), None);
    }

    #[test]
    fn mouse_capture_is_enabled_by_default() {
        let app = App::<(), ()>::new(());

        assert!(app.mouse_capture);
    }

    #[test]
    fn drag_events_keep_captured_target_when_pointer_leaves_hit_region() {
        let captured = WidgetPath::root().child("slider");
        let regions = vec![HitRegion {
            path: captured.clone(),
            area: crate::render::area::Area::new((0, 0).into(), (10, 2).into()),
        }];
        let event = Event::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 40,
            row: 20,
            modifiers: KeyModifiers::empty(),
        });

        assert_eq!(
            mouse_target_path(&event, &regions, Some(&captured)),
            Some(captured)
        );
    }
}
