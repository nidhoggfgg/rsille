//! Terminal lifecycle, batched input, and polling or wake-driven updates.
//!
//! Construct an [`EventLoop`] through [`Builder::build_event_loop`]. Synchronous
//! entry points create a Tokio runtime; asynchronous entry points use the
//! caller's runtime. Terminal modes are restored when execution ends.

#[cfg(test)]
use crossterm::event::MouseEvent;
use std::{
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::{
    cursor::{position, MoveToNextLine},
    event::{Event, EventStream, KeyEvent, KeyEventKind},
    execute,
    terminal::size as terminal_size,
};
use futures::StreamExt;
use log::{error, info, warn};
use tokio::time::Instant as TokioInstant;

use crate::render::{
    area::Size, terminal::TerminalGuard, Builder, DrawErr, DrawUpdate, InlineMouseMode,
    KeyboardMode, Render, WakeHandle,
};

#[cfg(not(test))]
type LoopOutput = std::io::Stdout;
// Exercise the real loop and renderer without writing control sequences to the
// test runner's terminal. Terminal setup is bypassed by run_loop_with_stream.
#[cfg(test)]
type LoopOutput = tests::TestOutput;

/// Owns an application, its standard-output renderer, and terminal settings.
///
/// Each iteration dispatches input and updates the application before deciding
/// whether to draw. Input, update, and drawing failures during iterations are
/// logged and the loop continues. Setup and commit-frame presentation failures
/// are returned to the caller.
/// Cleanup is best-effort, including when a running future is dropped.
pub struct EventLoop<T> {
    render: Render<LoopOutput, T>,
    raw_mode: bool,
    exit_code: Option<KeyEvent>,
    max_event_per_frame: usize,
    frame_limit: Option<u16>,
    alt_screen: bool,
    mouse_capture: bool,
    hide_cursor: bool,
    keyboard_mode: KeyboardMode,
    focus_events: bool,
    inline_mode: bool,
    inline_max_height: u16,
    inline_mouse_mode: InlineMouseMode,
    terminal_size: Size,
}

impl<T> EventLoop<T>
where
    T: DrawUpdate,
{
    pub(super) fn from_builder(builder: &Builder, thing: T) -> Self
    where
        T: DrawUpdate,
    {
        #[cfg(not(test))]
        let output = stdout();
        #[cfg(test)]
        let output = LoopOutput::default();
        Self {
            render: Render::from_builder(builder, thing, output),
            raw_mode: builder.enable_raw_mode,
            exit_code: builder.exit_code,
            max_event_per_frame: builder.max_event_per_frame,
            frame_limit: builder.frame_limit,
            alt_screen: builder.enable_alt_screen,
            mouse_capture: builder.enable_mouse_capture,
            hide_cursor: builder.enable_hide_cursor,
            keyboard_mode: builder.keyboard_mode,
            focus_events: builder.focus_events,
            inline_mode: builder.inline_mode,
            inline_max_height: builder.inline_max_height,
            inline_mouse_mode: builder.inline_mouse_mode,
            terminal_size: builder.size,
        }
    }

    /// Sets the batch limit used when draining queued input.
    ///
    /// Zero disables draining; an event that woke the loop is still dispatched.
    pub fn max_event_per_frame(&mut self, max_event_per_frame: usize) -> &mut Self {
        self.max_event_per_frame = max_event_per_frame;
        self
    }

    /// Sets the periodic update rate in frames per second.
    ///
    /// `None` and `Some(0)` disable periodic polling. Input may trigger earlier
    /// iterations, so this is not a strict redraw limit. Applications returning
    /// a [`crate::render::Update::wake_handle`] use their own deadlines instead.
    pub fn frame_limit(&mut self, frame_limit: Option<u16>) -> &mut Self {
        self.frame_limit = frame_limit;
        self
    }

    /// Sets the automatic exit key, matching its code and modifiers on press.
    ///
    /// Repeats and releases do not exit, and the matching press is not passed
    /// to the application. The default key is Escape.
    pub fn exit_code(&mut self, exit_code: KeyEvent) -> &mut Self {
        self.exit_code = Some(exit_code);
        self
    }

    /// Disables automatic key-based exit so the application controls shutdown.
    pub fn disable_exit_code(&mut self) -> &mut Self {
        self.exit_code = None;
        self
    }

    /// Enters and clears the alternate screen during startup.
    pub fn enable_alt_screen(&mut self) -> &mut Self {
        self.alt_screen = true;
        self
    }

    /// Keeps rendering on the current terminal screen.
    pub fn disable_alt_screen(&mut self) -> &mut Self {
        self.alt_screen = false;
        self
    }

    /// Enables terminal mouse reporting during startup.
    pub fn enable_mouse_capture(&mut self) -> &mut Self {
        self.mouse_capture = true;
        self
    }

    /// Leaves terminal mouse reporting disabled during startup.
    pub fn disable_mouse_capture(&mut self) -> &mut Self {
        self.mouse_capture = false;
        self
    }

    /// Hides the cursor at startup and shows it again during cleanup.
    pub fn hide_cursor_when_render(&mut self) -> &mut Self {
        self.hide_cursor = true;
        self
    }

    /// Leaves cursor visibility unchanged during startup.
    pub fn show_cursor_when_render(&mut self) -> &mut Self {
        self.hide_cursor = false;
        self
    }

    /// Set initial used height for inline mode
    ///
    /// This should be called before run() to set the initial rendering height
    /// without reallocating the buffer. Only effective in inline mode.
    /// Values greater than the initial buffer height are clamped.
    pub fn set_initial_used_height(&mut self, height: u16) -> &mut Self {
        self.render.set_used_height(height);
        self
    }

    /// Reject synchronous entry inside Tokio before changing terminal state.
    ///
    /// # Errors
    ///
    /// Returns [`DrawErr::RuntimeCreation`] if called within a Tokio runtime.
    pub fn check_sync_runtime() -> Result<(), DrawErr> {
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(DrawErr::RuntimeCreation(std::io::Error::other(
                "synchronous terminal entry inside Tokio; use run_async instead",
            )));
        }
        Ok(())
    }

    /// Runs synchronously until exit, discarding the application after cleanup.
    ///
    /// # Errors
    ///
    /// Returns the same runtime, terminal setup, and startup I/O errors as
    /// [`run_returning`](Self::run_returning). Use [`run_async`](Self::run_async)
    /// when already inside Tokio.
    pub fn run(self) -> Result<(), DrawErr> {
        self.run_returning().map(|_| ())
    }

    /// Run and return the application after terminal cleanup.
    ///
    /// Creates a current-thread Tokio runtime with the time driver enabled.
    ///
    /// # Errors
    ///
    /// Returns [`DrawErr::RuntimeCreation`] when already inside Tokio or runtime
    /// creation fails, [`DrawErr::TerminalSetup`] on terminal setup failure,
    /// or [`DrawErr::Io`] if inline cursor positioning fails.
    pub fn run_returning(self) -> Result<T, DrawErr> {
        Self::check_sync_runtime()?;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(DrawErr::RuntimeCreation)?;
        rt.block_on(self.run_async_returning())
    }

    /// Run on an existing Tokio runtime with the time driver enabled.
    ///
    /// Discards the application after cleanup. Returns the same startup errors
    /// as [`run_async_returning`](Self::run_async_returning).
    ///
    /// # Panics
    ///
    /// May panic if polled outside a Tokio runtime or with its time driver disabled.
    pub async fn run_async(self) -> Result<(), DrawErr> {
        self.run_async_returning().await.map(|_| ())
    }

    /// Run on the caller's runtime and return the application after cleanup.
    ///
    /// Enable the runtime's time driver with `tokio::runtime::Builder::enable_time`.
    /// Terminal input uses Crossterm's reader thread and does not require Tokio's
    /// I/O driver. Actual enabled input capabilities are delivered to the
    /// application before event dispatch.
    ///
    /// # Errors
    ///
    /// Returns [`DrawErr::TerminalSetup`] on setup failure or [`DrawErr::Io`]
    /// on failed inline cursor positioning. Successfully enabled terminal
    /// modes are restored on return, including after partial setup failure.
    ///
    /// # Panics
    ///
    /// May panic if polled outside a Tokio runtime or with its time driver disabled.
    pub async fn run_async_returning(mut self) -> Result<T, DrawErr> {
        let guard = TerminalGuard::new(
            self.alt_screen,
            self.raw_mode,
            self.mouse_capture,
            self.hide_cursor,
            self.keyboard_mode,
            self.focus_events,
        )?;
        self.render.set_alternate_screen(self.alt_screen);
        self.render
            .thing_mut()
            .on_input_capabilities(guard.input_capabilities());

        info!(
            target: "render::event_loop",
            "event loop started: mode={}, frame_limit={:?}, raw_mode={}, alt_screen={}",
            if self.inline_mode { "inline" } else { "fullscreen" },
            self.frame_limit,
            self.raw_mode,
            self.alt_screen
        );

        if self.inline_mode {
            if let Ok((width, height)) = terminal_size() {
                self.apply_terminal_size(width, height, true);
            }

            match terminal_cursor_position() {
                Ok((x, y)) => {
                    self.initialize_inline_origin(x, y, &mut stdout())?;
                }
                Err(e) => {
                    warn!(
                        "Failed to get cursor position in inline mode: {}, using (0, 0)",
                        e
                    );
                    self.render.pos = (0, 0).into();
                }
            }
        }

        self.run_loop().await?;
        info!(target: "render::event_loop", "event loop stopped");
        Ok(self.render.thing)
    }

    async fn run_loop(&mut self) -> Result<(), DrawErr> {
        super::terminal_probe::reserve_input()?;
        self.run_loop_with_stream(&mut EventStream::new()).await
    }

    async fn run_loop_with_stream<S>(&mut self, reader: &mut S) -> Result<(), DrawErr>
    where
        S: futures::Stream<Item = std::io::Result<Event>> + Unpin,
    {
        let frame_interval = self
            .frame_limit
            .filter(|fps| *fps > 0)
            .map(|fps| Duration::from_secs_f64(1.0 / fps as f64));
        let mut pending_initial_frame = true;
        let mut next_frame_at = Instant::now();
        let wake = self.render.thing().wake_handle();
        let mut pending_events = std::collections::VecDeque::new();
        let mut resume_after_commit = false;
        let mut input_closed = false;
        let mut pending_render = false;

        loop {
            let mut stop_after_iteration = false;
            let mut events = Vec::new();

            match wait_for_frame_or_event(
                reader,
                self.exit_code,
                if wake.is_some() {
                    self.render.thing().next_update_at()
                } else {
                    frame_interval.map(|_| next_frame_at)
                },
                wake.as_ref(),
                pending_initial_frame
                    || resume_after_commit
                    || !pending_events.is_empty()
                    || input_closed
                    || self.render.thing().frame_commit_pending(),
            )
            .await
            {
                WaitOutcome::Event(event) => events.push(event),
                WaitOutcome::Timeout => {}
                WaitOutcome::ExitRequested => {
                    info!(
                        target: "render::event_loop",
                        "Automatic exit key pressed"
                    );
                    break;
                }
                WaitOutcome::Disconnected => break,
            }

            let drained = if pending_events.is_empty()
                && !resume_after_commit
                && !input_closed
                && !self.render.thing().frame_commit_pending()
            {
                drain_pending_events(
                    reader,
                    self.exit_code,
                    &mut events,
                    self.max_event_per_frame,
                )
                .await
            } else {
                DrainOutcome::Continue
            };
            match drained {
                DrainOutcome::Continue => {}
                DrainOutcome::ExitRequested => {
                    info!(
                        target: "render::event_loop",
                        "Automatic exit key pressed"
                    );
                    stop_after_iteration = true;
                }
                DrainOutcome::Disconnected => {
                    stop_after_iteration = true;
                }
            }

            pending_initial_frame = false;
            let frame_started_at = Instant::now();

            input_closed |= stop_after_iteration;
            pending_events.extend(events);
            let resuming = std::mem::take(&mut resume_after_commit);
            let events: Vec<_> = if resuming || self.render.thing().frame_commit_pending() {
                Vec::new()
            } else if self.render.thing().event_frame_boundaries() {
                pending_events.pop_front().into_iter().collect()
            } else {
                pending_events.drain(..).collect()
            };
            for event in &events {
                if let Event::Resize(width, height) = event {
                    self.apply_terminal_size(*width, *height, false);
                }
            }

            let has_events = if self.render.thing().frame_commit_pending() {
                false
            } else {
                self.dispatch_events(events)
            };
            let needs_render = match self.render.update() {
                Ok(needs_render) => needs_render,
                Err(e) => {
                    error!("Error updating render state: {}", e);
                    false
                }
            };

            if self.render.thing().should_quit() {
                info!(
                    target: "render::event_loop",
                    "Application requested quit"
                );
                break;
            }

            let committing = self.render.thing().frame_commit_pending();
            if committing && !self.inline_mode {
                return Err(DrawErr::Io(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "frame commits require inline terminal mode",
                )));
            }
            let current_size = self.render.size();
            if let Some(new_size) = self.render.thing().required_size(current_size) {
                if self.inline_mode {
                    self.render.set_used_height(new_size.height);
                } else {
                    self.render.resize(new_size);
                }
            }

            let force_inline_frame =
                self.inline_mode && self.inline_mouse_mode.force_follows_viewport();

            pending_render |= has_events
                || needs_render
                || self.render.has_pending_changes()
                || force_inline_frame;
            if committing || (pending_render && (resuming || pending_events.is_empty())) {
                if let Err(e) = self.render.render() {
                    if committing || e.kind() == std::io::ErrorKind::Unsupported {
                        // Strict raster policy is an actionable configuration
                        // error, not a transient frame failure to log forever.
                        return Err(DrawErr::Io(e));
                    }
                    error!("Error rendering: {}", e);
                } else {
                    pending_render = false;
                    if self.can_force_inline_viewport_follow() && !committing {
                        if let Err(e) = self.render.force_inline_viewport_follow() {
                            error!("Error following inline viewport: {}", e);
                        }
                    }
                    self.align_inline_origin_after_render();
                    if committing {
                        // Never acknowledge a partial write or flush failure.
                        self.render.commit_frame()?;
                        self.render.thing_mut().frame_committed();
                        resume_after_commit = true;
                    }
                }
            }

            if input_closed && pending_events.is_empty() && !resume_after_commit {
                break;
            }

            if let Some(interval) = frame_interval {
                if next_frame_at <= frame_started_at {
                    next_frame_at = frame_started_at + interval;
                }

                let finished_at = Instant::now();
                while next_frame_at <= finished_at {
                    next_frame_at += interval;
                }
            }
        }

        Ok(())
    }

    fn initialize_inline_origin(
        &mut self,
        x: u16,
        y: u16,
        output: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        self.render.pos = (x, y).into();
        if x != 0 {
            execute!(output, MoveToNextLine(1))?;
            self.render.pos.x = 0;
            self.render.pos.down(1);
        }
        Ok(())
    }

    fn apply_terminal_size(&mut self, width: u16, height: u16, initializing: bool) {
        self.terminal_size = (width, height).into();
        if self.inline_mode {
            let initial_size = self.render.size();
            let initial_used = self.render.inline_used_height();
            self.render.set_inline_viewport_height(height);
            if initializing {
                // Builder::size owns the initial allocation; the height cap is
                // a resize policy. Startup only clips for physical row space.
                let available = self.inline_available_height(height);
                self.render
                    .resize((initial_size.width, initial_size.height.min(available)).into());
                self.render.set_used_height(initial_used);
            } else {
                self.render
                    .resize((width, self.inline_buffer_height(height)).into());
            }
        } else {
            self.render.resize(
                (
                    width.saturating_sub(self.render.pos.x),
                    height.saturating_sub(self.render.pos.y),
                )
                    .into(),
            );
        }
    }

    fn can_force_inline_viewport_follow(&self) -> bool {
        self.inline_mode
            && self.inline_mouse_mode.force_follows_viewport()
            // The extra newline must not scroll a full-height UI off screen.
            && self.terminal_size.height > self.render.inline_used_height()
    }

    fn inline_available_height(&self, viewport_height: u16) -> u16 {
        if self.inline_mouse_mode.force_follows_viewport() {
            viewport_height.saturating_sub(1).max(1)
        } else {
            viewport_height
        }
    }

    fn inline_buffer_height(&self, viewport_height: u16) -> u16 {
        self.inline_max_height
            .min(self.inline_available_height(viewport_height))
    }

    fn align_inline_origin_after_render(&mut self) {
        if !self.inline_mode {
            return;
        }

        let used_height = self.render.inline_used_height();
        let bottom_padding = self
            .render
            .inline_bottom_padding()
            .max(u16::from(self.can_force_inline_viewport_follow()));
        let reserved_height = used_height.saturating_add(bottom_padding);
        let max_origin_y = self.terminal_size.height.saturating_sub(reserved_height);

        if self.render.inline_origin().y > max_origin_y {
            self.render.set_inline_origin_y(max_origin_y);
        }
    }

    fn dispatch_events(&mut self, events: Vec<Event>) -> bool {
        if self.inline_mode {
            let origin = self.render.inline_origin();
            let viewport =
                Size::from((self.render.inline_width(), self.render.inline_used_height()));
            match self.render.thing_mut().on_events_in_viewport(
                &events,
                (origin.x, origin.y),
                viewport,
            ) {
                Ok(changed) => changed,
                Err(e) => {
                    error!("Error processing events: {}", e);
                    false
                }
            }
        } else {
            if let Err(e) = self.render.on_events(&events) {
                error!("Error processing events: {}", e);
            }
            !events.is_empty()
        }
    }

    #[cfg(test)]
    fn normalize_inline_mouse_event(&self, event: Event) -> Option<Event> {
        let Event::Mouse(mouse) = event else {
            return Some(event);
        };

        let origin = self.render.inline_origin();
        if mouse.column < origin.x || mouse.row < origin.y {
            return None;
        }

        let column = mouse.column - origin.x;
        let row = mouse.row - origin.y;
        if column >= self.render.inline_width() || row >= self.render.inline_used_height() {
            return None;
        }

        Some(Event::Mouse(MouseEvent {
            kind: mouse.kind,
            column,
            row,
            modifiers: mouse.modifiers,
        }))
    }
}

