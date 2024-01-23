use rsille::{Canvas, Turtle};

fn main() {
    let mut canvas = Canvas::new();
    let mut t = Turtle::new();
    t.left(140.0);
    t.forward(22.0);
    for _ in 0..200 {
        t.right(1.0);
        t.forward(0.2);
    }
    t.left(120.0);
    for _ in 0..200 {
        t.right(1.0);
        t.forward(0.2);
    }
    t.forward(22.0);

    canvas.draw(&t, 30.0, 40.0);
    println!("{}", canvas.frame());
}
