use rsille::{Canvas, Turtle};

fn main() {
    let mut canvas = Canvas::new();
    let mut t = Turtle::new();
    t.circle(30.0, 360.0, 100);
    canvas.draw(&t, 30.0, 60.0);
    println!("{}", canvas.frame());
}