fn terminal_cursor_position() -> std::io::Result<(u16, u16)> {
    super::terminal_probe::reserve_input()?;
    position()
}

enum WaitOutcome {
    Event(Event),
    Timeout,
    Disconnected,
    ExitRequested,
}

enum DrainOutcome {
    Continue,
    Disconnected,
    ExitRequested,
}

async fn wait_for_frame_or_event<S>(
    reader: &mut S,
    exit_code: Option<KeyEvent>,
    deadline: Option<Instant>,
    wake: Option<&WakeHandle>,
    pending_initial_frame: bool,
) -> WaitOutcome
where
    S: futures::Stream<Item = std::io::Result<Event>> + Unpin,
{
    if pending_initial_frame {
        return WaitOutcome::Timeout;
    }

    loop {
        let result = tokio::select! {
            event = reader.next() => event,
            _ = wait_for_deadline(deadline) => return WaitOutcome::Timeout,
            _ = wait_for_wake(wake) => return WaitOutcome::Timeout,
        };
        match result {
            Some(Ok(event)) => {
                if is_exit_event(&event, exit_code) {
                    return WaitOutcome::ExitRequested;
                }
                return WaitOutcome::Event(event);
            }
            Some(Err(e)) => error!("Error reading event: {}", e),
            None => return WaitOutcome::Disconnected,
        }
    }
}

