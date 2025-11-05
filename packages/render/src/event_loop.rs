use std::{
    io::{Stdout, stdout},
    thread,
    time::Duration,
};

use crossterm::{
    cursor::{Hide, Show},
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyEvent},
    execute,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::{FutureExt, StreamExt};
use log::{error, warn};
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
    exit_code: KeyEvent,
    max_event_per_frame: usize,
    frame_limit: Option<u16>,
    alt_screen: bool,
    mouse_capture: bool,
    hide_cursor: bool,
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
        self.exit_code = exit_code;
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

    pub fn run(self) -> Result<(), DrawErr> {
        // Create terminal guard - this will automatically cleanup on drop
        let _guard = TerminalGuard::new(
            self.alt_screen,
            self.raw_mode,
            self.mouse_capture,
            self.hide_cursor,
        )?;

        let (render_tx, render_rx) = mpsc::channel(1);
        let (event_tx, event_rx) = mpsc::channel(1);
        let (stop_tx, stop_rx) = std::sync::mpsc::channel();

        let event_thread = self.make_event_thread(render_rx, event_tx, stop_tx);
        let render_thread = self.make_render_thread(render_tx, event_rx, stop_rx);

        // Join threads and handle panics gracefully
        event_thread.join().map_err(DrawErr::thread_panic)?;
        render_thread.join().map_err(DrawErr::thread_panic)?;

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

                let now = std::time::Instant::now();

                if let Err(e) = self.render.on_events(&events) {
                    error!("Error processing events: {}", e);
                }
                if let Err(e) = self.render.update() {
                    error!("Error updating render state: {}", e);
                }

                if let Err(e) = self.render.render() {
                    error!("Error rendering: {}", e);
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
    exit_code: KeyEvent,
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

        loop {
            let event = reader.next().fuse();
            select! {
                Some(_) = render_rx.recv() => {
                    // Send collected events to render thread
                    if event_tx.send(events).await.is_err() {
                        // Render thread has stopped, exit gracefully
                        break;
                    }
                    events = Vec::new();
                }
                maybe_event = event => {
                    match maybe_event {
                        Some(Ok(event)) => {
                            if let Event::Key(key_event) = event {
                                if key_event == exit_code {
                                    // User requested exit
                                    let _ = stop_tx.send(());
                                    break;
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
