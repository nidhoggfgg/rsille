use std::{
    io::{self, Stdout},
    thread,
    time::Duration,
};

use futures::{FutureExt, StreamExt};
use term::{
    crossterm::event::EventStream,
    event::{Event, KeyEvent},
};
use tokio::{select, sync::mpsc};

use crate::{DrawUpdate, Render, Update};

use super::Builder;

pub struct EventLoop {
    render: Render<Stdout>,
    raw_mode: bool,
    exit_code: KeyEvent,
    max_event_per_frame: usize,
    frame_limit: Option<u16>,
    alt_screen: bool,
    mouse_capture: bool,
    hide_cursor: bool,
}

impl EventLoop {
    pub(super) fn from_builder<T>(builder: &Builder, thing: T) -> io::Result<Self>
    where
        T: DrawUpdate + Send + Sync + 'static,
    {
        Ok(Self {
            render: Render::new(thing),
            raw_mode: builder.enable_raw_mode,
            exit_code: builder.exit_code,
            max_event_per_frame: builder.max_event_per_frame,
            frame_limit: builder.frame_limit,
            alt_screen: builder.enable_alt_screen,
            mouse_capture: builder.enable_mouse_capture,
            hide_cursor: builder.enable_hide_cursor,
        })
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

    pub fn run(self) {
        let alt_screen = self.alt_screen;
        let raw_mode = self.raw_mode;
        let mouse_capture = self.mouse_capture;
        let hide_cursor = self.hide_cursor;

        // first enter alt screen then enable raw mode
        if alt_screen {
            term::enter_alt_screen().unwrap();
        }
        if raw_mode {
            term::enable_raw_mode().unwrap();
        }
        if mouse_capture {
            term::enable_mouse_capture().unwrap();
        }
        if hide_cursor {
            term::hide_cursor().unwrap();
        }

        let (render_tx, render_rx) = mpsc::channel(1);
        let (event_tx, event_rx) = mpsc::channel(1);
        let (stop_tx, stop_rx) = std::sync::mpsc::channel();

        let event_thread = self.make_event_thread(render_rx, event_tx, stop_tx);
        let render_thread = self.make_render_thread(render_tx, event_rx, stop_rx);
        event_thread.join().unwrap();
        render_thread.join().unwrap();

        // some cleanup, reverse the order of enable and disable
        if hide_cursor {
            term::show_cursor().unwrap();
        }
        if mouse_capture {
            term::disable_mouse_capture().unwrap();
        }
        if raw_mode {
            term::disable_raw_mode().unwrap();
        }
        if alt_screen {
            term::leave_alt_screen().unwrap();
        }
    }

    fn make_event_thread(
        &self,
        mut render_rx: mpsc::Receiver<()>,
        event_tx: mpsc::Sender<Vec<Event>>,
        stop_tx: std::sync::mpsc::Sender<()>,
    ) -> thread::JoinHandle<()> {
        let exit_code = self.exit_code;
        let max_event_per_frame = self.max_event_per_frame;
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let mut reader = EventStream::new();
                let mut events = Vec::new();
                event_tx.send(Vec::new()).await.unwrap();
                loop {
                    let event = reader.next().fuse();
                    select! {
                        Some(_) = render_rx.recv() => {
                            event_tx.send(events).await.unwrap();
                            events = Vec::new();
                        }
                        maybe_event = event => {
                            match maybe_event {
                                Some(Ok(event)) => {
                                    let event = Event::from(event);
                                    if event == Event::Key(exit_code) {
                                        stop_tx.send(()).unwrap();
                                        break;
                                    }
                                    if events.len() < max_event_per_frame {
                                        events.push(event);
                                    }
                                }
                                Some(Err(_e)) => {
                                    #[cfg(feature = "log")]
                                    log::error!("read event error: {:#?}", _e);
                                }
                                None => {}
                            }
                        }
                    }
                }
            });
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
                let events = event_rx.blocking_recv().unwrap();

                let now = std::time::Instant::now();

                // update
                self.render.on_events(&events).unwrap_or(());
                self.render.update().unwrap_or(false);

                // draw
                self.render.render().unwrap_or(());

                // frame limit
                let used_time = now.elapsed();
                if used_time < time_per_frame {
                    thread::sleep(time_per_frame - used_time);
                }

                if render_tx.blocking_send(()).is_err() {}
            }
        })
    }
}
