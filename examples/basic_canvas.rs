use std::io::stdout;

use rsille::{canvas::Canvas, render};

pub fn main() {
    let mut c = Canvas::new();

    for x in 0..1800 {
        let x = x as f64;
        c.set(x / 10.0, 15.0 + x.to_radians().sin() * 10.0);
    }

    let mut render = render::Builder::new()
        .pos((0, 0))
        .size((40, 20))
        .build_render(c, stdout());
    render.render().unwrap();
}