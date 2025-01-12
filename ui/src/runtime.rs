use std::{
    collections::VecDeque,
    io::{self, Write},
    thread,
};

use futures::{FutureExt, StreamExt};
use term::crossterm::{
    cursor::{MoveTo, MoveToNextLine},
    event::{Event, EventStream, KeyCode, KeyEvent},
    queue,
    style::Print,
};
use tokio::{select, sync::mpsc};

use crate::{panel::Panel, style::Stylized, traits::Draw, Update};

pub struct Runtime {
    panel: Panel,
    raw_mode: bool,
    exit_code: KeyEvent,
    max_event_per_frame: usize,
    alt_screen: bool,
    mouse_capture: bool,
    hide_cursor: bool,
}

impl Runtime {
    pub fn new(panel: Panel) -> Runtime {
        Runtime {
            panel,
            raw_mode: false,
            exit_code: KeyCode::Esc.into(),
            max_event_per_frame: 10,
            alt_screen: true,
            mouse_capture: true,
            hide_cursor: true,
        }
    }

    pub fn set_max_event_per_frame(&mut self, max_event_per_frame: usize) {
        self.max_event_per_frame = max_event_per_frame;
    }

    pub fn print(&mut self, data: Vec<Stylized>) -> io::Result<()> {
        queue!(io::stdout(), MoveTo(0, 0))?;
        let (width, _) = self
            .panel
            .size()
            .ok_or(io::Error::new(io::ErrorKind::Other, "draw error"))?;

        for chunk in data.chunks(width as usize) {
            for v in chunk {
                v.queue(&mut io::stdout())?;
            }
            if self.raw_mode {
                queue!(io::stdout(), MoveToNextLine(1))?;
            } else {
                queue!(io::stdout(), Print("\n"))?;
            }
        }

        io::stdout().flush()?;
        Ok(())
    }

    pub fn run(mut self) {
        self.raw_mode = true;
        let alt_screen = self.alt_screen;
        let raw_mode = self.raw_mode;
        let mouse_capture = self.mouse_capture;
        let hide_cursor = self.hide_cursor;

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

        // some cleanup
        if hide_cursor {
            term::show_cursor().unwrap();
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
                                    if event == Event::Key(exit_code) {
                                        stop_tx.send(()).unwrap();
                                        break;
                                    }
                                    if events.len() < max_event_per_frame {
                                        events.push(event);
                                    }
                                }
                                #[cfg(feature = "log")]
                                Some(Err(e)) => {
                                    log::error!("read event error: {:#?}", e);
                                }
                                #[cfg(not(feature = "log"))]
                                Some(Err(_)) => {}
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
        thread::spawn(move || {
            #[cfg(feature = "log")]
            let mut queue = VecDeque::with_capacity(10);
            #[cfg(feature = "log")]
            for _ in 0..10 {
                queue.push_back(0);
            }
            loop {
                #[cfg(feature = "log")]
                log::info!("|----------start render a frame----------|");

                // check stop signal
                if let Ok(_) = stop_rx.try_recv() {
                    #[cfg(feature = "log")]
                    log::info!("break in render thread");
                    break;
                }

                // collect events
                let events = event_rx.blocking_recv().unwrap();

                #[cfg(feature = "log")]
                let now = std::time::Instant::now();

                // update
                self.panel.update(&events).unwrap_or(false);

                // refresh cache
                self.panel.refresh_cache().unwrap();

                // draw
                let data = self.panel.draw().unwrap();

                // print
                self.print(data).unwrap();

                // collect fps
                #[cfg(feature = "log")]
                {
                    let elapsed = now.elapsed();
                    log::info!("total use time: {:?}", elapsed);
                    queue.pop_front();
                    queue.push_back(elapsed.as_micros());
                    let mut sum = 0;
                    for v in &queue {
                        sum += *v;
                    }
                    let len = queue.len();
                    let fps = 1000.0 / (sum as f64 / 1000.0) * len as f64;
                    log::info!("fps: {:.2}", fps);
                }

                // send signal to event thread
                #[cfg(feature = "log")]
                if let Err(e) = render_tx.blocking_send(()) {
                    log::error!("render_tx error: {:#?}", e);
                }
                #[cfg(not(feature = "log"))]
                if let Err(_) = render_tx.blocking_send(()) {}

                // end of a frame
                #[cfg(feature = "log")]
                log::info!("|---------- end render a frame ----------|");
            }
        })
    }
}
