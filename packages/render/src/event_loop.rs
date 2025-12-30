use std::{
    io::{stdout, Stdout},
    thread,
    time::Duration,
};

use crossterm::{
    cursor::{position, Hide, MoveToNextLine, Show},
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyEvent},
    execute,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::{FutureExt, StreamExt};
use log::{error, info, warn};
use tokio::{select, sync::mpsc};

use crate::{Builder, DrawErr, DrawUpdate, Render};

struct TerminalGuard {
    hide_cursor: bool,
    mouse_capture: bool,
    raw_mode: bool,
    alt_screen: bool,
}

impl TerminalGuard {
    /// Create and initialize terminal guard with the given settings
    fn new(
        alt_screen: bool,
        raw_mode: bool,
        mouse_capture: bool,
        hide_cursor: bool,
    ) -> Result<Self, DrawErr> {
        // Setup terminal in the correct order
        if alt_screen {
            execute!(stdout(), EnterAlternateScreen).map_err(DrawErr::TerminalSetup)?;
            execute!(stdout(), Clear(ClearType::All)).map_err(DrawErr::TerminalSetup)?;
        }
        if raw_mode {
            crossterm::terminal::enable_raw_mode().map_err(DrawErr::TerminalSetup)?;
        }
        if mouse_capture {
            execute!(stdout(), EnableMouseCapture).map_err(DrawErr::TerminalSetup)?;
        }
        if hide_cursor {
            execute!(stdout(), Hide).map_err(DrawErr::TerminalSetup)?;
        }

        Ok(Self {
            hide_cursor,
            mouse_capture,
            raw_mode,
            alt_screen,
        })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Cleanup in reverse order, ignoring errors since we're in Drop
        // We use let _ to explicitly ignore errors since there's nothing we can do in Drop
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
}

impl<T> EventLoop<T>
where
    T: DrawUpdate + Send + Sync + 'static,
{
    pub(super) fn from_builder(builder: &Builder, thing: T) -> Self
    where
        T: DrawUpdate + Send + Sync + 'static,
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

    pub fn run(mut self) -> Result<(), DrawErr> {
        // Create terminal guard first - this will automatically cleanup on drop
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

        // In inline mode, capture the current cursor position AFTER enabling raw mode
        // Raw mode is needed for position() to work properly
        if self.inline_mode {
            // Get current cursor position
            match position() {
                Ok((x, y)) => {
                    self.render.pos = (x, y).into();
                    // not a newline, move to the next line
                    if x != 0 {
                        execute!(stdout(), MoveToNextLine(1))?;
                        self.render.pos.down(1);
                    }
                }
                Err(e) => {
                    // If we can't get cursor position, use (0, 0) as fallback
                    // This might happen in non-interactive environments
                    warn!(
                        "Failed to get cursor position in inline mode: {}, using (0, 0)",
                        e
                    );
                    self.render.pos = (0, 0).into();
                }
            }
        }

        let (render_tx, render_rx) = mpsc::channel(1);
        let (event_tx, event_rx) = mpsc::channel(1);
        let (stop_tx, stop_rx) = std::sync::mpsc::channel();

        let event_thread = self.make_event_thread(render_rx, event_tx, stop_tx);
        let render_thread = self.make_render_thread(render_tx, event_rx, stop_rx);

        // Join threads and handle panics gracefully
        event_thread.join().map_err(DrawErr::thread_panic)?;
        render_thread.join().map_err(DrawErr::thread_panic)?;

        info!(
            target: "render::event_loop",
            "event loop stopped"
        );

        // Terminal cleanup happens automatically via TerminalGuard's Drop
        Ok(())
    }

    fn make_event_thread(
        &self,
        render_rx: mpsc::Receiver<()>,
        event_tx: mpsc::Sender<Vec<Event>>,
        stop_tx: std::sync::mpsc::Sender<()>,
    ) -> thread::JoinHandle<()> {
        let exit_code = self.exit_code;
        let max_event_per_frame = self.max_event_per_frame;
        thread::spawn(move || {
            event_thread(render_rx, event_tx, stop_tx, exit_code, max_event_per_frame)
        })
    }

    fn make_render_thread(
        mut self,
        render_tx: mpsc::Sender<()>,
        mut event_rx: mpsc::Receiver<Vec<Event>>,
        stop_rx: std::sync::mpsc::Receiver<()>,
    ) -> thread::JoinHandle<()> {
        let frame_limit = self.frame_limit.unwrap_or(0);
        let time_per_frame = if frame_limit > 0 {
            Duration::from_secs_f64(1.0 / frame_limit as f64)
        } else {
            Duration::from_secs(0)
        };

        thread::spawn(move || {
            loop {
                let now = std::time::Instant::now();

                // check stop signal
                if stop_rx.try_recv().is_ok() {
                    break;
                }

                // collect events
                let events = match event_rx.blocking_recv() {
                    Some(events) => events,
                    None => {
                        // Channel closed, exit gracefully
                        warn!("Event channel closed, stopping render thread");
                        break;
                    }
                };

                // Check for resize events and update render buffer size
                for event in &events {
                    if let Event::Resize(width, height) = event {
                        self.render.resize((*width, *height).into());
                    }
                }

                if let Err(e) = self.render.on_events(&events) {
                    error!("Error processing events: {}", e);
                }
                if let Err(e) = self.render.update() {
                    error!("Error updating render state: {}", e);
                }

                // Query required size BEFORE rendering
                // This allows dynamic size adjustment based on current state
                let current_size = self.render.size();
                if let Some(new_size) = self.render.thing().required_size(current_size) {
                    // In inline mode, use set_used_height to avoid buffer reallocation
                    // Buffer capacity remains constant at inline_max_height
                    if self.inline_mode {
                        self.render.set_used_height(new_size.height);
                    } else {
                        // In fullscreen mode, do full resize
                        self.render.resize(new_size);
                    }
                }

                // Only render if there are pending changes
                if !events.is_empty() || self.render.has_pending_changes() {
                    if let Err(e) = self.render.render() {
                        error!("Error rendering: {}", e);
                    }
                }

                // frame limit
                let used_time = now.elapsed();
                if used_time < time_per_frame {
                    thread::sleep(time_per_frame - used_time);
                }

                // Notify event thread we're ready for more events
                // If send fails, event thread has stopped, so we should stop too
                if render_tx.blocking_send(()).is_err() {
                    break;
                }
            }
        })
    }
}

fn event_thread(
    mut render_rx: mpsc::Receiver<()>,
    event_tx: mpsc::Sender<Vec<Event>>,
    stop_tx: std::sync::mpsc::Sender<()>,
    exit_code: Option<KeyEvent>,
    max_event_per_frame: usize,
) {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            error!("Failed to create tokio runtime: {}", e);
            let _ = stop_tx.send(());
            return;
        }
    };

    rt.block_on(async move {
        let mut reader = EventStream::new();
        let mut events = Vec::new();

        // Send initial empty event list
        if event_tx.send(Vec::new()).await.is_err() {
            warn!("Failed to send initial events, render thread may have stopped");
            return;
        }

        // Create a timer for continuous rendering (60 FPS = ~16.67ms per frame)
        let mut interval = tokio::time::interval(Duration::from_millis(16));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            let event = reader.next().fuse();
            select! {
                _ = interval.tick() => {
                    // Timer tick - wait for render thread to be ready, then send events
                    if render_rx.recv().await.is_none() {
                        break;
                    }
                    if event_tx.send(events.clone()).await.is_err() {
                        break;
                    }
                    events.clear();
                }
                maybe_event = event => {
                    match maybe_event {
                        Some(Ok(event)) => {
                            // Only check exit code if it's enabled
                            if let Some(exit_key) = exit_code {
                                if let Event::Key(key_event) = event {
                                    if key_event == exit_key {
                                        // User requested exit
                                        let _ = stop_tx.send(());
                                        break;
                                    }
                                }
                            }
                            if events.len() < max_event_per_frame {
                                events.push(event);
                            }
                        }
                        Some(Err(e)) => {
                            error!("Error reading event: {}", e);
                            // Continue processing despite errors
                        }
                        None => {
                            // Event stream ended
                            warn!("Event stream ended");
                            let _ = stop_tx.send(());
                            break;
                        }
                    }
                }
            }
        }
    });
}
