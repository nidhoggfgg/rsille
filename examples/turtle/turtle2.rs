use rsille::{Canvas, Turtle};

fn main() {
    let mut canvas = Canvas::new();
    let mut t = Turtle::new();
    for _ in 0..5 {
        t.forward(100.0);
        t.right(144.0);
    }

    canvas.draw(&t, 0.0, 30.0);
    println!("{}", canvas.frame());
}
