use rsille::{color::TermColor, Canvas, Turtle};

fn star5() -> (Turtle, (f64, f64)) {
    let mut t = Turtle::new();
    for _ in 0..5 {
        t.color(TermColor::C16(rsille::color::Color16::Red));
        t.forward(100.0);
        t.right(144.0);
    }
    (t, (0.0, 30.0))
}

fn spiral() -> (Turtle, (f64, f64)) {
    let mut t = Turtle::new();
    let mut length = 1.0;
    for i in 0..150 {
        t.color(TermColor::Crgb(100 + i, 255 - i, 60 + i));
        t.forward(length);
        t.right(10.0);
        length += 0.05;
    }
    (t, (50.0, 130.0))
}

fn circle() -> (Turtle, (f64, f64)) {
    let mut t = Turtle::new();
    t.color(TermColor::C256(123));
    t.circle(30.0, 360.0, 100);
    (t, (130.0, 80.0))
}

fn star6() -> (Turtle, (f64, f64)) {
    let mut t = Turtle::new();
    for i in 0..6 {
        for j in 0..3 {
            t.color(TermColor::C256(i * 20 + j * 5));
            t.forward(10.0);
            t.left(120.0);
        }
        t.forward(10.0);
        t.right(60.0);
    }
    (t, (120.0, 130.0))
}

fn main() {
    let mut canvas = Canvas::new();
    let things = vec![star5(), spiral(), circle(), star6()];
    for (t, (x, y)) in things {
        canvas.paint(&t, x, y).unwrap();
    }
    println!("{}", canvas.frame());
}
