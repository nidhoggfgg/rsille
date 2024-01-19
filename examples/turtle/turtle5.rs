use rsille::Turtle;

fn main() {
    let mut t = Turtle::new(30.0, 40.0);
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

    println!("{}", t.frame());
}