async fn wait_for_deadline(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(TokioInstant::from_std(deadline)).await,
        None => std::future::pending().await,
    }
}

async fn wait_for_wake(wake: Option<&WakeHandle>) {
    match wake {
        Some(wake) => wake.notified().await,
        None => std::future::pending().await,
    }
}

async fn drain_pending_events<S>(
    reader: &mut S,
    exit_code: Option<KeyEvent>,
    events: &mut Vec<Event>,
    max_event_per_frame: usize,
) -> DrainOutcome
where
    S: futures::Stream<Item = std::io::Result<Event>> + Unpin,
{
    // Poll with the event loop's actual task waker. `now_or_never()` installs a
    // noop waker; EventStream can retain that waker while its reader thread is
    // blocked, losing the next keyboard wake when there is no frame timer.
    futures::future::poll_fn(|cx| {
        while events.len() < max_event_per_frame {
            match reader.poll_next_unpin(cx) {
                std::task::Poll::Ready(Some(Ok(event))) => {
                    if is_exit_event(&event, exit_code) {
                        return std::task::Poll::Ready(DrainOutcome::ExitRequested);
                    }
                    events.push(event);
                }
                std::task::Poll::Ready(Some(Err(e))) => error!("Error reading event: {}", e),
                std::task::Poll::Ready(None) => {
                    warn!("Event stream ended");
                    return std::task::Poll::Ready(DrainOutcome::Disconnected);
                }
                std::task::Poll::Pending => break,
            }
        }
        std::task::Poll::Ready(DrainOutcome::Continue)
    })
    .await
}

