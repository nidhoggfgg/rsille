use rsille::{Canvas, Turtle};

#[cfg(feature = "color")]
use rsille::color::TermColor;

fn main() {
    let mut canvas = Canvas::new();
    let mut t = Turtle::new();
    for i in 0..36 {
        t.right(10.0);
        for j in 0..36 {
            #[cfg(feature = "color")]
            t.color(TermColor::Crgb(60+i*5, 190-j*5, 220));
            t.right(10.0);
            t.forward(8.0);
        }
    }
    canvas.paint(&t, 100.0, 100.0);
    println!("{}", canvas.frame());
}
