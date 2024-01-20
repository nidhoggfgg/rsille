use rsille::Turtle;

fn main() {
    let mut t = Turtle::new(50.0, 50.0);
    let mut length = 1.0;
    for _ in 0..150 {
        t.forward(length);
        t.right(10.0);
        length += 0.05;
    }

    println!("{}", t.draw());
}