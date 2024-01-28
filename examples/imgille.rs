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
            println!("useage: [{}] <path>", args[0]);
            return;
        };
        canvas.paint(&imgille, 0.0, 0.0).unwrap();
        println!("{}", canvas.frame());
    } else {
        println!("useage: [{}] <path>", args[0]);
    }

    // for debug
    // let path = "test-files/a.jpeg";
    // let mut canvas = Canvas::new();
    // let img = Imgille::new(path).unwrap();
    // canvas.paint(&img, 0.0, 0.0).unwrap();
    // println!("{}", canvas.frame());
}
