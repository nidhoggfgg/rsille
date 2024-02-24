use rsille::{extra::Object3D, Animation};

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
    let object = Object3D::cube(side_len);
    let mut k = 0;
    anime.push(
        object,
        move |obj| {
            let (angle, zoom) = gen(k);
            obj.rotate(angle);
            obj.zoom(zoom);
            k += 1;
            k > 1500
        },
        (0.0, 0.0),
    );
    anime.set_size(1.5 * side_len, 1.5 * side_len);
    anime.run();
}
