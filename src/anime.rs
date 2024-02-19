use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use crate::{term, Canvas, Paint};

/// create an animation
///
/// make the animation easy
///
/// ## Example
///
/// ```
/// use rsille::{Turtle, Animation};
/// let mut anime = Animation::new();
/// let mut t = Turtle::new();
/// let mut length = 1.0;
/// for _ in 0..150 {
///     t.forward(length);
///     t.right(10.0);
///     length += 0.05;
/// }
/// t.anime();
/// anime.push(t, move |t: &mut Turtle| t.update(), (50.0, 50.0));
/// anime.run();
/// ```
pub struct Animation {
    canvas: Arc<Mutex<Canvas>>,
    objs: Arc<Mutex<Vec<Box<dyn Anime + Send>>>>,
    fps: u32,
    hide_cursor: bool,
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
    /// When all the objects are end, the animation will stop.
    pub fn run(&mut self) {
        let duration = Duration::from_secs(1) / self.fps;
        let objs = Arc::clone(&self.objs);
        let canvas = Arc::clone(&self.canvas);
        term::clear();
        if self.hide_cursor {
            term::hide_cursor();
        }
        let mainloop = thread::spawn(move || loop {
            let start_time = Instant::now();
            let mut objs = objs.lock().unwrap();
            let mut canvas = canvas.lock().unwrap();
            canvas.clear();
            term::move_to(0, 0);
            for obj in &mut *objs {
                obj.update();
                obj.paint(&mut canvas);
            }
            println!("{}", canvas.frame());
            if objs.iter().all(|obj| obj.is_end()) {
                break;
            }
            let elapsed = start_time.elapsed();
            if elapsed < duration {
                thread::sleep(duration - elapsed);
            }
        });
        mainloop.join().unwrap();
        term::show_cursor();
    }

    /// set the fps of animation
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

trait Anime {
    fn update(&mut self);
    fn is_end(&self) -> bool;
    fn paint(&self, canvas: &mut Canvas);
}

impl<T, F> Anime for UserObj<T, F>
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

    fn paint(&self, canvas: &mut Canvas) {
        let (x, y) = self.xy;
        canvas.paint(&self.obj, x, y).unwrap();
    }
}
