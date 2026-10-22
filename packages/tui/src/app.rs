use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use render::InlineMouseMode;

use crate::animation::MotionPolicy;
use crate::effect::UpdateCtx;
use crate::event::{KeyCode, KeyEvent};
use crate::runtime::{
    AppRuntime, Clock, Executor, Runtime, SimpleUpdate, SystemClock, ThreadExecutor, UpdateDriver,
};
use crate::shell::{CommandRouter, Hotkey, HotkeyRegistry};
use crate::style::Theme;
use crate::subscription::Subscription;
use crate::widget::Widget;
use crate::widgets::visual::TerminalVisualCapabilities;
use crate::WidgetResult;

pub type EventHandler<M> = Box<dyn Fn() -> M>;
pub type FrameHandler<M> = Box<dyn Fn(FrameInfo) -> M>;
pub type SubscriptionResolver<State, M> = Box<dyn Fn(&State) -> Subscription<M>>;
pub type ThemeResolver<State> = Box<dyn Fn(&State) -> Theme>;

pub(crate) struct TickConfig<M> {
    pub(crate) interval: Duration,
    pub(crate) handler: EventHandler<M>,
}

pub(crate) struct FrameConfig<M> {
    pub(crate) handler: FrameHandler<M>,
}

/// Timing information for a rendered frame.
#[derive(Debug, Clone, Copy)]
pub struct FrameInfo {
    pub now: Instant,
    pub delta: Duration,
    pub since_start: Duration,
    pub frame: u64,
}

/// Quit key configuration for the application.
#[derive(Debug, Clone, Default)]
pub enum QuitBehavior {
    /// Default quit key (Esc).
    #[default]
    Default,
    /// Custom quit key (simple key without modifiers).
    CustomKey(KeyCode),
    /// Custom key event (with modifiers like Ctrl+C).
    CustomKeyEvent(KeyEvent),
    /// Disable built-in quit handling.
    Disabled,
}

// ---------------------------------------------------------------------------
// App – public builder API
// ---------------------------------------------------------------------------

/// Application builder. Create with [`App::new`], configure, then call `.run()`.
pub struct App<State, M = ()> {
    pub(crate) state: State,
    pub(crate) theme: Theme,
    pub(crate) theme_resolver: Option<ThemeResolver<State>>,
    pub(crate) subscriptions: Option<SubscriptionResolver<State, M>>,
    pub(crate) global_key_handlers: HashMap<KeyCode, EventHandler<M>>,
    pub(crate) hotkeys: HotkeyRegistry<M>,
    pub(crate) command_router: Option<CommandRouter<M>>,
    pub(crate) tick_handlers: Vec<TickConfig<M>>,
    pub(crate) frame_handlers: Vec<FrameConfig<M>>,
    pub(crate) quit_behavior: QuitBehavior,
    pub(crate) mouse_capture: bool,
    pub(crate) inline_mouse_mode: InlineMouseMode,
    pub(crate) motion_policy: MotionPolicy,
    pub(crate) visual_capabilities: TerminalVisualCapabilities,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) executor: Arc<dyn Executor>,
}

impl<State: std::fmt::Debug, M> std::fmt::Debug for App<State, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("state", &self.state)
            .field("quit_behavior", &self.quit_behavior)
            .finish()
    }
}

