use rsille::{color::Color, extra::Object3D, Animation};

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
    let mut anime = Animation::new();
    let object = gen_cube(side_len);
    let mut k = 0;
    anime.push(
        object,
        move |obj| {
            let (angle, f) = gen(k);
            obj.rotate(angle);
            obj.zoom(f);
            k += 1;
            k > 180
        },
        (0.0, 0.0),
    );
    anime.set_maxy(1.5 * side_len);
    anime.set_minx(-1.5 * side_len);
    anime.run();
}
