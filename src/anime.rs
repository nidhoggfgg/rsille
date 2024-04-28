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

use crate::{
    term::{self, get_terminal_size, is_raw_mode},
    Canvas, Paint,
};

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
/// let mut anime = Animation::new();
/// anime.push(cube, |cube| {
///     cube.rotate((1.0, 2.0, 3.0));
///     false
/// }, (30, -30));
/// anime.run();
/// ```
pub struct Animation {
    canvas: Canvas,
    objs: Vec<Box<dyn Update + Send>>,
    fps: u32,
    hide_cursor: bool,
    size: Option<(i32, i32)>,
    end: Arc<Mutex<bool>>,
}

impl Animation {
    /// Create a new animation
    ///
    /// The default fps is 30 and hide the cursor.
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(),
            objs: Vec::new(),
            fps: 30,
            hide_cursor: true,
            size: None,
            end: Arc::new(Mutex::new(false)),
        }
    }

    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }

    pub fn with_hide_cursor(mut self, hide_cursor: bool) -> Self {
        self.hide_cursor = hide_cursor;
        self
    }

    pub fn with_minx<T>(mut self, minx: T) -> Self
    where
        T: Into<f64>,
    {
        self.canvas.set_minx(minx);
        self
    }

    pub fn with_maxy<T>(mut self, maxy: T) -> Self
    where
        T: Into<f64>,
    {
        self.canvas.set_maxy(maxy);
        self
    }

    /// Set the fps of animation
    ///
    /// Default is 30
    pub fn set_fps(&mut self, fps: u32) {
        self.fps = fps;
    }

    /// Hide the cursor or not
    pub fn hide_cursor(&mut self, hide_cursor: bool) {
        self.hide_cursor = hide_cursor;
    }

    // Set the size of the canvas
    //
    // Give a look at [Canvas::set_size](crate::Canvas::set_size)
    // pub fn set_size<T>(&mut self, width: T, height: T)
    // where
    //     T: Into<f64>,
    // {
    //     self.canvas.lock().unwrap().set_size(width, height);
    // }

    /// Set the min `x` of the canvas
    pub fn set_minx<T>(&mut self, minx: T)
    where
        T: Into<f64>,
    {
        self.canvas.set_minx(minx);
    }

    /// Set the max `y` of the canvas
    pub fn set_maxy<T>(&mut self, maxy: T)
    where
        T: Into<f64> + Copy,
    {
        self.canvas.set_maxy(maxy);
    }

    /// Push an object to the animation
    ///
    /// * `obj` - the object to paint
    /// * `f` - the function to update the object
    /// * `xy` - the position to paint the object
    pub fn push<T, F, N>(&mut self, obj: T, f: F, xy: (N, N))
    where
        T: Paint + Send + 'static,
        F: FnMut(&mut T) -> bool + Send + 'static,
        N: Into<f64> + Copy,
    {
        self.objs.push(Box::new(UserObj {
            obj,
            f,
            xy: (xy.0.into(), xy.1.into()),
            is_end: false,
        }));
    }

    /// Run the animation
    ///
    /// When all the objects are end or press `ctrl+c` or `esc`, the animation will stop.
    pub fn run(self) {
        // should be very carefully to change these code
        let mut _self = self;

        // init
        let duration = Duration::from_secs(1) / _self.fps;
        let mut stdout = std::io::stdout();
        term::clear();
        if _self.hide_cursor {
            term::hide_cursor();
        }
        enable_raw_mode().expect("can't enbale raw mode");

        // main loop
        let end = Arc::clone(&_self.end);
        let mainloop = thread::spawn(move || loop {
            let start_time = Instant::now();
            // must wraped! for drop the objs
            {
                let mut end = end.lock().unwrap();
                if *end {
                    break;
                }
                if _self.objs.iter().all(|obj| obj.is_end()) {
                    *end = true;
                    break;
                }
                _self.canvas.clear();
                queue!(stdout, MoveTo(0, 0)).unwrap();
                for obj in &mut _self.objs {
                    obj.update(); // shouldn't wrap with if obj.is_end() { ... }
                    obj.paint(&mut _self.canvas);
                }
                // canvas.print_on(&mut stdout, true).unwrap();
                _self.canvas.print();
                stdout.flush().unwrap();
            }
            let elapsed = start_time.elapsed();
            if elapsed < duration {
                thread::sleep(duration - elapsed - Duration::from_millis(3));
            }
        });

        // deal with the key
        let end = Arc::clone(&_self.end);
        let _keyloop = thread::spawn(move || loop {
            if *end.lock().unwrap() {
                break;
            }
            if crossterm::event::poll(Duration::from_millis(300)).unwrap() {
                let event = crossterm::event::read().expect("can't read key");
                let end_fn = || {
                    let mut end = end.lock().unwrap();
                    *end = true;
                };
                if let Event::Resize(_, _) = event {
                    term::clear();
                }
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

    // only used in run!
    #[allow(unused)]
    fn check_size(&self) {
        let size = if let Some(size) = self.size {
            size
        } else {
            return;
        };
        if !is_raw_mode() {
            return;
        }
        let (rows, cols) = get_terminal_size();
        if (rows as i32) < size.0 || (cols as i32) < size.1 {
            println!(
                "this anime need at least {}x{} terminal size, but only {}x{}",
                size.1, size.0, cols, rows
            );
        }
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
