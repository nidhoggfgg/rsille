use std::env;

use rsille::{lifegame::LifeGame, Animation};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("useage: [{}] <path>", args[0]);
        return;
    }
    let lg = if let Ok(lg) = LifeGame::from_path(&args[1]) {
        lg
    } else {
        println!("can't parse {}!", args[1]);
        return;
    };
    let mut anime = Animation::new();
    anime.push(lg, |lg: &mut LifeGame| lg.update(), (0.0, 0.0));
    anime.run();
}
