use rsille::{object3d::Object3D, Canvas};

// generate the vertices(6) of cube and sides(12) of cube
// the sides contain the index of the vertice
fn gen_cube(side_len: f64) -> Object3D {
    let a = [
        (-1, -1, -1),
        (-1, -1, 1),
        (-1, 1, -1),
        (1, -1, -1),
        (-1, 1, 1),
        (1, -1, 1),
        (1, 1, -1),
        (1, 1, 1),
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

#[cfg(feature = "color")]
fn gen_cube_colorful(side_len: f64) -> Object3D {
    use rsille::color::TermColor;
    let a = [
        (-1, -1, -1),
        (-1, -1, 1),
        (-1, 1, -1),
        (1, -1, -1),
        (-1, 1, 1),
        (1, -1, 1),
        (1, 1, -1),
        (1, 1, 1),
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
        .add_sides_colorful(&[
            ((0, 1), TermColor::C256(240)),
            ((1, 4), TermColor::C256(220)),
            ((4, 2), TermColor::C256(200)),
            ((2, 0), TermColor::C256(180)),
            ((3, 5), TermColor::C256(160)),
            ((5, 7), TermColor::C256(140)),
            ((7, 6), TermColor::C256(140)),
            ((6, 3), TermColor::C256(160)),
            ((1, 5), TermColor::C256(180)),
            ((4, 7), TermColor::C256(200)),
            ((2, 6), TermColor::C256(220)),
            ((0, 3), TermColor::C256(240)),
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
    return (rotate, zoom);
}

fn main() {
    let side_len = 30.0;
    let mut canvas = Canvas::new();
    let mut object = gen_cube(side_len);
    #[cfg(feature = "color")]
    let mut object_colorful = gen_cube_colorful(side_len);
    let mut k = 0;
    // hide the cursor and clear screen
    println!("\x1B[?25l\x1B[2J");
    loop {
        let (angle, f) = gen(k);
        object.rotate(angle);
        object.zoom(f);
        canvas.clear();
        canvas
            .paint(&object, 1.5 * side_len, 1.5 * side_len)
            .unwrap();
        #[cfg(feature = "color")]
        {
            object_colorful.rotate(angle);
            object_colorful.zoom(f);
            canvas
                .paint(&object_colorful, 4.0 * side_len, 1.5 * side_len)
                .unwrap();
        }
        println!("\x1B[H{}", canvas.frame());
        std::thread::sleep(std::time::Duration::from_millis(32));
        k += 1;
    }
}
