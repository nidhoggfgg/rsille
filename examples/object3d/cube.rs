use rsille::{object3d::Object3D, term, Canvas};

// generate the vertices(6) of cube and sides(12) of cube
// the sides contain the index of the vertice
fn gen_cube(side_len: f64) -> Object3D {
    #[rustfmt::skip]
    let a = [
        (-1, -1, -1),
        (-1, -1,  1),
        (-1,  1, -1),
        ( 1, -1, -1),
        (-1,  1,  1),
        ( 1, -1,  1),
        ( 1,  1, -1),
        ( 1,  1,  1),
    ];
    let mut points = Vec::new();
    let mut object = Object3D::new();
    for i in a {
        let x = side_len / 2.0 * i.0 as f64;
        let y = side_len / 2.0 * i.1 as f64;
        let z = side_len / 2.0 * i.2 as f64;
        points.push((x, y, z));
    }
    object.add_points(&points);
    object
        .add_sides(&[
            (0, 1),
            (1, 4),
            (4, 2),
            (2, 0),
            (3, 5),
            (5, 7),
            (7, 6),
            (6, 3),
            (1, 5),
            (4, 7),
            (2, 6),
            (0, 3),
        ])
        .unwrap();
    object
}

// just make the rotate looks more "random"
fn gen(k: i32) -> ((f64, f64, f64), f64) {
    let rotate = match k {
        k if k % 3 == 0 => (1.0, 2.0, 3.0),
        k if k % 3 == 1 => (2.0, 3.0, 4.0),
        k if k % 3 == 2 => (3.0, 4.0, 5.0),
        _ => panic!("impossible"),
    };
    let zoom = if k % 60 <= 30 {
        1.0 + (k % 60) as f64 * 0.02
    } else {
        1.6 - (k % 60 - 30) as f64 * 0.02
    };
    (rotate, zoom)
}

fn main() {
    let side_len = 30.0;
    let mut canvas = Canvas::new();
    let mut object = gen_cube(side_len);
    let mut k = 0;
    term::clear();
    term::disable_wrap();
    term::hide_cursor();
    loop {
        let (angle, f) = gen(k);
        object.rotate(angle);
        object.zoom(f);
        canvas.clear();
        canvas
            .paint(&object, 1.5 * side_len, 1.5 * side_len)
            .unwrap();
        term::move_to(0, 0);
        println!("{}", canvas.frame());
        std::thread::sleep(std::time::Duration::from_millis(32));
        k += 1;
    }
}
