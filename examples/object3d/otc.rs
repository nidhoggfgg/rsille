use rsille::{Canvas, Object3D, Paint};

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
    object.add_sides(&[
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
    ]);
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
    let mut k = 0;
    // hide the cursor and clear screen
    println!("\x1B[?25l\x1B[2J");
    loop {
        let ((rx, ry, rz), zoom) = gen(k);
        object.rotate(rx, ry, rz);
        object.zoom(zoom);
        canvas.clear();
        object.paint(&mut canvas, 1.6 * side_len, 1.6 * side_len);
        println!("\x1B[H{}", canvas.frame());
        std::thread::sleep(std::time::Duration::from_millis(32));
        k += 1;
    }
}
