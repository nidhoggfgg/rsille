use rsille::Turtle;

fn main() {
    let mut t = Turtle::new(15.0, 15.0);
    for _ in 0..6 {
        for _ in 0..3 {
            t.forward(10.0);
            t.left(120.0);
        }
        t.forward(10.0);
        t.right(60.0);
    }

    println!("{}", t.draw());
}