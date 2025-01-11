use std::{
    collections::VecDeque,
    io::{self, Write},
    thread,
    time::Duration,
};

use futures::{FutureExt, StreamExt};
use term::{
    crossterm::{
        cursor::{MoveTo, MoveToNextLine},
        event::{Event, EventStream, KeyCode, KeyEvent},
        queue,
        style::Print,
        terminal::{disable_raw_mode, enable_raw_mode},
    },
    enable_mouse_capture,
};
use tokio::{select, sync::mpsc};

use crate::{panel::Panel, style::Stylized, traits::Draw, DrawErr, Update};

pub struct Runtime {
    panel: Panel,
    raw_mode: bool,
    exit_code: KeyEvent,
}

impl Runtime {
    pub fn new(panel: Panel) -> Runtime {
        Runtime {
            panel,
            raw_mode: false,
            exit_code: KeyCode::Esc.into(),
        }
    }

    pub fn draw(&mut self) -> Result<(Duration, Vec<Stylized>), DrawErr> {
        let now = std::time::Instant::now();
        let data = self.panel.draw()?;
        Ok((now.elapsed(), data))
    }

    pub fn print(&mut self, data: Vec<Stylized>) -> io::Result<Duration> {
        // queue!(self.buffer, Clear(ClearType::All))?;
        let now = std::time::Instant::now();
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
        Ok(now.elapsed())
    }

    pub fn run(mut self) {
        enable_raw_mode().unwrap();
        enable_mouse_capture(&mut io::stdout()).unwrap();
        self.raw_mode = true;
        let (render_tx, mut render_rx) = mpsc::channel(1);
        let (event_tx, mut event_rx) = mpsc::channel(1);
        let (stop_tx, stop_rx) = std::sync::mpsc::channel();

        let exit_code = self.exit_code;

        let event_thread = thread::spawn(move || {
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
                                    if events.len() < 10 {
                                        events.push(event);
                                    }
                                }
                                Some(Err(_)) => {}
                                None => {}
                            }
                        }
                    }
                }
            });
        });

        let render_thread = thread::spawn(move || {
            let mut _self = self;
            let mut queue = VecDeque::with_capacity(10);
            for _ in 0..10 {
                queue.push_back(0);
            }
            loop {
                if let Ok(_) = stop_rx.try_recv() {
                    disable_raw_mode().unwrap();
                    break;
                }

                let events = event_rx.blocking_recv().unwrap();
                let now = std::time::Instant::now();
                _self.panel.update(&events).unwrap_or(false);
                let update_elapsed = now.elapsed();
                let (draw_elapsed, data) = _self.draw().unwrap();
                let print_elapsed = _self.print(data).unwrap();
                let elapsed = now.elapsed();
                queue.pop_front();
                queue.push_back(elapsed.as_micros());
                let mut sum = 0;
                for v in &queue {
                    sum += *v;
                }
                let fps = 1000.0 / (sum as f64 / 1000.0);
                println!("update_elapsed: {:?}", update_elapsed.as_micros());
                println!("draw_elapsed: {:?}", draw_elapsed.as_micros());
                println!("print_elapsed: {:?}", print_elapsed.as_micros());
                println!("elapsed: {:?}", elapsed.as_micros());
                println!("fps: {:?}", fps);

                if let Err(e) = render_tx.blocking_send(()) {
                    println!("render_tx error: {:#?}", e);
                }
            }
        });
        event_thread.join().unwrap();
        render_thread.join().unwrap();
    }
}
