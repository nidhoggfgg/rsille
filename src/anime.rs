use std::{
    io::Write,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use crossterm::{
    cursor::MoveTo,
    event::{Event, KeyCode, KeyModifiers},
    queue,
    terminal::{disable_raw_mode, enable_raw_mode},
};

use crate::{term, Canvas, Paint};

/// Create an animation
///
/// Make the animation easy
///
/// ## Example
///
/// draw a cube and rotate it
/// ```no_run
/// use rsille::{extra::Object3D, Animation};
/// let cube = Object3D::cube(30.0);
/// let mut anime = rsille::Animation::new();
/// anime.push(cube, |cube| {
///     cube.rotate((1.0, 2.0, 3.0));
///     false
/// }, (30.0, 30.0));
/// anime.run();
/// ```
pub struct Animation {
    canvas: Arc<Mutex<Canvas>>,
    objs: Arc<Mutex<Vec<Box<dyn Update + Send>>>>,
    fps: u32,
    hide_cursor: bool,
    end: Arc<Mutex<bool>>,
}

impl Animation {
    /// create a new animation
    ///
    /// The default fps is 30 and hide the cursor.
    pub fn new() -> Self {
        Self {
            canvas: Arc::new(Mutex::new(Canvas::new())),
            objs: Arc::new(Mutex::new(Vec::new())),
            fps: 30,
            hide_cursor: true,
            end: Arc::new(Mutex::new(false)),
        }
    }

    /// push an object to the animation
    ///
    /// * `obj` - the object to paint
    /// * `f` - the function to update the object
    /// * `xy` - the position to paint the object
    pub fn push<T, F>(&mut self, obj: T, f: F, xy: (f64, f64))
    where
        T: Paint,
        F: FnMut(&mut T) -> bool + Send + 'static,
    {
        let mut objs = self.objs.lock().unwrap();
        objs.push(Box::new(UserObj {
            obj,
            f,
            xy,
            is_end: false,
        }));
    }

    /// run the animation
    ///
    /// When all the objects are end or press `ctrl+c` or `esc`, the animation will stop.
    pub fn run(&mut self) {
        // should be very carefully to change these code

        // init
        let duration = Duration::from_secs(1) / self.fps;
        let objs = Arc::clone(&self.objs);
        let canvas = Arc::clone(&self.canvas);
        let mut stdout = std::io::stdout();
        term::clear();
        if self.hide_cursor {
            term::hide_cursor();
        }
        enable_raw_mode().expect("can't enbale raw mode");

        // main loop
        let end = Arc::clone(&self.end);
        let mainloop = thread::spawn(move || loop {
            let start_time = Instant::now();
            // must wraped! for drop the objs
            {
                let mut objs = objs.lock().unwrap();
                let mut canvas = canvas.lock().unwrap();
                canvas.clear();
                queue!(stdout, MoveTo(0, 0)).unwrap();
                for obj in &mut *objs {
                    obj.update(); // shouldn't wrap with if obj.is_end() { ... }
                    obj.paint(&mut canvas);
                }
                // canvas.print_on(&mut stdout, true).unwrap();
                canvas.print().unwrap();
                stdout.flush().unwrap();
                let mut end = end.lock().unwrap();
                if *end {
                    break;
                }
                if objs.iter().all(|obj| obj.is_end()) {
                    *end = true;
                    break;
                }
            }
            let elapsed = start_time.elapsed();
            if elapsed < duration {
                thread::sleep(duration - elapsed);
            }
        });

        // deal with the key
        let objs = Arc::clone(&self.objs);
        let end = Arc::clone(&self.end);
        let _keyloop = thread::spawn(move || loop {
            if *end.lock().unwrap() {
                break;
            }
            if crossterm::event::poll(Duration::from_millis(300)).unwrap() {
                let event = crossterm::event::read().expect("can't read key");
                let end_fn = || {
                    let mut objs = objs.lock().unwrap();
                    let mut end = end.lock().unwrap();
                    for obj in &mut *objs {
                        obj.end();
                    }
                    *end = true;
                };
                if let Event::Key(key) = event {
                    if key.code == KeyCode::Esc {
                        end_fn();
                        break;
                    }
                    if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
                        end_fn();
                        break;
                    }
                }
            }
        });

        // keyloop.join().unwrap();
        mainloop.join().unwrap();
        term::show_cursor();
        disable_raw_mode().expect("can't disable raw mode");
    }

    /// set the fps of animation
    ///
    /// Default is 30
    pub fn set_fps(&mut self, fps: u32) {
        self.fps = fps;
    }

    /// hide the cursor or not
    pub fn set_cursor(&mut self, hide_cursor: bool) {
        self.hide_cursor = hide_cursor;
    }
}

struct UserObj<T, F> {
    obj: T,
    f: F,
    xy: (f64, f64),
    is_end: bool,
}

// a helper trait only for UserObj
trait Update {
    fn update(&mut self);
    fn is_end(&self) -> bool;
    fn end(&mut self);
    fn paint(&self, canvas: &mut Canvas);
}

impl<T, F> Update for UserObj<T, F>
where
    T: Paint,
    F: FnMut(&mut T) -> bool,
{
    fn update(&mut self) {
        if self.is_end {
            return;
        }
        self.is_end = (self.f)(&mut self.obj);
    }

    fn is_end(&self) -> bool {
        self.is_end
    }

    fn end(&mut self) {
        self.is_end = true;
    }

    fn paint(&self, canvas: &mut Canvas) {
        let (x, y) = self.xy;
        canvas.paint(&self.obj, x, y).unwrap();
    }
}
