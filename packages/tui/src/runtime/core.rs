use std::cell::RefCell;
use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::event::Event;
use render::area::Size;
use render::chunk::Chunk;
use render::{Draw, DrawErr, InlineMouseMode, Update};

use super::{Clock, Executor};
use crate::animation::{
    AnimationCtx, AnimationStore, MotionPolicy, Presence, Timeline, TimelineFrame, TransitionEffect,
};
use crate::effect::{
    run_task_attempt, CancellationToken, Effect, Task, TaskContext, TaskEvent, TaskEventSender,
    TaskId, TaskState, TaskStatus, UpdateCtx,
};
use crate::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use crate::focus::FocusManager;
use crate::layout::{AxisLimit, SizeProposal};
use crate::shell::{CommandRouter, HotkeyRegistry};
use crate::style::Theme;
use crate::widget::{
    EventCtx, EventPhase, FocusRequest, HitRegion, MeasureCtx, RenderCtx, Widget, WidgetId,
    WidgetKey, WidgetPath, WidgetStore,
};
use crate::widgets::text_input::TextInputState;
use crate::widgets::textarea::TextAreaState;
use crate::widgets::visual::TerminalVisualCapabilities;
use crate::WidgetResult;

use crate::app::{App, EventHandler, FrameHandler, FrameInfo, QuitBehavior, ThemeResolver};

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
    buffer: render::buffer::Buffer,
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

