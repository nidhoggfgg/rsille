use rsille::{color::Color, extra::Turtle, Canvas};

fn main() {
    let mut canvas = Canvas::new();
    let mut t = Turtle::new();
    for i in 0..36 {
        t.right(10.0);
        for j in 0..36 {
            t.color(Color::Rgb {
                r: 60 + i * 5,
                g: 190 - j * 5,
                b: 220,
            });
            t.right(10.0);
            t.forward(8.0);
        }
    }
    canvas.paint(&t, 100.0, 100.0).unwrap();
    canvas.print();
}
