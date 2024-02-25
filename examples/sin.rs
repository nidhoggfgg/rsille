use rsille::Canvas;

fn main() {
    let mut c = Canvas::new();

    for x in 0..1800 {
        let x = x as f64;
        c.set(x / 10.0, 15.0 + x.to_radians().sin() * 10.0);
    }
    c.print();
    c.reset();

    for x in (0..1800).step_by(10) {
        let x = x as f64;
        c.set(x / 10.0, 10.0 + x.to_radians().sin() * 10.0);
        c.set(x / 10.0, 10.0 + x.to_radians().cos() * 10.0);
    }
    c.print();
    c.reset();

    for x in (0..3600).step_by(20) {
        let x = x as f64;
        c.set(x / 20.0, 4.0 + x.to_radians().sin() * 4.0);
    }
    c.print();
    c.reset();

    for x in (0..360).step_by(4) {
        let x = x as f64;
        c.set(x / 4.0, 30.0 + x.to_radians().sin() * 30.0);
    }

    for x in 0..30 {
        for y in 0..30 {
            let (x, y) = (x as f64, y as f64);
            c.set(x, y);
            c.toggle(x + 30.0, y + 30.0);
            c.toggle(x + 60.0, y);
        }
    }
    c.print();
}
