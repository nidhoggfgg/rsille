use std::env;

use rsille::{extra::{math::glm::Vec3, Object3D}, Animation, Canvas};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("useage: [{}] <path>", args[0]);
        return;
    }
    let path = &args[1];
    let mut anime = Animation::new().with_fps(15);
    let obj = if let Ok(obj) = Object3D::from_obj(path, 200.0) {
        obj
    } else {
        println!("useage: [{}] <path>", args[0]);
        return;
    };
    anime.push(obj, |obj| {
        let angle = Vec3::new(1.0, 2.0, 3.0);
        obj.rotate(angle);
        false
    }, (0, 0));
    anime.run();
}
