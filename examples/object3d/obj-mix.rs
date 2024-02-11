use rsille::{color::TermColor, object3d::Object3D, term, Canvas};

fn gen_cube(side_len: f64) -> (Object3D, Object3D) {
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
    let mut colorful = Object3D::new();
    for i in a {
        let x = side_len / 2.0 * i.0 as f64;
        let y = side_len / 2.0 * i.1 as f64;
        let z = side_len / 2.0 * i.2 as f64;
        points.push((x, y, z));
    }
    colorful.add_points(&points);
    let mut nocolor = colorful.clone();
    colorful
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
    nocolor.add_sides(&colorful.sides()).unwrap();
    (colorful, nocolor)
}

fn gen_octahedron(side_len: f64) -> (Object3D, Object3D) {
    let a = [
        (0, 0, 1),
        (1, 0, 0),
        (0, 1, 0),
        (-1, 0, 0),
        (0, -1, 0),
        (0, 0, -1),
    ];
    let mut points = Vec::new();
    let mut colorful = Object3D::new();
    for i in a {
        let x = side_len * i.0 as f64;
        let y = side_len * i.1 as f64;
        let z = side_len * i.2 as f64;
        points.push((x, y, z));
    }
    colorful.add_points(&points);
    let mut nocolor = colorful.clone();
    colorful
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
    nocolor.add_sides(&colorful.sides()).unwrap();
    (colorful, nocolor)
}

// just make the rotate looks more "random"
fn gen(k: i32) -> (f64, f64, f64) {
    match k {
        k if k % 4 == 0 => (1.0, 2.0, 3.0),
        k if k % 4 == 1 => (-2.0, -3.0, 4.0),
        k if k % 4 == 2 => (2.0, 3.0, 4.0),
        k if k % 4 == 3 => (-1.0, -2.0, -3.0),
        _ => panic!("impossible"),
    }
    // let zoom = if k % 120 <= 60 {
    //     1.0 + (k % 120) as f64 * 0.01
    // } else {
    //     1.6 - (k % 120 - 60) as f64 * 0.01
    // };
    // (rotate, zoom)
}

fn main() {
    let side_len = 40.0;
    let mut canvas = Canvas::new();
    let (cotc, otc) = gen_octahedron(side_len);
    let (ccube, cube) = gen_cube(side_len);
    let mut objs = [
        (otc, (50.0, 50.0)),
        (cotc, (115.0, 50.0)),
        (ccube, (50.0, 115.0)),
        (cube, (110.0, 115.0)),
    ];
    let mut k = 0;
    term::clear();
    term::disable_wrap();
    term::hide_cursor();
    loop {
        canvas.clear();
        for (obj, location) in &mut objs {
            // let (angle, zoom) = gen(k);
            // obj.zoom(zoom);
            let angle = gen(k);
            obj.rotate(angle);
            canvas.paint(obj, location.0, location.1).unwrap();
            k += 1;
        }
        term::move_to(0, 0);
        println!("{}", canvas.frame());
        std::thread::sleep(std::time::Duration::from_millis(64));
    }
}
