use rsille::Turtle;

fn main() {
    let mut t = Turtle::new(0.0, 30.0);
    for _ in 0..5 {
        t.forward(100.0);
        t.right(144.0);
    }

    println!("{}", t.frame());
}