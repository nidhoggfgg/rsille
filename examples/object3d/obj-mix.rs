use std::sync::{Arc, Mutex};

use rsille::{color::Color, extra::Object3D, Animation};

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
            ((0, 1), Color::AnsiValue(240)),
            ((1, 4), Color::AnsiValue(220)),
            ((4, 2), Color::AnsiValue(200)),
            ((2, 0), Color::AnsiValue(180)),
            ((3, 5), Color::AnsiValue(160)),
            ((5, 7), Color::AnsiValue(140)),
            ((7, 6), Color::AnsiValue(140)),
            ((6, 3), Color::AnsiValue(160)),
            ((1, 5), Color::AnsiValue(180)),
            ((4, 7), Color::AnsiValue(200)),
            ((2, 6), Color::AnsiValue(220)),
            ((0, 3), Color::AnsiValue(240)),
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
            ((0, 1), Color::AnsiValue(100)),
            ((0, 2), Color::AnsiValue(120)),
            ((0, 3), Color::AnsiValue(140)),
            ((0, 4), Color::AnsiValue(160)),
            ((5, 1), Color::AnsiValue(180)),
            ((5, 2), Color::AnsiValue(200)),
            ((5, 3), Color::AnsiValue(200)),
            ((5, 4), Color::AnsiValue(180)),
            ((1, 2), Color::AnsiValue(160)),
            ((2, 3), Color::AnsiValue(140)),
            ((3, 4), Color::AnsiValue(120)),
            ((4, 1), Color::AnsiValue(100)),
        ])
        .unwrap();
    nocolor.add_sides(&colorful.sides()).unwrap();
    (colorful, nocolor)
}

fn gen(k: i32) -> (f64, f64, f64) {
    match k {
        k if k % 4 == 3 => (1.0, 2.0, 3.0),
        k if k % 4 == 2 => (-2.0, -3.0, 4.0),
        k if k % 4 == 1 => (2.0, 3.0, 4.0),
        k if k % 4 == 0 => (-1.0, -2.0, -3.0),
        _ => panic!("impossible"),
    }
}

fn main() {
    let side_len = 40.0;
    let mut anime = Animation::new();
    let (cotc, otc) = gen_octahedron(side_len);
    let (ccube, cube) = gen_cube(side_len);
    let objs = [
        (otc, (0.0, 0.0)),
        (cotc, (70.0, 0.0)),
        (ccube, (0.0, 70.0)),
        (cube, (70.0, 70.0)),
    ];
    let k = Arc::new(Mutex::new(0));
    for (obj, location) in objs {
        let k = Arc::clone(&k);
        anime.push(
            obj.clone(),
            move |obj| {
                let mut k = k.lock().unwrap();
                let angle = gen(*k);
                obj.rotate(angle);
                *k += 1;
                *k > 1200
            },
            location,
        );
    }
    anime.set_maxy(110);
    anime.run();
}
