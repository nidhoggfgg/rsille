use std::{
    io::{stdout, Stdout},
    time::{Duration, Instant},
};

use crossterm::{
    cursor::{position, Hide, MoveToNextLine, Show},
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, EventStream, KeyEvent, MouseEvent,
    },
    execute,
    terminal::{
        size as terminal_size, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use futures::StreamExt;
use log::{error, info, warn};
use tokio::time::Instant as TokioInstant;

use crate::render::{
    area::Size, Builder, DrawErr, DrawUpdate, InlineMouseMode, Render, WakeHandle,
};

struct TerminalGuard {
    hide_cursor: bool,
    mouse_capture: bool,
    raw_mode: bool,
    alt_screen: bool,
    bracketed_paste: bool,
}

impl TerminalGuard {
    /// Create and initialize terminal guard with the given settings
    fn new(
        alt_screen: bool,
        raw_mode: bool,
        mouse_capture: bool,
        hide_cursor: bool,
    ) -> Result<Self, DrawErr> {
        // Keep the guard live during setup so partial failures roll back.
        let mut guard = Self {
            hide_cursor: false,
            mouse_capture: false,
            raw_mode: false,
            alt_screen: false,
            bracketed_paste: false,
        };
        if alt_screen {
            execute!(stdout(), EnterAlternateScreen).map_err(DrawErr::TerminalSetup)?;
            guard.alt_screen = true;
            execute!(stdout(), Clear(ClearType::All)).map_err(DrawErr::TerminalSetup)?;
        }
        if raw_mode {
            crossterm::terminal::enable_raw_mode().map_err(DrawErr::TerminalSetup)?;
            guard.raw_mode = true;
            execute!(stdout(), EnableBracketedPaste).map_err(DrawErr::TerminalSetup)?;
            guard.bracketed_paste = true;
        }
        if mouse_capture {
            execute!(stdout(), EnableMouseCapture).map_err(DrawErr::TerminalSetup)?;
            guard.mouse_capture = true;
        }
        if hide_cursor {
            execute!(stdout(), Hide).map_err(DrawErr::TerminalSetup)?;
            guard.hide_cursor = true;
        }
        Ok(guard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.bracketed_paste {
            let _ = execute!(stdout(), DisableBracketedPaste);
        }
        if self.hide_cursor {
            let _ = execute!(stdout(), Show);
        }
        if self.mouse_capture {
            let _ = execute!(stdout(), DisableMouseCapture);
        }
        if self.raw_mode {
            let _ = crossterm::terminal::disable_raw_mode();
        }
        if self.alt_screen {
            let _ = execute!(stdout(), LeaveAlternateScreen);
        }
    }
}

pub struct EventLoop<T> {
    render: Render<Stdout, T>,
    raw_mode: bool,
    exit_code: Option<KeyEvent>,
    max_event_per_frame: usize,
    frame_limit: Option<u16>,
    alt_screen: bool,
    mouse_capture: bool,
    hide_cursor: bool,
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
        Self {
            render: Render::from_builder(builder, thing, stdout()),
            raw_mode: builder.enable_raw_mode,
            exit_code: builder.exit_code,
            max_event_per_frame: builder.max_event_per_frame,
            frame_limit: builder.frame_limit,
            alt_screen: builder.enable_alt_screen,
            mouse_capture: builder.enable_mouse_capture,
            hide_cursor: builder.enable_hide_cursor,
            inline_mode: builder.inline_mode,
            inline_max_height: builder.inline_max_height,
            inline_mouse_mode: builder.inline_mouse_mode,
            terminal_size: builder.size,
        }
    }

    pub fn max_event_per_frame(&mut self, max_event_per_frame: usize) -> &mut Self {
        self.max_event_per_frame = max_event_per_frame;
        self
    }

    pub fn frame_limit(&mut self, frame_limit: Option<u16>) -> &mut Self {
        self.frame_limit = frame_limit;
        self
    }

    pub fn exit_code(&mut self, exit_code: KeyEvent) -> &mut Self {
        self.exit_code = Some(exit_code);
        self
    }

    pub fn disable_exit_code(&mut self) -> &mut Self {
        self.exit_code = None;
        self
    }

    pub fn enable_alt_screen(&mut self) -> &mut Self {
        self.alt_screen = true;
        self
    }

    pub fn disable_alt_screen(&mut self) -> &mut Self {
        self.alt_screen = false;
        self
    }

    pub fn enable_mouse_capture(&mut self) -> &mut Self {
        self.mouse_capture = true;
        self
    }

    pub fn disable_mouse_capture(&mut self) -> &mut Self {
        self.mouse_capture = false;
        self
    }

    pub fn hide_cursor_when_render(&mut self) -> &mut Self {
        self.hide_cursor = true;
        self
    }

    pub fn show_cursor_when_render(&mut self) -> &mut Self {
        self.hide_cursor = false;
        self
    }

    /// Set initial used height for inline mode
    ///
    /// This should be called before run() to set the initial rendering height
    /// without reallocating the buffer. Only effective in inline mode.
    pub fn set_initial_used_height(&mut self, height: u16) -> &mut Self {
        self.render.set_used_height(height);
        self
    }

    /// Reject synchronous entry inside Tokio before changing terminal state.
    pub fn check_sync_runtime() -> Result<(), DrawErr> {
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(DrawErr::RuntimeCreation(std::io::Error::other(
                "synchronous terminal entry inside Tokio; use run_async instead",
            )));
        }
        Ok(())
    }

    pub fn run(self) -> Result<(), DrawErr> {
        self.run_returning().map(|_| ())
    }

    /// Run and return the application after terminal cleanup.
    pub fn run_returning(self) -> Result<T, DrawErr> {
        Self::check_sync_runtime()?;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(DrawErr::RuntimeCreation)?;
        rt.block_on(self.run_async_returning())
    }

    /// Run on an existing Tokio runtime with time and I/O drivers enabled.
    pub async fn run_async(self) -> Result<(), DrawErr> {
        self.run_async_returning().await.map(|_| ())
    }

    /// Run on the caller's runtime and return the application after cleanup.
    pub async fn run_async_returning(mut self) -> Result<T, DrawErr> {
        let _guard = TerminalGuard::new(
            self.alt_screen,
            self.raw_mode,
            self.mouse_capture,
            self.hide_cursor,
        )?;

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
                self.terminal_size = (width, height).into();
            }

            match position() {
                Ok((x, y)) => {
                    self.render.pos = (x, y).into();
                    if x != 0 {
                        execute!(stdout(), MoveToNextLine(1))?;
                        self.render.pos.down(1);
                    }
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
        let frame_interval = self
            .frame_limit
            .filter(|fps| *fps > 0)
            .map(|fps| Duration::from_secs_f64(1.0 / fps as f64));
        let mut pending_initial_frame = true;
        let mut next_frame_at = Instant::now();
        let mut reader = EventStream::new();
        let wake = self.render.thing().wake_handle();

        loop {
            let mut stop_after_iteration = false;
            let mut events = Vec::new();

            match wait_for_frame_or_event(
                &mut reader,
                self.exit_code,
                if wake.is_some() {
                    self.render.thing().next_update_at()
                } else {
                    frame_interval.map(|_| next_frame_at)
                },
                wake.as_ref(),
                pending_initial_frame,
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

            match drain_pending_events(
                &mut reader,
                self.exit_code,
                &mut events,
                self.max_event_per_frame,
            )
            .await
            {
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

            for event in &events {
                if let Event::Resize(width, height) = event {
                    self.terminal_size = (*width, *height).into();
                    if self.inline_mode {
                        self.render
                            .resize((*width, self.inline_buffer_height(*height)).into());
                    } else {
                        self.render.resize((*width, *height).into());
                    }
                }
            }

            let events = self.normalize_inline_mouse_events(events);

            if let Err(e) = self.render.on_events(&events) {
                error!("Error processing events: {}", e);
            }
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

            if !events.is_empty()
                || needs_render
                || self.render.has_pending_changes()
                || force_inline_frame
            {
                if let Err(e) = self.render.render() {
                    error!("Error rendering: {}", e);
                } else {
                    if force_inline_frame {
                        if let Err(e) = self.render.force_inline_viewport_follow() {
                            error!("Error following inline viewport: {}", e);
                        }
                    }
                    self.align_inline_origin_after_render();
                }
            }

            if stop_after_iteration {
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

    fn inline_buffer_height(&self, viewport_height: u16) -> u16 {
        let available = if self.inline_mouse_mode.force_follows_viewport() {
            viewport_height.saturating_sub(1).max(1)
        } else {
            viewport_height
        };

        self.inline_max_height.min(available)
    }

    fn align_inline_origin_after_render(&mut self) {
        if !self.inline_mode {
            return;
        }

        let used_height = self.render.inline_used_height();
        let bottom_padding = if self.inline_mouse_mode.force_follows_viewport() {
            1
        } else {
            0
        };
        let reserved_height = used_height.saturating_add(bottom_padding);
        let max_origin_y = self.terminal_size.height.saturating_sub(reserved_height);

        if self.render.inline_origin().y > max_origin_y {
            self.render.set_inline_origin_y(max_origin_y);
        }
    }

    fn normalize_inline_mouse_events(&self, events: Vec<Event>) -> Vec<Event> {
        if !self.inline_mode {
            return events;
        }

        events
            .into_iter()
            .filter_map(|event| self.normalize_inline_mouse_event(event))
            .collect()
    }

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
        (Event::Key(key_event), Some(exit_key)) if *key_event == exit_key
    )
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEventKind};
    use futures::FutureExt;

    use super::*;
    use crate::render::{area::Size, chunk::Chunk, Draw, Update};

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

    fn inline_loop() -> EventLoop<Noop> {
        let mut builder = Builder::new();
        builder.inline_mode(true).size((20, 5));
        let mut event_loop = builder.build_event_loop(Noop);
        event_loop.render.pos = (2, 3).into();
        event_loop.render.set_used_height(4);
        event_loop
    }

    #[test]
    fn inline_mouse_coordinates_are_translated_to_local_space() {
        let event_loop = inline_loop();
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
        let event_loop = inline_loop();
        let event = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 7,
            row: 8,
            modifiers: KeyModifiers::empty(),
        });

        assert_eq!(event_loop.normalize_inline_mouse_event(event), None);
    }
}
