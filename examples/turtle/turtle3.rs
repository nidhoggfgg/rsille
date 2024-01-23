use rsille::{Canvas, Turtle};

fn main() {
    let mut canvas = Canvas::new();
    let mut t = Turtle::new();
    let mut length = 1.0;
    for _ in 0..150 {
        t.forward(length);
        t.right(10.0);
        length += 0.05;
    }

    canvas.draw(&t, 50.0, 50.0);
    println!("{}", canvas.frame());
}
