use std::env;

use rsille::{extra::Imgille, Canvas};

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
        canvas.print().unwrap();
    } else {
        println!("useage: [{}] <path>", args[0]);
    }
}
