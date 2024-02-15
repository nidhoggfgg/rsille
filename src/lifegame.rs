use std::{fs, ops::RangeBounds};

use crate::{
    utils::{Offset, RsilleErr},
    Paint,
};

type Grid = Vec<Vec<bool>>;

#[derive(Debug, Clone)]
pub struct LifeGame {
    offset: Offset,
    grid: Grid,
}

impl LifeGame {
    pub fn new() -> Self {
        Self {
            offset: Offset(0, 0),
            grid: Vec::new(),
        }
    }

    pub fn from(path: &str) -> Result<LifeGame, RsilleErr> {
        let err = Err(RsilleErr::new(format!("can't open rle file: {}", path)));
        let Ok(rle) = fs::read_to_string(path) else {
            return err;
        };
        let grid = parse(&rle)?;
        Ok(Self {
            offset: Offset(0, 0),
            grid,
        })
    }

    pub fn next(&mut self) {
        let mut next = self.grid.clone();
        for (y, cs) in self.grid.iter().enumerate() {
            for (x, _) in cs.iter().enumerate() {
                let count = self.count_neighbors(x, y);
                // neibor == 3 then alive
                if count == 3 {
                    next[y][x] = true;
                }
                // neibor < 2 or 4 <= neibor then dead
                if !(2..4).contains(&count) {
                    next[y][x] = false
                }
            }
        }
    }

    fn count_neighbors(&self, x: usize, y: usize) -> usize {
        let mut count = 0;
        let (height, width) = (self.grid.len() as isize, self.grid[0].len() as isize);
        for i in -1..=1 {
            for j in -1..=1 {
                if i == 0 && j == 0 {
                    continue;
                }
                let neighbor_x = x as isize + j;
                let neighbor_y = y as isize + i;
                if 0 <= neighbor_x && neighbor_x < width && 0 <= neighbor_y && neighbor_y < height {
                    count += self.grid[neighbor_y as usize][neighbor_x as usize] as usize;
                }
            }
        }
        count
    }
}

impl Paint for LifeGame {
    fn paint(&self, canvas: &mut crate::Canvas, x: f64, y: f64) -> Result<(), RsilleErr> {
        for (iy, cs) in self.grid.iter().enumerate() {
            for (ix, c) in cs.iter().enumerate() {
                if *c {
                    canvas.set(ix as f64 + x, iy as f64 + y);
                }
            }
        }
        Ok(())
    }
}

fn parse(rle: &str) -> Result<Grid, RsilleErr> {
    let mut lines = rle.lines().peekable();
    let mut grid = Vec::new();
    let mut width = 0;
    let mut height = 0;

    // width and height
    // skip comments and rules
    while let Some(line) = lines.peek() {
        if line.starts_with('#') {
            lines.next();
            continue;
        }
        if line.starts_with('x') {
            let s: Vec<&str> = line.split(',').collect();
            width = (s[0].split('=').collect::<Vec<&str>>())[1]
                .trim()
                .parse()
                .unwrap();
            height = (s[1].split('=').collect::<Vec<&str>>())[1]
                .trim()
                .parse()
                .unwrap();
            grid = vec![vec![false; width]; height];
            break;
        }
    }
    if width == 0 || height == 0 {
        return Err(RsilleErr::new("can't parse width or height".to_string()));
    }

    // parse
    let mut x = 0;
    let mut y = 0;
    for line in lines {
        if line.starts_with('#') || line.starts_with('x') {
            continue;
        }
        let mut chars = line.chars().peekable();
        loop {
            let Some(c) = chars.next() else {
                break;
            };
            match c {
                'b' => x += 1,
                'o' => {
                    grid[y][x] = true;
                    x += 1;
                }
                '$' => {
                    y += 1;
                    x = 0;
                }
                '!' => break,
                '0'..='9' => {
                    let mut count_str = String::new();
                    count_str.push(c);
                    while let Some(digit) = chars.peek() {
                        if digit.is_ascii_digit() {
                            let digit = chars.next().unwrap();
                            count_str.push(digit);
                        } else {
                            break;
                        }
                    }
                    let count: usize = count_str.parse().unwrap();
                    x += count - 1;
                }
                _ => (),
            }
        }
    }
    Ok(grid)
}
