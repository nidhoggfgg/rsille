use std::io::{self, Stdout};

pub struct Terminal {
    width: u32,
    height: u32,
    stdout: Stdout,
}

impl Terminal {
    pub fn new() -> Self {
        Terminal {
            width: 0,
            height: 0,
            stdout: io::stdout(),
        }
    }
}