impl<State, M: Clone + std::fmt::Debug + Send + 'static> App<State, M> {
    pub fn new(state: State) -> Self {
        Self {
            state,
            theme: Theme::dark(),
            theme_resolver: None,
            subscriptions: None,
            global_key_handlers: HashMap::new(),
            hotkeys: HotkeyRegistry::new(),
            command_router: None,
            tick_handlers: Vec::new(),
            frame_handlers: Vec::new(),
            quit_behavior: QuitBehavior::default(),
            mouse_capture: true,
            inline_mouse_mode: InlineMouseMode::default(),
            motion_policy: MotionPolicy::default(),
            visual_capabilities: TerminalVisualCapabilities::default(),
            clock: Arc::new(SystemClock),
            executor: Arc::new(ThreadExecutor),
        }
    }

    /// Declare continuous event sources from current state. Stable scoped keys
    /// keep producers running across view rebuilds; removal cancels them.
    pub fn subscriptions(mut self, resolver: impl Fn(&State) -> Subscription<M> + 'static) -> Self {
        self.subscriptions = Some(Box::new(resolver));
        self
    }

    /// Configure the theme used for rendering.
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self.theme_resolver = None;
        self
    }

    /// Resolve the theme from application state before each render.
    pub fn with_theme_from<F>(mut self, resolver: F) -> Self
    where
        F: Fn(&State) -> Theme + 'static,
    {
        self.theme_resolver = Some(Box::new(resolver));
        self
    }

    /// Configure global animation behavior.
    pub fn with_motion_policy(mut self, motion_policy: MotionPolicy) -> Self {
        self.motion_policy = motion_policy;
        self
    }

    /// Configure terminal capability hints used by visual effects.
    pub fn with_visual_capabilities(mut self, capabilities: TerminalVisualCapabilities) -> Self {
        self.visual_capabilities = capabilities;
        self
    }

    /// Register a global keyboard shortcut.
    pub fn on_key<F>(mut self, key: KeyCode, handler: F) -> Self
    where
        F: Fn() -> M + 'static,
    {
        self.global_key_handlers.insert(key, Box::new(handler));
        self
    }

    /// Register a richer hotkey with metadata and modifier support.
    pub fn on_hotkey<F>(mut self, hotkey: Hotkey, handler: F) -> Self
    where
        F: Fn() -> M + 'static,
    {
        self.hotkeys.bind(hotkey, handler);
        self
    }

    /// Install a reusable hotkey registry.
    pub fn with_hotkeys(mut self, hotkeys: HotkeyRegistry<M>) -> Self {
        self.hotkeys = hotkeys;
        self
    }

    /// Install a command router that can dispatch commands from hotkeys.
    pub fn with_command_router(mut self, command_router: CommandRouter<M>) -> Self {
        self.command_router = Some(command_router);
        self
    }

    /// Register a periodic timer that emits a message at the given interval.
    ///
    /// Missed ticks are coalesced into a single message, so a busy frame does
    /// not cause a backlog of timer messages.
    pub fn on_tick<F>(mut self, interval: Duration, handler: F) -> Self
    where
        F: Fn() -> M + 'static,
    {
        let interval = if interval.is_zero() {
            Duration::from_millis(1)
        } else {
            interval
        };
        self.tick_handlers.push(TickConfig {
            interval,
            handler: Box::new(handler),
        });
        self
    }

    /// Register a callback that runs once per frame.
    ///
    /// This is a good fit for time-based animation because the callback
    /// receives real frame timing data.
    pub fn on_frame<F>(mut self, handler: F) -> Self
    where
        F: Fn(FrameInfo) -> M + 'static,
    {
        self.frame_handlers.push(FrameConfig {
            handler: Box::new(handler),
        });
        self
    }

    /// Configure the quit key.
    pub fn with_quit_key(mut self, key: KeyCode) -> Self {
        self.quit_behavior = QuitBehavior::CustomKey(key);
        self
    }

    /// Configure quit key with modifiers (e.g., Ctrl+C).
    pub fn with_quit_key_event(mut self, key_event: KeyEvent) -> Self {
        self.quit_behavior = QuitBehavior::CustomKeyEvent(key_event);
        self
    }

    /// Disable built-in quit key handling.
    pub fn disable_quit_key(mut self) -> Self {
        self.quit_behavior = QuitBehavior::Disabled;
        self
    }

    /// Enable mouse capture for wheel, click, and drag interactions.
    ///
    /// Mouse capture is enabled by default; this method is kept for explicit
    /// builder chains and backward compatibility.
    pub fn enable_mouse_capture(mut self) -> Self {
        self.mouse_capture = true;
        self
    }

    /// Disable mouse capture when an application wants keyboard-only input.
    pub fn disable_mouse_capture(mut self) -> Self {
        self.mouse_capture = false;
        self
    }

    /// Configure mouse coordinate handling for inline mode.
    pub fn with_inline_mouse_mode(mut self, mode: InlineMouseMode) -> Self {
        self.inline_mouse_mode = mode;
        self
    }

    /// Keep inline mouse interactions pinned to the live terminal viewport.
    ///
    /// This is intentionally opt-in: it emits a newline after each inline frame
    /// and moves the cursor back so terminals that follow output stay aligned
    /// with the rendered UI.
    pub fn force_inline_mouse_follow(mut self) -> Self {
        self.inline_mouse_mode = InlineMouseMode::ForceFollowViewport;
        self
    }

    /// Run the application in full-screen mode.
    pub fn run<F, V, W>(self, update: F, view: V) -> WidgetResult<()>
    where
        F: Fn(&mut State, M),
        V: Fn(&State) -> W,
        W: Widget<M> + 'static,
    {
        let view_fn = move |state: &State| -> Box<dyn Widget<M>> { Box::new(view(state)) };
        self.run_with_options(SimpleUpdate(update), view_fn, false)
    }

    /// Run the application in inline (non-fullscreen) mode.
    pub fn run_inline<F, V, W>(self, update: F, view: V) -> WidgetResult<()>
    where
        F: Fn(&mut State, M),
        V: Fn(&State) -> W,
        W: Widget<M> + 'static,
    {
        let view_fn = move |state: &State| -> Box<dyn Widget<M>> { Box::new(view(state)) };
        self.run_with_options(SimpleUpdate(update), view_fn, true)
    }

    /// Run the application with an update context that can schedule effects.
    pub fn run_with_effects<F, V, W>(self, update: F, view: V) -> WidgetResult<()>
    where
        F: Fn(&mut State, M, &mut UpdateCtx<M>),
        V: Fn(&State) -> W,
        W: Widget<M> + 'static,
    {
        let view_fn = move |state: &State| -> Box<dyn Widget<M>> { Box::new(view(state)) };
        self.run_with_options(update, view_fn, false)
    }

    /// Run the application inline with an update context that can schedule effects.
    pub fn run_inline_with_effects<F, V, W>(self, update: F, view: V) -> WidgetResult<()>
    where
        F: Fn(&mut State, M, &mut UpdateCtx<M>),
        V: Fn(&State) -> W,
        W: Widget<M> + 'static,
    {
        let view_fn = move |state: &State| -> Box<dyn Widget<M>> { Box::new(view(state)) };
        self.run_with_options(update, view_fn, true)
    }

    /// Use an injected monotonic clock (for example `ManualClock`).
    pub fn with_clock(mut self, clock: impl Clock + 'static) -> Self {
        self.clock = Arc::new(clock);
        self
    }

    /// Choose how blocking task jobs are executed.
    pub fn with_executor(mut self, executor: impl Executor + 'static) -> Self {
        self.executor = Arc::new(executor);
        self
    }

    /// Build a runtime without querying or configuring a terminal.
    pub fn build<F, V, W>(self, update: F, view: V) -> Runtime<State, M>
    where
        F: Fn(&mut State, M) + 'static,
        V: Fn(&State) -> W + 'static,
        W: Widget<M> + 'static,
    {
        self.build_with_effects(move |state, message, _ctx| update(state, message), view)
    }

    /// Build a terminal-independent runtime with the same effect handling as `run`.
    pub fn build_with_effects<F, V, W>(self, update: F, view: V) -> Runtime<State, M>
    where
        F: Fn(&mut State, M, &mut UpdateCtx<M>) + 'static,
        V: Fn(&State) -> W + 'static,
        W: Widget<M> + 'static,
    {
        Runtime::from_app(self, update, view)
    }

    fn run_with_options<U, V>(self, update: U, view: V, inline_mode: bool) -> WidgetResult<()>
    where
        U: UpdateDriver<State, M>,
        V: Fn(&State) -> Box<dyn Widget<M>>,
    {
        let mouse_capture = self.mouse_capture;
        let inline_mouse_mode = self.inline_mouse_mode;
        AppRuntime::new(self, update, view, inline_mode, 50)
            .run_terminal(mouse_capture, inline_mouse_mode)
    }
}
