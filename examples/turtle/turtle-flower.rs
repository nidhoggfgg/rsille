use rsille::{Canvas, Turtle};

fn main() {
    let mut canvas = Canvas::new();
    let mut t = Turtle::new();
    for _ in 0..36 {
        t.right(10.0);
        for _ in 0..36 {
            t.right(10.0);
            t.forward(8.0);
        }
    }
    canvas.paint(&t, 100.0, 100.0);
    println!("{}", canvas.frame());
}
