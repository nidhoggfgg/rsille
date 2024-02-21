use rsille::{
    color::Color,
    extra::{Object3D, Turtle},
    Animation,
};

fn main() {
    let mut anime = Animation::new();
    let mut t = Turtle::new();
    for i in 0..12 {
        t.right(30.0);
        for j in 0..36 {
            t.color(Color::Rgb {
                r: 240 - i * 10,
                g: 60 + j * 5,
                b: 220,
            });
            t.right(10.0);
            t.forward(4.2);
        }
    }
    let object = gen_cube();
    let mut k = 0;
    t.anime();
    anime.set_fps(60);
    anime.push(t, move |t| t.update(), (50.0, 50.0));
    anime.push(
        object,
        move |obj| {
            obj.rotate((1.0, 2.0, 3.0));
            k += 1;
            k > 12 * 36
        },
        (50.0, 50.0),
    );
    anime.run();
    println!("End!");
}

fn gen_cube() -> Object3D {
    let side_len = 30.0;
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
