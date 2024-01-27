use rsille::{object3d::Object3D, Canvas, Paint};

// generate the vertices(6) of cube and sides(12) of cube
// the sides contain the index of the vertice
fn gen_octahedron(side_len: f64) -> Object3D {
    let a = [
        (0, 0, 1),
        (1, 0, 0),
        (0, 1, 0),
        (-1, 0, 0),
        (0, -1, 0),
        (0, 0, -1),
    ];
    let mut points = Vec::new();
    let mut object = Object3D::new();
    for i in a {
        let x = side_len * i.0 as f64;
        let y = side_len * i.1 as f64;
        let z = side_len * i.2 as f64;
        points.push((x, y, z));
    }
    object.add_points(&points);
    object
        .add_sides(&[
            (0, 1),
            (0, 2),
            (0, 3),
            (0, 4),
            (5, 1),
            (5, 2),
            (5, 3),
            (5, 4),
            (1, 2),
            (2, 3),
            (3, 4),
            (4, 1),
        ])
        .unwrap();
    object
}

#[cfg(feature = "color")]
fn gen_octahedron_colorful(side_len: f64) -> Object3D {
    use rsille::color::TermColor;

    let a = [
        (0, 0, 1),
        (1, 0, 0),
        (0, 1, 0),
        (-1, 0, 0),
        (0, -1, 0),
        (0, 0, -1),
    ];
    let mut points = Vec::new();
    let mut object = Object3D::new();
    for i in a {
        let x = side_len * i.0 as f64;
        let y = side_len * i.1 as f64;
        let z = side_len * i.2 as f64;
        points.push((x, y, z));
    }
    object.add_points(&points);
    object
        .add_sides_colorful(&[
            ((0, 1), TermColor::C256(100)),
            ((0, 2), TermColor::C256(120)),
            ((0, 3), TermColor::C256(140)),
            ((0, 4), TermColor::C256(160)),
            ((5, 1), TermColor::C256(180)),
            ((5, 2), TermColor::C256(200)),
            ((5, 3), TermColor::C256(200)),
            ((5, 4), TermColor::C256(180)),
            ((1, 2), TermColor::C256(160)),
            ((2, 3), TermColor::C256(140)),
            ((3, 4), TermColor::C256(120)),
            ((4, 1), TermColor::C256(100)),
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
    let side_len = 40.0;
    let mut canvas = Canvas::new();
    let mut object = gen_octahedron(side_len);
    #[cfg(feature = "color")]
    let mut object_colurful = gen_octahedron_colorful(side_len);
    let mut k = 0;
    // hide the cursor and clear screen
    println!("\x1B[?25l\x1B[2J");
    loop {
        let (angle, zoom) = gen(k);
        object.rotate(angle);
        object.zoom(zoom);
        canvas.clear();
        object.paint(&mut canvas, 1.6 * side_len, 1.6 * side_len).unwrap();
        #[cfg(feature = "color")]
        {
            object_colurful.rotate(angle);
            object_colurful.zoom(zoom);
            object_colurful.paint(&mut canvas, 4.2 * side_len, 1.6 * side_len).unwrap();
        }
        println!("\x1B[H{}", canvas.frame());
        std::thread::sleep(std::time::Duration::from_millis(32));
        k += 1;
    }
}
