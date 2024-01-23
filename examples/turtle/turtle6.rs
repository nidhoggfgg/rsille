use rsille::{Canvas, Turtle};

fn main() {
    let mut canvas = Canvas::new();
    let mut t = Turtle::new();
    for _ in 0..6 {
        for _ in 0..3 {
            t.forward(10.0);
            t.left(120.0);
        }
        t.forward(10.0);
        t.right(60.0);
    }

    canvas.draw(&t, 15.0, 15.0);
    println!("{}", canvas.frame());
}
