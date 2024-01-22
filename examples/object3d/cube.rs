use rsille::object3d::Object3D;

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
    object.add_sides(&[
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
    ]);
    object
}

// just make the rotate looks more "random"
fn gen_rotate(k: i32) -> (f64, f64, f64) {
    match k {
        k if k % 2 == 0 => (1.0, 2.0, 3.0),
        k if k % 5 == 0 => (2.0, 3.0, 4.0),
        _ => (3.0, 4.0, 5.0),
    }
}

fn main() {
    let side_len = 30.0;
    let mut object = gen_cube(side_len);
    let mut k = 0;
    // hide the cursor and clear screen
    println!("\x1B[?25l\x1B[2J");
    loop {
        let (rx, ry, rz) = gen_rotate(k);
        object.rotate_xyz(rx, ry, rz);
        println!("\x1B[H{}", object.draw(side_len, side_len));
        std::thread::sleep(std::time::Duration::from_millis(32));
        k += 1;
    }
}