use rsille::Turtle;

fn main() {
    let mut t = Turtle::new(100.0, 100.0);
    for _ in 0..36 {
        t.right(10.0);
        for _ in 0..36 {
            t.right(10.0);
            t.forward(8.0);
        }
    }

    println!("{}", t.frame());
}