impl<State, M: Clone + std::fmt::Debug + Send + 'static> Runtime<State, M> {
    pub(crate) fn from_app<F, V, W>(app: App<State, M>, update: F, view: V) -> Self
    where
        F: Fn(&mut State, M, &mut UpdateCtx<M>) + 'static,
        V: Fn(&State) -> W + 'static,
        W: Widget<M> + 'static,
    {
        let update: RuntimeUpdate<State, M> = Box::new(update);
        let view: RuntimeView<State, M> = Box::new(move |state| Box::new(view(state)));
        Runtime {
            mouse_capture: app.mouse_capture,
            inline_mouse_mode: app.inline_mouse_mode,
            core: AppRuntime::new(app, update, view, false, 50),
            buffer: render::buffer::Buffer::new((80, 24).into()),
            needs_draw: true,
        }
    }

    /// Enter the terminal adapter after obtaining a handle with `handle()`.
    pub fn run(self) -> WidgetResult<()> {
        self.core
            .run_terminal(self.mouse_capture, self.inline_mouse_mode)
    }

    /// Enter the inline terminal adapter using this runtime and its message queue.
    pub fn run_inline(mut self) -> WidgetResult<()> {
        self.core.inline_mode = true;
        self.core
            .run_terminal(self.mouse_capture, self.inline_mouse_mode)
    }

    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.resize((width, height).into());
        self
    }

    pub fn resize(&mut self, size: Size) {
        self.buffer = render::buffer::Buffer::new(size);
        self.needs_draw = true;
    }

    pub fn handle(&self) -> AppHandle<M> {
        AppHandle {
            sender: self.core.task_sender.clone(),
        }
    }

    pub fn state(&self) -> &State {
        &self.core.state
    }

    pub fn frame(&self) -> &render::buffer::Buffer {
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

    pub fn bounds(&self, path: &WidgetPath) -> Option<render::area::Area> {
        self.core.geometry.borrow().get(path).copied()
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

    pub fn wake_handle(&self) -> render::WakeHandle {
        self.core.wake.clone()
    }

    /// Stop timers and signal cancellation without waiting for blocking jobs.
    pub fn shutdown(&mut self) {
        self.core.shutdown();
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
        let area = render::area::Area::new((0, 0).into(), self.buffer.size());
        let chunk = Chunk::new(&mut self.buffer, area)?;
        self.core.draw(chunk)?;
        self.needs_draw = false;
        Ok(())
    }
}

struct TaskRecord<M> {
    status: TaskStatus,
    cancellation: CancellationToken,
    task: Task<M>,
    deadline: Option<Instant>,
    retry_at: Option<Instant>,
}

enum QueuedMessage<M> {
    Plain(M),
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
    state: State,
    theme: Theme,
    theme_resolver: Option<ThemeResolver<State>>,
    update_fn: U,
    view_fn: V,
    store: WidgetStore,
    animation_store: RefCell<AnimationStore>,
    geometry: RefCell<HashMap<WidgetPath, render::area::Area>>,
    hit_regions: RefCell<Vec<HitRegion>>,
    focus: FocusManager,
    cached_tree: Option<Box<dyn Widget<M>>>,
    presence_previous_tree: Option<Box<dyn Widget<M>>>,
    presence_previous_geometry: HashMap<WidgetPath, render::area::Area>,
    exiting_visuals: Vec<ExitingVisual<M>>,
    messages: MessageQueue<M>,
    should_quit: bool,
    quit_behavior: QuitBehavior,
    global_key_handlers: HashMap<KeyCode, Box<dyn Fn() -> M>>,
    hotkeys: HotkeyRegistry<M>,
    command_router: Option<CommandRouter<M>>,
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
    debounces: HashMap<String, (Instant, M)>,
    timers: Vec<(Instant, M)>,
    wake: render::WakeHandle,
    clock: Arc<dyn Clock>,
    executor: Arc<dyn Executor>,
    next_animation_at: Option<Instant>,
    next_task_id: u64,
    tasks: HashMap<TaskId, TaskRecord<M>>,
    task_keys: HashMap<String, TaskId>,
    task_order: Vec<TaskId>,
}

struct ExitingVisual<M> {
    root: Box<dyn Widget<M>>,
    nodes: Vec<ExitingNode>,
}

struct ExitingNode {
    path: WidgetPath,
    bounds: render::area::Area,
}

impl<State, U, V, M> Drop for AppRuntime<State, U, V, M> {
    fn drop(&mut self) {
        // Blocking jobs may outlive the UI. Signal cancellation without joining
        // them; dropping the receiver rejects all later messages.
        for record in self.tasks.values() {
            record.cancellation.cancel();
        }
    }
}

fn exit_display_area(area: render::area::Area, frames: &[TimelineFrame]) -> render::area::Area {
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

fn vertical_area_for_exit(area: render::area::Area, progress: f64) -> render::area::Area {
    let height = ((area.height() as f64) * progress.clamp(0.0, 1.0)).round() as u16;
    render::area::Area::new(area.pos(), (area.width(), height).into())
}

fn scale_area_for_exit(area: render::area::Area, progress: f64) -> render::area::Area {
    let progress = progress.clamp(0.0, 1.0);
    let width = ((area.width() as f64) * progress).round() as u16;
    let height = ((area.height() as f64) * progress).round() as u16;
    let x = area.x() + area.width().saturating_sub(width) / 2;
    let y = area.y() + area.height().saturating_sub(height) / 2;

    render::area::Area::new((x, y).into(), (width, height).into())
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
            theme,
            theme_resolver,
            global_key_handlers,
            hotkeys,
            command_router,
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
        let wake = render::WakeHandle::new();
        let task_sender = TaskEventSender::new(sender, wake.clone());

        AppRuntime {
            state,
            theme,
            theme_resolver,
            update_fn: update,
            view_fn: view,
            store: WidgetStore::new(),
            animation_store: RefCell::new(AnimationStore::new()),
            geometry: RefCell::new(HashMap::new()),
            hit_regions: RefCell::new(Vec::new()),
            focus: FocusManager::new(),
            cached_tree: None,
            presence_previous_tree: None,
            presence_previous_geometry: HashMap::new(),
            exiting_visuals: Vec::new(),
            messages: MessageQueue::new(),
            should_quit: false,
            quit_behavior,
            global_key_handlers,
            hotkeys,
            command_router,
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
        if self.cached_tree.is_none() {
            let tree = (self.view_fn)(&self.state);
            self.focus.rebuild(tree.as_ref());
            self.retain_state_for_live_and_exiting_paths();
            self.cached_tree = Some(tree);
        }
    }

    fn exiting_paths(&self) -> Vec<WidgetPath> {
        self.exiting_visuals
            .iter()
            .flat_map(|visual| visual.nodes.iter().map(|node| node.path.clone()))
            .collect()
    }

    fn retain_state_for_live_and_exiting_paths(&mut self) {
        let live_paths: Vec<WidgetPath> = self.focus.live_paths().iter().cloned().collect();
        let live_ids: Vec<WidgetId> = self.focus.live_ids().iter().cloned().collect();
        let exiting_paths = self.exiting_paths();
        let mut live_shared_ids = HashSet::new();
        if let Some(tree) = self.cached_tree.as_ref() {
            let mut path = WidgetPath::root();
            Self::collect_shared_transition_ids(tree.as_ref(), &mut path, &mut live_shared_ids);
        }
        self.store
            .retain_active(|id| live_ids.iter().any(|live| live == id));
        let mut animation_store = self.animation_store.borrow_mut();
        animation_store.retain_active(|id| live_ids.iter().any(|live| live == id));
        animation_store.retain_timeline_paths(|path| {
            live_paths.iter().any(|live| live == path)
                || exiting_paths.iter().any(|exit| path.starts_with(exit))
        });
        animation_store.retain_shared_layouts(|id| live_shared_ids.contains(id));
    }

    fn collect_presence_declarations(
        widget: &dyn Widget<M>,
        path: &mut WidgetPath,
        declarations: &mut HashMap<WidgetPath, Presence>,
    ) {
        if let Some(presence) = widget.presence() {
            declarations.insert(path.clone(), presence.clone());
        }

        for (index, child) in widget.children().iter().enumerate() {
            path.push(WidgetKey::for_child(index, child.as_ref()));
            Self::collect_presence_declarations(child.as_ref(), path, declarations);
            path.pop();
        }
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
        previous_geometry: HashMap<WidgetPath, render::area::Area>,
        now: Instant,
    ) {
        let mut previous_presence = HashMap::new();
        let mut path = WidgetPath::root();
        Self::collect_presence_declarations(
            previous_tree.as_ref(),
            &mut path,
            &mut previous_presence,
        );

        if previous_presence.is_empty() {
            return;
        }

        let live_paths: Vec<WidgetPath> = self.focus.live_paths().iter().cloned().collect();
        let mut candidates: Vec<(WidgetPath, render::area::Area, Timeline)> = previous_presence
            .into_iter()
            .filter_map(|(path, presence)| {
                if live_paths.iter().any(|live| live == &path) {
                    return None;
                }

                let timeline = presence.exit?;
                let bounds = previous_geometry.get(&path).copied()?;
                Some((path, bounds, timeline))
            })
            .collect();

        candidates.sort_by_key(|(path, _, _)| path.len());

        let mut nodes: Vec<ExitingNode> = Vec::new();
        for (path, bounds, timeline) in candidates {
            if nodes.iter().any(|node| path.starts_with(&node.path)) {
                continue;
            }

            self.animation_store.borrow_mut().track_timeline(
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
                root: previous_tree,
                nodes,
            });
        }
    }

    fn sync_theme(&mut self) {
        if let Some(resolver) = self.theme_resolver.as_ref() {
            self.theme = resolver(&self.state);
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
        geometry: &HashMap<WidgetPath, render::area::Area>,
        phase: EventPhase,
        already_handled: bool,
        theme: &Theme,
        now: Instant,
        frame: u64,
        visual_capabilities: TerminalVisualCapabilities,
    ) -> crate::widget::EventOutcome {
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
        geometry: &HashMap<WidgetPath, render::area::Area>,
        hit_regions: &[HitRegion],
        pointer_capture: Option<&WidgetPath>,
        theme: &Theme,
        now: Instant,
        frame: u64,
        visual_capabilities: TerminalVisualCapabilities,
    ) -> bool {
        let target_path = match mouse_position(event) {
            Some(_) => mouse_target_path(event, hit_regions, pointer_capture),
            None => focus.current_path().cloned(),
        };
        let route = Self::build_event_route(tree, target_path.as_ref());
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
        animation_theme: crate::animation::AnimationTheme,
    ) -> bool {
        let mut needs_render = {
            let mut ctx = AnimationCtx::with_identity(
                store,
                path.clone(),
                id.clone(),
                focused_id.clone(),
                now,
                motion_policy,
                animation_theme,
            );
            widget.animate(&mut ctx)
        };

        for (index, child) in widget.children().iter().enumerate() {
            let key = WidgetKey::for_child(index, child.as_ref());
            let (child_id, child_stable_scope_id) =
                WidgetId::for_child(&id, &stable_scope_id, &key);
            path.push(key);
            needs_render |= Self::animate_widget_tree(
                child.as_ref(),
                path,
                child_id,
                child_stable_scope_id,
                store,
                focused_id.clone(),
                now,
                motion_policy,
                animation_theme,
            );
            path.pop();
        }

        needs_render
    }

    fn animation_now(&mut self) -> Instant {
        if !self.motion_policy.deterministic {
            return self.clock.now();
        }

        let now = self.animation_epoch
            + self
                .motion_policy
                .deterministic_step
                .saturating_mul(self.animation_frame.min(u32::MAX as u64) as u32);
        self.animation_frame = self.animation_frame.wrapping_add(1);
        now
    }

    fn animate_widgets(&mut self) -> bool {
        if self.cached_tree.is_none() {
            return false;
        }

        let focused_id = self.focus.current_id().cloned();
        let mut path = WidgetPath::root();
        let now = self.animation_now();
        let animation_theme = self.theme.animations;
        let tree = self.cached_tree.as_ref().unwrap();
        let mut store = self.animation_store.borrow_mut();
        let mut needs_render = Self::animate_widget_tree(
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
        needs_render |= store.advance(now);
        needs_render
    }

    fn render_exiting_visuals(&self, chunk: &mut Chunk, ctx: &RenderCtx, now: Instant) {
        let exit_geometry = RefCell::new(HashMap::new());
        let exit_hit_regions = RefCell::new(Vec::new());
        let exit_ctx = ctx.exit_render_ctx(&exit_geometry, &exit_hit_regions);

        for visual in &self.exiting_visuals {
            for node in &visual.nodes {
                let frames = self
                    .animation_store
                    .borrow()
                    .timeline_frames(&node.path, "exit", now);
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
        area: render::area::Area,
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
        let animation_store = self.animation_store.borrow();
        for visual in &mut self.exiting_visuals {
            visual
                .nodes
                .retain(|node| animation_store.timeline_is_active(&node.path, "exit", now));
        }
        drop(animation_store);

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
                self.messages.push_back(message);
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
            record.status.attempt == attempt && record.status.state == TaskState::Running
        })
    }

    fn accepts_task_message(&self, id: TaskId, attempt: u32) -> bool {
        self.tasks.get(&id).is_some_and(|record| {
            record.status.attempt == attempt
                && !matches!(
                    record.status.state,
                    TaskState::Cancelled | TaskState::Cancelling | TaskState::TimedOut
                )
                && record
                    .status
                    .key
                    .as_ref()
                    .is_none_or(|key| self.task_keys.get(key) == Some(&id))
        })
    }

    fn shutdown(&mut self) {
        self.should_quit = true;
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
        let Some(record) = self.tasks.get(&id) else {
            return;
        };
        if record.status.state.is_terminal() {
            return;
        }
        record.cancellation.cancel();
        self.set_task_state(id, TaskState::Cancelling);
        // Cancellation stops accepting results immediately. Blocking jobs must
        // cooperate with their token; the UI never waits for an arbitrary thread.
        self.set_task_state(id, TaskState::Cancelled);
    }

    fn process_deadlines(&mut self) {
        let now = self.clock.now();
        let mut pending = Vec::new();
        let mut waiting = Vec::new();
        for (at, message) in self.timers.drain(..) {
            if at <= now {
                self.messages.push_back(message);
            } else {
                waiting.push((at, message));
            }
        }
        self.timers = waiting;
        let keys: Vec<_> = self
            .debounces
            .iter()
            .filter(|(_, (at, _))| *at <= now)
            .map(|(key, _)| key.clone())
            .collect();
        for key in keys {
            if let Some((_, message)) = self.debounces.remove(&key) {
                self.messages.push_back(message);
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

    fn cancel_task_key(&mut self, key: &str) {
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
                        record.status.state.is_terminal() && !self.messages.contains_task(*task_id)
                    })
                    .unwrap_or(false)
            }) else {
                break;
            };

            let task_id = self.task_order.remove(index);
            if let Some(record) = self.tasks.remove(&task_id) {
                if let Some(key) = record.status.key {
                    if self.task_keys.get(&key) == Some(&task_id) {
                        self.task_keys.remove(&key);
                    }
                }
            }
        }
    }

    fn spawn_task(&mut self, task: Task<M>) {
        if let Some(key) = task.key.as_deref() {
            self.cancel_task_key(key);
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
        if let Some(key) = task.key.as_ref() {
            self.task_keys.insert(key.clone(), id);
        }
        self.task_order.push(id);
        self.tasks.insert(
            id,
            TaskRecord {
                status: status.clone(),
                cancellation: CancellationToken::new(),
                task,
                deadline: None,
                retry_at: None,
            },
        );
        self.record_task_status(status);
        self.start_task_attempt(id);
        self.prune_finished_tasks();
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
        match effect {
            Effect::None => {}
            Effect::Message(message) => self.messages.push_back(message),
            Effect::Batch(effects) => {
                for effect in effects {
                    self.apply_effect(effect);
                }
            }
            Effect::Spawn(task) => self.spawn_task(task),
            Effect::CancelTask(task_id) => self.cancel_task(task_id),
            Effect::CancelTaskKey(key) => self.cancel_task_key(&key),
            Effect::After(duration, message) => {
                self.timers.push((self.clock.now() + duration, message));
            }
            Effect::Debounce {
                key,
                duration,
                message,
            } => {
                self.debounces
                    .insert(key, (self.clock.now() + duration, message));
            }
            Effect::CancelDebounce(key) => {
                self.debounces.remove(&key);
            }
        }
    }
}

impl<State, U, V, M> Draw for AppRuntime<State, U, V, M>
where
    U: UpdateDriver<State, M>,
    V: Fn(&State) -> Box<dyn Widget<M>>,
    M: Clone + std::fmt::Debug + Send + 'static,
{
    fn draw(&mut self, mut chunk: Chunk) -> Result<Size, DrawErr> {
        let size = chunk.area().size();

        self.sync_theme();

        // Rebuild widget tree from current state
        let tree = (self.view_fn)(&self.state);

        // Rebuild focus chain
        self.focus.rebuild(tree.as_ref());

        let render_now = self.animation_now();
        if let Some(previous_tree) = self.presence_previous_tree.take() {
            let previous_geometry = std::mem::take(&mut self.presence_previous_geometry);
            self.start_exit_visuals(previous_tree, previous_geometry, render_now);
        }

        self.retain_state_for_live_and_exiting_paths();

        // Render
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
        );
        tree.render(&mut chunk, &ctx);
        self.render_exiting_visuals(&mut chunk, &ctx, render_now);
        self.prune_completed_exit_visuals(render_now);
        self.retain_state_for_live_and_exiting_paths();
        self.render_frame = self.render_frame.wrapping_add(1);

        // Cache tree for event handling in on_events()
        self.cached_tree = Some(tree);

        Ok(size)
    }
}

impl<State, U, V, M> AppRuntime<State, U, V, M>
where
    U: UpdateDriver<State, M>,
    V: Fn(&State) -> Box<dyn Widget<M>>,
    M: Clone + std::fmt::Debug + Send + 'static,
{
    pub(crate) fn run_terminal(
        self,
        mouse_capture: bool,
        inline_mouse_mode: InlineMouseMode,
    ) -> WidgetResult<()> {
        let (width, height) = crossterm::terminal::size()?;
        let inline_max_height = self.inline_max_height;
        let inline_mode = self.inline_mode;
        let inline_available_height =
            inline_available_height(height, inline_mouse_mode.force_follows_viewport());

        let (buffer_height, initial_used_height) = if inline_mode {
            let layout = (self.view_fn)(&self.state);
            let buffer_height = inline_max_height.min(inline_available_height);
            let measure_store = WidgetStore::new();
            let measure_theme = self
                .theme_resolver
                .as_ref()
                .map(|resolver| resolver(&self.state))
                .unwrap_or_else(|| self.theme.clone());
            let measure_ctx = MeasureCtx::with_runtime(
                &measure_store,
                &measure_theme,
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

        let mut builder = render::Builder::new();
        builder
            .enable_raw_mode()
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

        event_loop
            .run()
            .map_err(|e| crate::error::WidgetError::render_error(e.to_string()))?;

        Ok(())
    }
}

impl<State, U, V, M> Update for AppRuntime<State, U, V, M>
where
    U: UpdateDriver<State, M>,
    V: Fn(&State) -> Box<dyn Widget<M>>,
    M: Clone + std::fmt::Debug + Send + 'static,
{
    fn wake_handle(&self) -> Option<render::WakeHandle> {
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
            .chain(self.timers.iter().map(|(at, _)| *at))
            .chain(self.debounces.values().map(|(at, _)| *at))
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
        // Reset TextInputState.modified_this_batch so we sync from parent when value differs
        self.store
            .for_each_state_mut::<TextInputState, _>(|_, s| s.modified_this_batch = false);
        self.store
            .for_each_state_mut::<TextAreaState, _>(|_, s| s.modified_this_batch = false);

        // Ensure we have a tree (may be missing on first frame before draw)
        self.ensure_tree();

        let tree = self.cached_tree.as_ref().unwrap();
        let geometry = self.geometry.borrow();
        let hit_regions = self.hit_regions.borrow();

        for event in events {
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
                continue;
            }

            // Only handle keyboard events after widget propagation.
            let key_event = match event {
                Event::Key(k) => k,
                _ => continue,
            };

            // 1. Tab / Shift+Tab → default focus navigation
            match key_event.code {
                KeyCode::Tab if key_event.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.focus.prev();
                    continue;
                }
                KeyCode::Tab => {
                    self.focus.next();
                    continue;
                }
                _ => {}
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
                self.should_quit = true;
                return Ok(());
            }

            // 3. Command router
            if let Some(router) = self.command_router.as_ref() {
                if let Some(message) = router.dispatch_hotkey(key_event) {
                    self.messages.push_back(message);
                    continue;
                }
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
                self.update_fn.handle(&mut self.state, message, &mut ctx);
            }
            for effect in effects {
                self.apply_effect(effect);
            }
        }
        self.prune_finished_tasks();

        if processed_messages {
            // Preserve the last rendered tree so draw() can play exit presence
            // for widgets that disappear from the next view tree.
            if self.presence_previous_tree.is_none() {
                self.presence_previous_tree = self.cached_tree.take();
                self.presence_previous_geometry = self.geometry.borrow().clone();
            } else {
                self.cached_tree = None;
            }
        }

        self.sync_theme();
        self.ensure_tree();
        let needs_animation_frame = self.animate_widgets();
        self.next_animation_at =
            needs_animation_frame.then(|| self.clock.now() + Self::FRAME_INTERVAL);

        Ok(processed_messages || needs_animation_frame)
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
    use crate::animation::{AnimationSpec, Easing, Transition};

    #[test]
    fn exit_collapse_reduces_height_without_moving_origin() {
        let area = render::area::Area::new((2, 3).into(), (10, 6).into());
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
        let area = render::area::Area::new((2, 4).into(), (10, 6).into());
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
                area: render::area::Area::new((0, 0).into(), (10, 5).into()),
            },
            HitRegion {
                path: upper.clone(),
                area: render::area::Area::new((2, 1).into(), (10, 5).into()),
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
            area: render::area::Area::new((0, 0).into(), (10, 2).into()),
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
