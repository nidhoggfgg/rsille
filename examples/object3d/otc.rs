use rsille::{extra::{math::glm::{Pos3, Vec3}, object3d::Object3D}, Animation};

fn gen_octahedron(side_len: f32) -> Object3D {
    #[rustfmt::skip]
    let a = [
        ( 0,  0,  1),
        ( 1,  0,  0),
        ( 0,  1,  0),
        (-1,  0,  0),
        ( 0, -1,  0),
        ( 0,  0, -1),
    ];
    let mut points = Vec::new();
    let mut object = Object3D::new();
    for i in a {
        let x = side_len * i.0 as f32;
        let y = side_len * i.1 as f32;
        let z = side_len * i.2 as f32;
        points.push(Pos3::new(x, y, z));
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

fn main() {
    let side_len = 40.0;
    let mut anime = Animation::new();
    let object = gen_octahedron(side_len);
    anime.push(
        object,
        |obj| {
            let angle = Vec3::new(1.0, 2.0, 3.0);
            obj.rotate(angle);
            false
        },
        (0, 0),
    );
    anime.run();
}
