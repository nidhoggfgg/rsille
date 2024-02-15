use std::env;

use rsille::{lifegame::LifeGame, term, Canvas};

fn main() {
    let args: Vec<String> = env::args().collect();
    // if args.len() != 2 {
    //     println!("useage: [{}] <path>", args[0]);
    //     return
    // }
    let path = "examples/files/112p15_synth.rle";
    let mut canvas = Canvas::new();
    let mut lg = if let Ok(lg) = LifeGame::from(path) {
        lg
    } else {
        println!("can't parse {}!", args[1]);
        return;
    };
    term::clear();
    term::disable_wrap();
    loop {
        canvas.clear();
        canvas.paint(&lg, 0.0, 0.0).unwrap();
        term::move_to(0, 0);
        println!("{}", canvas.frame());
        lg.next();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
