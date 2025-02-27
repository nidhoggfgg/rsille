use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
};

use canvas::{Canvas, Paint};

#[derive(Clone)]
pub struct Animation {
    canvas: Canvas,
    objs: Arc<Mutex<Vec<Box<dyn Update + Send>>>>,
    fps: u32,
    hide_cusror: bool,
    size: Option<(u32, u32)>,
    end: Arc<Mutex<bool>>,
    thread_num: u32,
}

impl Animation {
    pub fn run(&mut self) {
        let (tx, rx): (Sender<Box<dyn Update>>, Receiver<Box<dyn Update>>) = mpsc::channel();
        let hander: Vec<_> = (0..self.thread_num)
            .map(|_| {
                let thread_tx = tx.clone();
                let objs = self.objs.clone();
                thread::spawn(move || {});
            })
            .collect();
    }
}

#[derive(Debug, Clone)]
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
