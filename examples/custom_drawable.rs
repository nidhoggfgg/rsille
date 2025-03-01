use std::fs::File;
use std::io::Write;

use canvas::Canvas;
use render::style::Stylized;
use render::{Draw, DrawChunk, Update};

struct A {
    count: u128,
}

impl Draw for A {
    fn draw(&mut self) -> Result<render::DrawChunk, render::DrawErr> {
        let mut result = Vec::new();
        let mut height = 1;
        for i in 0..self.count {
            if i % 100 == 0 {
                height += 1;
            }
            if i % 10 == 0 {
                result.push(Stylized::new('一', None, None));
            } else {
                result.push(Stylized::new('1', None, None));
            }
        }
        let width = if height == 1 { result.len() } else { 100 };

        Ok(DrawChunk(result, width))
    }
}

impl Update for A {
    fn on_events(&mut self, _events: &[term::event::Event]) -> Result<(), render::DrawErr> {
        Ok(())
    }

    fn update(&mut self) -> Result<bool, render::DrawErr> {
        self.count += 1;
        Ok(true)
    }
}

#[tokio::main]
async fn main() {
    let target = Box::new(File::create("rsille.log").expect("Can't create file"));

    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} {} - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.level(),
                record.args()
            )
        })
        .filter(None, log::LevelFilter::Info)
        .target(env_logger::Target::Pipe(target))
        .init();

    let mut _canvas = Canvas::new();

    for x in 0..1800 {
        let x = x as f64;
        _canvas.set(x / 10.0, 15.0 + x.to_radians().sin() * 10.0);
    }

    let a = A { count: 0 };

    let render = render::Builder::new()
        .enable_all()
        .full_screen()
        .frame_limit(100)
        .size((20, 20))
        .build(a)
        .unwrap();

    render.run();
}