fn is_exit_event(event: &Event, exit_code: Option<KeyEvent>) -> bool {
    matches!(
        (event, exit_code),
        (Event::Key(key_event), Some(exit_key))
            if key_event.kind == KeyEventKind::Press
                && key_event.code == exit_key.code
                && key_event.modifiers == exit_key.modifiers
    )
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEventKind};
    use futures::FutureExt;

    use super::*;
    use crate::render::{area::Size, chunk::Chunk, Draw, Update};

    #[derive(Clone, Default)]
    pub(super) struct TestOutput(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for TestOutput {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn exit_ignores_releases_repeats_and_keyboard_state_bits() {
        use crossterm::event::KeyEventState;
        let exit = KeyEvent::new(crossterm::event::KeyCode::Esc, KeyModifiers::empty());
        for kind in [
            KeyEventKind::Press,
            KeyEventKind::Repeat,
            KeyEventKind::Release,
        ] {
            let event = Event::Key(KeyEvent {
                kind,
                state: KeyEventState::CAPS_LOCK,
                ..exit
            });
            assert_eq!(
                is_exit_event(&event, Some(exit)),
                kind == KeyEventKind::Press
            );
        }
    }

    #[tokio::test]
    async fn idle_wait_stays_pending_until_a_producer_wakes_it() {
        let mut input = futures::stream::pending();
        let wake = WakeHandle::new();
        assert!(
            wait_for_frame_or_event(&mut input, None, None, Some(&wake), false)
                .now_or_never()
                .is_none()
        );

        // Enqueue-before-wait is the important race: the permit must survive.
        wake.wake();
        assert!(matches!(
            wait_for_frame_or_event(&mut input, None, None, Some(&wake), false).await,
            WaitOutcome::Timeout
        ));
        assert!(
            wait_for_frame_or_event(&mut input, None, None, Some(&wake), false)
                .now_or_never()
                .is_none()
        );
    }

    #[tokio::test]
    async fn scheduled_deadline_wakes_without_terminal_input() {
        let mut input = futures::stream::pending();
        assert!(matches!(
            wait_for_frame_or_event(&mut input, None, Some(Instant::now()), None, false).await,
            WaitOutcome::Timeout
        ));
    }

    #[tokio::test]
    async fn input_and_exit_keys_work_without_periodic_frames() {
        let key = KeyEvent::new(crossterm::event::KeyCode::Char('a'), KeyModifiers::empty());
        let mut input = futures::stream::iter([Ok(Event::Key(key))]);
        assert!(matches!(
            wait_for_frame_or_event(&mut input, None, None, None, false).await,
            WaitOutcome::Event(Event::Key(event)) if event == key
        ));
        let mut input = futures::stream::iter([Ok(Event::Key(key))]);
        assert!(matches!(
            wait_for_frame_or_event(&mut input, Some(key), None, None, false).await,
            WaitOutcome::ExitRequested
        ));
    }

    #[tokio::test]
    async fn draining_input_preserves_the_waker_used_by_a_background_reader() {
        use std::sync::{Arc, Mutex};
        use std::task::{Poll, Waker};

        #[derive(Default)]
        struct InputState {
            event: Option<Event>,
            waker: Option<Waker>,
        }
        // Like EventStream's blocked reader, retain the first registered waker
        // until an event arrives instead of replacing it on every poll.
        struct Input(Arc<Mutex<InputState>>);
        impl futures::Stream for Input {
            type Item = std::io::Result<Event>;
            fn poll_next(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                let mut state = self.0.lock().unwrap();
                if let Some(event) = state.event.take() {
                    Poll::Ready(Some(Ok(event)))
                } else {
                    state.waker.get_or_insert_with(|| cx.waker().clone());
                    Poll::Pending
                }
            }
        }

        let state = Arc::new(Mutex::new(InputState::default()));
        let mut input = Input(state.clone());
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let reader = tokio::spawn(async move {
            let mut events = Vec::new();
            drain_pending_events(&mut input, None, &mut events, 10).await;
            ready_tx.send(()).unwrap();
            matches!(
                wait_for_frame_or_event(&mut input, None, None, None, false).await,
                WaitOutcome::Event(Event::Key(_))
            )
        });
        ready_rx.await.unwrap();
        {
            let mut state = state.lock().unwrap();
            state.event = Some(Event::Key(crossterm::event::KeyCode::Esc.into()));
            state.waker.take().unwrap().wake();
        }
        assert!(tokio::time::timeout(Duration::from_secs(1), reader)
            .await
            .unwrap()
            .unwrap());
    }

    #[derive(Default)]
    struct BoundaryProbe {
        checkpoints: bool,
        pending: bool,
        resume: bool,
        commits: usize,
        fail_draw: bool,
        received: String,
        state: String,
        batches: Vec<usize>,
        draws: Vec<String>,
    }

    impl Draw for BoundaryProbe {
        fn draw(&mut self, _: Chunk) -> Result<Size, DrawErr> {
            if self.fail_draw {
                return Err(DrawErr::Io(std::io::Error::other("injected draw failure")));
            }
            self.draws.push(self.state.clone());
            Ok((0, 0).into())
        }
    }

    impl Update for BoundaryProbe {
        fn on_events(&mut self, events: &[Event]) -> Result<(), DrawErr> {
            if !events.is_empty() {
                self.batches.push(events.len());
            }
            for event in events {
                if let Event::Key(KeyEvent {
                    code: crossterm::event::KeyCode::Char(c),
                    ..
                }) = event
                {
                    assert!(!self.pending && !self.resume, "input crossed a boundary");
                    self.received.push(*c);
                    self.state = c.to_string();
                    if *c == 'c' {
                        self.pending = true;
                    }
                }
            }
            Ok(())
        }
        fn update(&mut self) -> Result<bool, DrawErr> {
            if std::mem::take(&mut self.resume) {
                self.state = "prompt".into();
            }
            Ok(true)
        }
        fn event_frame_boundaries(&self) -> bool {
            self.checkpoints
        }
        fn frame_commit_pending(&self) -> bool {
            self.pending
        }
        fn frame_committed(&mut self) {
            assert!(self.pending);
            assert_eq!(self.draws.last(), Some(&self.state));
            self.pending = false;
            self.resume = true;
            self.commits += 1;
        }
    }

    fn boundary_loop(checkpoints: bool) -> EventLoop<BoundaryProbe> {
        Builder::new()
            .inline_mode(true)
            .size((0, 0))
            .clear(false)
            .build_event_loop(BoundaryProbe {
                checkpoints,
                ..Default::default()
            })
    }

    fn key_event(c: char) -> Event {
        Event::Key(KeyEvent::new(
            crossterm::event::KeyCode::Char(c),
            KeyModifiers::NONE,
        ))
    }

    #[tokio::test]
    async fn ordinary_events_keep_batch_delivery_and_coalesce_checkpoint_draws() {
        for checkpoints in [false, true] {
            let mut event_loop = boundary_loop(checkpoints);
            let mut reader = futures::stream::iter(['a', 'b', 'd'].map(|c| Ok(key_event(c))));
            event_loop.run_loop_with_stream(&mut reader).await.unwrap();
            let app = event_loop.render.thing();
            assert_eq!(app.received, "abd");
            assert_eq!(app.draws, ["d"]);
            assert_eq!(
                app.batches,
                if checkpoints { vec![1, 1, 1] } else { vec![3] }
            );
        }
    }

    #[tokio::test]
    async fn drained_exit_or_eof_preserves_all_prior_commits_and_continuations() {
        for exit_key in [false, true] {
            let mut event_loop = boundary_loop(true);
            let exit = KeyEvent::new(crossterm::event::KeyCode::Esc, KeyModifiers::NONE);
            event_loop.exit_code = Some(exit);
            let mut events = vec![Ok(key_event('c')), Ok(key_event('c')), Ok(key_event('a'))];
            if exit_key {
                events.extend([Ok(Event::Key(exit)), Ok(key_event('z'))]);
            }
            let mut reader = futures::stream::iter(events);
            event_loop.run_loop_with_stream(&mut reader).await.unwrap();
            let app = event_loop.render.thing();
            assert_eq!(app.received, "cca");
            assert_eq!(app.commits, 2);
            assert_eq!(app.draws, ["c", "prompt", "c", "prompt", "a"]);
        }
    }

    #[tokio::test]
    async fn drained_exit_does_not_hide_commit_failure_or_run_continuation() {
        let mut event_loop = boundary_loop(true);
        event_loop.render.thing_mut().fail_draw = true;
        let exit = KeyEvent::new(crossterm::event::KeyCode::Esc, KeyModifiers::NONE);
        event_loop.exit_code = Some(exit);
        let mut reader = futures::stream::iter([Ok(key_event('c')), Ok(Event::Key(exit))]);
        assert!(event_loop.run_loop_with_stream(&mut reader).await.is_err());
        let app = event_loop.render.thing();
        assert_eq!(app.commits, 0);
        assert!(app.pending);
        assert!(!app.resume);
        assert_eq!(app.state, "c");
    }

    #[tokio::test]
    async fn already_pending_commit_is_presented_before_reading_exit() {
        let mut event_loop = boundary_loop(true);
        event_loop.render.thing_mut().pending = true;
        event_loop.render.thing_mut().state = "result".into();
        let exit = KeyEvent::new(crossterm::event::KeyCode::Esc, KeyModifiers::NONE);
        event_loop.exit_code = Some(exit);
        let mut reader = futures::stream::iter([Ok(Event::Key(exit))]);
        event_loop.run_loop_with_stream(&mut reader).await.unwrap();
        let app = event_loop.render.thing();
        assert_eq!(app.commits, 1);
        assert_eq!(app.draws, ["result", "prompt"]);
        assert!(app.received.is_empty());
    }

    struct Noop;

    impl Draw for Noop {
        fn draw(&mut self, _chunk: Chunk) -> Result<Size, DrawErr> {
            Ok((0, 0).into())
        }
    }

    impl Update for Noop {
        fn on_events(&mut self, _events: &[Event]) -> Result<(), DrawErr> {
            Ok(())
        }

        fn update(&mut self) -> Result<bool, DrawErr> {
            Ok(false)
        }
    }

    fn inline_loop<T: DrawUpdate>(thing: T) -> EventLoop<T> {
        let mut builder = Builder::new();
        builder.inline_mode(true).size((20, 5));
        let mut event_loop = builder.build_event_loop(thing);
        event_loop.render.pos = (2, 3).into();
        event_loop.render.set_used_height(4);
        event_loop
    }

    #[test]
    fn inline_resize_respects_viewport_reservation_and_height_cap() {
        for (mode, viewport, cap, expected) in [
            (InlineMouseMode::ForceFollowViewport, 8, 50, 7),
            (InlineMouseMode::ForceFollowViewport, 8, 3, 3),
            (InlineMouseMode::ForceFollowViewport, 1, 50, 1),
            (InlineMouseMode::Local, 8, 50, 8),
        ] {
            let mut event_loop = inline_loop(Noop);
            event_loop.inline_mouse_mode = mode;
            event_loop.inline_max_height = cap;
            event_loop.apply_terminal_size(30, viewport, false);
            assert_eq!(event_loop.terminal_size, (30, viewport).into());
            assert_eq!(event_loop.render.size(), (30, expected).into());
            assert_eq!(event_loop.render.inline_used_height(), expected);
            event_loop.apply_terminal_size(20, 2, false);
            event_loop.apply_terminal_size(30, viewport, false);
            assert_eq!(event_loop.render.size(), (30, expected).into());
        }
    }

    #[test]
    fn inline_startup_preserves_builder_size_and_initial_used_height() {
        for (mode, height, used, expected_height, expected_used) in [
            (InlineMouseMode::Local, 5, 3, 5, 3),
            (InlineMouseMode::ForceFollowViewport, 5, 3, 5, 3),
            (InlineMouseMode::ForceFollowViewport, 10, 10, 7, 7),
            (InlineMouseMode::Local, 10, 10, 8, 8),
        ] {
            let mut event_loop = Builder::new()
                .inline_mode(true)
                .inline_mouse_mode(mode)
                .inline_max_height(2)
                .size((12, height))
                .build_event_loop(Noop);
            event_loop.render.set_used_height(used);
            event_loop.apply_terminal_size(80, 8, true);
            assert_eq!(event_loop.render.size(), (12, expected_height).into());
            assert_eq!(event_loop.render.inline_used_height(), expected_used);
            event_loop.apply_terminal_size(80, 8, false);
            assert_eq!(event_loop.render.size(), (80, 2).into());
        }
    }

    #[cfg(feature = "sixel")]
    #[tokio::test]
    async fn initial_full_height_raster_force_follow_preserves_identical_frame() {
        use crate::render::raster::{RasterCanvas, RasterCell, RasterOptions, RasterScale, Rgb};
        use crate::render::raster_output::{CellPixels, RasterCapabilities, RasterOutput};

        struct FullRaster {
            canvas: RasterCanvas,
            draws: usize,
        }
        impl Draw for FullRaster {
            fn draw(&mut self, mut chunk: Chunk) -> Result<Size, DrawErr> {
                self.draws += 1;
                let target = chunk.area().size();
                for y in 0..target.height {
                    for x in 0..target.width {
                        chunk.set_raster_cell(
                            x,
                            y,
                            RasterCell {
                                canvas: self.canvas.clone(),
                                target,
                                x,
                                y,
                                options: RasterOptions {
                                    scale: RasterScale::Stretch,
                                    ..Default::default()
                                },
                            },
                        )?;
                    }
                }
                Ok(target)
            }
        }
        impl Update for FullRaster {
            fn on_events(&mut self, _: &[Event]) -> Result<(), DrawErr> {
                Ok(())
            }
            fn update(&mut self) -> Result<bool, DrawErr> {
                Ok(false)
            }
        }

        // This small terminal tracks graphics by row. DCS bodies are consumed
        // separately so Sixel data cannot be mistaken for text or cursor commands.
        struct Terminal {
            y: usize,
            saved: usize,
            rows: Vec<bool>,
            tiles: usize,
            lost_rows: usize,
            line_feeds: usize,
        }
        impl Terminal {
            fn read(&mut self, bytes: &[u8]) {
                let mut i = 0;
                while i < bytes.len() {
                    match bytes[i] {
                        b'\r' => i += 1,
                        b'\n' => {
                            self.line_feeds += 1;
                            if self.y + 1 == self.rows.len() {
                                self.lost_rows += usize::from(self.rows[0]);
                                self.rows.rotate_left(1);
                                *self.rows.last_mut().unwrap() = false;
                            } else {
                                self.y += 1;
                            }
                            i += 1;
                        }
                        0x1b => {
                            let command = bytes[i + 1];
                            i += 2;
                            match command {
                                b'7' => self.saved = self.y,
                                b'8' => self.y = self.saved,
                                b'P' => {
                                    let end = i + bytes[i..]
                                        .windows(2)
                                        .position(|w| w == b"\x1b\\")
                                        .unwrap();
                                    let body = &bytes[i..end];
                                    let attr = body.iter().position(|b| *b == b'"').unwrap() + 1;
                                    let dims: Vec<usize> = body[attr..]
                                        .split(|b| !b.is_ascii_digit() && *b != b';')
                                        .next()
                                        .unwrap()
                                        .split(|b| *b == b';')
                                        .map(|n| std::str::from_utf8(n).unwrap().parse().unwrap())
                                        .collect();
                                    assert_eq!(
                                        dims[3] % 18,
                                        0,
                                        "8x18 cells have no partial sixel bands"
                                    );
                                    let bottom = self.y + dims[3] / 18;
                                    assert!(
                                        bottom <= self.rows.len(),
                                        "tile must fit in physical viewport"
                                    );
                                    self.rows[self.y..bottom].fill(true);
                                    self.tiles += 1;
                                    i = end + 2;
                                }
                                b'[' => {
                                    let start = i;
                                    while !(0x40..=0x7e).contains(&bytes[i]) {
                                        i += 1;
                                    }
                                    let params = std::str::from_utf8(&bytes[start..i]).unwrap();
                                    let count = params.parse::<usize>().unwrap_or(1).max(1);
                                    match bytes[i] {
                                        b'F' => self.y = self.y.saturating_sub(count),
                                        b'E' => self.y = (self.y + count).min(self.rows.len() - 1),
                                        b'K' => {
                                            assert_eq!(params, "2");
                                            self.rows[self.y] = false;
                                        }
                                        b'm' | b'G' | b'h' | b'l' => {}
                                        other => panic!("unexpected CSI command {other}"),
                                    }
                                    i += 1;
                                }
                                other => panic!("unexpected escape {other}"),
                            }
                        }
                        byte => {
                            assert_eq!(byte, b' ');
                            i += 1;
                        }
                    }
                }
            }
        }

        for initial_y in [0, 5] {
            let mut builder = Builder::new();
            builder
                .inline_mode(true)
                .size((4, 6))
                .force_inline_mouse_follow()
                .raster_output(RasterOutput {
                    capabilities: RasterCapabilities {
                        sixel: true,
                        reliable_erase: false,
                        reliable_inline_erase: true,
                        no_scroll: true,
                        cell_pixels: Some(CellPixels {
                            width: 8,
                            height: 18,
                        }),
                    },
                });
            let output = TestOutput::default();
            let mut event_loop = builder.build_event_loop(FullRaster {
                canvas: RasterCanvas::new(4, 6, Rgb(255, 0, 0)).unwrap(),
                draws: 0,
            });
            event_loop.render = builder.build_render(
                FullRaster {
                    canvas: RasterCanvas::new(4, 6, Rgb(255, 0, 0)).unwrap(),
                    draws: 0,
                },
                output.clone(),
            );
            event_loop.apply_terminal_size(20, 6, true);
            event_loop
                .initialize_inline_origin(0, initial_y, &mut Vec::new())
                .unwrap();
            assert_eq!(event_loop.render.size(), (4, 5).into());
            let mut terminal = Terminal {
                y: initial_y as usize,
                saved: 0,
                rows: vec![false; 6],
                tiles: 0,
                lost_rows: 0,
                line_feeds: 0,
            };

            // EOF presents one frame; a fresh stream presents an identical one
            // through the same event-loop/render/force-follow path.
            let mut first_stats = None;
            for frame in 0..2 {
                let mut reader = futures::stream::empty();
                event_loop.run_loop_with_stream(&mut reader).await.unwrap();
                let bytes = std::mem::take(&mut *output.0.lock().unwrap());
                assert!(bytes.ends_with(b"\r\n\x1b[1F"), "force follow must run");
                let old_lfs = terminal.line_feeds;
                terminal.read(&bytes);
                assert!(terminal.line_feeds > old_lfs);
                assert_eq!(terminal.rows, [true, true, true, true, true, false]);
                assert_eq!(terminal.lost_rows, 0, "no painted top row may scroll away");
                assert_eq!(event_loop.render.inline_origin().y, 0);
                let stats = event_loop.render.raster_stats();
                if frame == 0 {
                    assert!(terminal.tiles > 0);
                    assert!(stats.tiles_sent > 0);
                    first_stats = Some((
                        stats.encodes,
                        stats.tiles_sent,
                        stats.bytes_sent,
                        terminal.tiles,
                    ));
                } else {
                    assert_eq!(
                        Some((
                            stats.encodes,
                            stats.tiles_sent,
                            stats.bytes_sent,
                            terminal.tiles
                        )),
                        first_stats,
                        "identical frame must not encode or resend tiles"
                    );
                }
            }
            assert_eq!(event_loop.render.thing().draws, 2);
        }
    }

    #[test]
    fn force_follow_requires_a_spare_physical_row() {
        let mut event_loop = inline_loop(Noop);
        event_loop.inline_mouse_mode = InlineMouseMode::ForceFollowViewport;
        for (viewport, used, expected) in
            [(1, 1, false), (4, 4, false), (4, 5, false), (4, 3, true)]
        {
            event_loop.terminal_size = (20, viewport).into();
            event_loop.render.set_used_height(used);
            assert_eq!(event_loop.can_force_inline_viewport_follow(), expected);
        }
        event_loop.inline_mouse_mode = InlineMouseMode::Local;
        assert!(!event_loop.can_force_inline_viewport_follow());
    }

    #[test]
    fn inline_startup_next_line_resets_the_column() {
        for (x, y, expected, bytes) in [
            (7, 3, (0, 4), b"\x1b[1E".as_slice()),
            (0, 3, (0, 3), b"".as_slice()),
        ] {
            let mut event_loop = inline_loop(Noop);
            let mut output = Vec::new();
            event_loop
                .initialize_inline_origin(x, y, &mut output)
                .unwrap();
            assert_eq!(event_loop.render.pos, expected.into());
            assert_eq!(output, bytes);
        }
    }

    #[tokio::test]
    async fn fullscreen_resize_subtracts_origin_with_saturation() {
        for (width, height, expected) in [
            (30, 10, (28, 7)),
            (1, 10, (0, 7)),
            (30, 2, (28, 0)),
            (1, 2, (0, 0)),
            (2, 3, (0, 0)),
        ] {
            let mut event_loop = Builder::new().size((20, 5)).build_event_loop(Noop);
            event_loop.render.pos = (2, 3).into();
            let mut reader = futures::stream::iter([Ok(Event::Resize(width, height))]);
            event_loop.run_loop_with_stream(&mut reader).await.unwrap();
            assert_eq!(event_loop.terminal_size, (width, height).into());
            assert_eq!(event_loop.render.size(), expected.into());
            assert_eq!(event_loop.render.pos, (2, 3).into());
        }
    }

    #[test]
    fn inline_mouse_coordinates_are_translated_to_local_space() {
        let event_loop = inline_loop(Noop);
        let event = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 7,
            row: 5,
            modifiers: KeyModifiers::empty(),
        });

        let normalized = event_loop.normalize_inline_mouse_event(event);

        assert_eq!(
            normalized,
            Some(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 5,
                row: 2,
                modifiers: KeyModifiers::empty(),
            }))
        );
    }

    #[test]
    fn inline_mouse_events_outside_rendered_area_are_dropped() {
        let event_loop = inline_loop(Noop);
        let event = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 7,
            row: 8,
            modifiers: KeyModifiers::empty(),
        });

        assert_eq!(event_loop.normalize_inline_mouse_event(event), None);
    }

    #[derive(Default)]
    struct PointerProbe {
        events: Vec<Event>,
        batches: usize,
        pointer: Option<(u16, u16)>,
        leaves: usize,
    }

    impl Draw for PointerProbe {
        fn draw(&mut self, _chunk: Chunk) -> Result<Size, DrawErr> {
            Ok((0, 0).into())
        }
    }

    impl Update for PointerProbe {
        fn on_events(&mut self, events: &[Event]) -> Result<(), DrawErr> {
            self.batches += 1;
            self.events.extend_from_slice(events);
            for event in events {
                if let Event::Mouse(mouse) = event {
                    self.pointer = Some((mouse.column, mouse.row));
                }
            }
            Ok(())
        }

        fn on_pointer_leave(&mut self) {
            self.pointer = None;
            self.leaves += 1;
        }

        fn update(&mut self) -> Result<bool, DrawErr> {
            Ok(false)
        }
    }

    fn moved(column: u16, row: u16) -> Event {
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            column,
            row,
            modifiers: KeyModifiers::empty(),
        })
    }

    #[test]
    fn inline_pointer_leave_clears_hover_at_each_edge_without_forwarding_outside_events() {
        for (x, y) in [(1, 5), (7, 2), (22, 5), (7, 7)] {
            let mut event_loop = inline_loop(PointerProbe::default());
            let key = Event::Key(crossterm::event::KeyCode::Char('a').into());
            assert!(event_loop.dispatch_events(vec![moved(7, 5), moved(x, y), key.clone()]));
            let probe = event_loop.render.thing();
            assert_eq!(probe.events, [moved(5, 2), key]);
            assert_eq!(probe.batches, 1);
            assert_eq!(probe.pointer, None);
            assert_eq!(probe.leaves, 1);

            // An outside-only batch must also notify the application, which
            // decides whether clearing hover requires repainting.
            assert!(!event_loop.dispatch_events(vec![moved(x, y)]));
            assert_eq!(event_loop.render.thing().leaves, 2);
        }
    }

    #[test]
    fn inline_pointer_reentry_in_the_same_batch_keeps_the_final_inside_position() {
        let mut event_loop = inline_loop(PointerProbe::default());
        event_loop.dispatch_events(vec![moved(7, 5), moved(1, 5), moved(8, 6)]);
        let probe = event_loop.render.thing();
        assert_eq!(probe.events, [moved(5, 2), moved(6, 3)]);
        assert_eq!(probe.batches, 1);
        assert_eq!(probe.pointer, Some((6, 3)));
        assert_eq!(probe.leaves, 0);
    }

    #[test]
    fn fullscreen_mouse_dispatch_preserves_events_without_inline_leave_notifications() {
        let mut event_loop = Builder::new().build_event_loop(PointerProbe::default());
        event_loop.dispatch_events(vec![moved(90, 30)]);
        let probe = event_loop.render.thing();
        assert_eq!(probe.events, [moved(90, 30)]);
        assert_eq!(probe.pointer, Some((90, 30)));
        assert_eq!(probe.leaves, 0);
    }
}
