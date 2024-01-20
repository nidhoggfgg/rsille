use rsille::Turtle;

fn main() {
    let mut t = Turtle::new(30.0, 60.0);
    t.circle(30.0, 360.0, 100);
    println!("{}", t.draw());
}