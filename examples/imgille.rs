use std::env;

use rsille::{Canvas, Imgille};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 2 {
        let path = &args[1];
        let mut canvas = Canvas::new();
        let imgille = if let Ok(img) = Imgille::new(path) {
            img
        } else {
            panic!("can't make imgille");
        };
        canvas.paint(&imgille, 0.0, 0.0).unwrap();
        println!("{}", canvas.frame());
    } else {
        println!("useage: [{}] <path>", args[0]);
    }
}
