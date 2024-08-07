use std::{collections::HashMap, fs, iter::Peekable};

use crate::{utils::RsilleErr, Paint};

// same as HashSet<(isize, isize)>
// but when using inplace algorithms, just simply change () to bool or u8
type LiveCells = HashMap<(isize, isize), ()>;

/// The life game
///
/// It support `rle` file download from the internet
///
/// ## Example
///
/// ```no_run
/// use rsille::{extra::LifeGame, Animation};
/// let lg = LifeGame::from_path("path/to/rle").unwrap();
/// let mut anime = Animation::new();
/// anime.push(lg, |lg| lg.update(), (0, 0));
/// anime.run();
/// ```
#[derive(Debug, Clone)]
pub struct LifeGame {
    cells: LiveCells,
    // boundless: bool,
}

impl LifeGame {
    /// Return a new life game
    pub fn new() -> Self {
        Self {
            cells: Default::default(),
            // boundless: true,
        }
    }

    /// Read the `rle` format string and build a life game from it
    ///
    /// Return `err` when can't parse the rle string
    pub fn from(rle: &str) -> Result<Self, RsilleErr> {
        parse(rle)
    }

    /// Read the `rle` file and build a life game from it
    ///
    /// Return `err` when can't parse the rle file or can't open file
    pub fn from_path(path: &str) -> Result<Self, RsilleErr> {
        let Ok(rle) = fs::read_to_string(path) else {
            return Err(RsilleErr::new(format!("can't open rle file: {}", path)));
        };
        Self::from(&rle)
    }

    /// The next moment of the cells
    pub fn update(&mut self) -> bool {
        let mut next = self.cells.clone();
        for cell in self.cells.keys() {
            let (x, y) = *cell;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let neibor = self.count_neighbors(x + dx, y + dy);
                    if neibor == 3 {
                        next.insert((x + dx, y + dy), ());
                    }

                    if !(2..4).contains(&neibor) {
                        next.remove(&(x + dx, y + dy));
                    }
                }
            }
        }
        self.cells = next;

        false
    }

    fn count_neighbors(&self, x: isize, y: isize) -> usize {
        let mut count = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                // the cell itself
                if dy == 0 && dx == 0 {
                    continue;
                }
                let neighbor_x = x + dx;
                let neighbor_y = y + dy;
                if self.cells.contains_key(&(neighbor_x, neighbor_y)) {
                    count += 1;
                }
            }
        }
        count
    }
}

impl Paint for LifeGame {
    fn paint<T>(&self, canvas: &mut crate::Canvas, x: T, y: T) -> Result<(), RsilleErr>
    where
        T: Into<f64>,
    {
        let (x, y) = (x.into(), y.into());
        for cell in self.cells.keys() {
            canvas.set(x + cell.0 as f64, y + cell.1 as f64);
        }
        Ok(())
    }
}

fn parse(rle: &str) -> Result<LifeGame, RsilleErr> {
    let mut lines = rle.lines().peekable();
    let mut cells = HashMap::new();
    // read the head
    // let (width, height) = read_head(&mut lines)?;
    let _ = read_head(&mut lines)?;
    // parse
    let (mut x, mut y) = (0_isize, 0_isize);
    let mut count = 0_isize;
    for line in lines {
        let mut chars = line.chars().peekable();
        loop {
            let Some(c) = chars.next() else {
                break;
            };
            match c {
                'b' => {
                    for _ in 0..=count {
                        x += 1;
                    }
                    count = 0;
                }
                'o' => {
                    for _ in 0..=count {
                        cells.insert((x, y), ());
                        x += 1;
                    }
                    count = 0;
                }
                '$' => {
                    y += 1;
                    x = 0;
                }
                '!' => break,
                '1'..='9' => {
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
                    count = count_str.parse().unwrap();
                    count -= 1;
                }
                _ => (),
            }
        }
    }
    Ok(LifeGame {
        cells,
        // boundless: false,
    })
}

fn read_head<'a, I>(lines: &mut Peekable<I>) -> Result<(usize, usize), RsilleErr>
where
    I: Iterator<Item = &'a str>,
{
    while let Some(line) = lines.peek() {
        if line.starts_with('#') {
            lines.next();
            continue;
        }
        if line.starts_with('x') {
            let s: Vec<&str> = line.split(',').collect();
            let width = (s[0].split('=').collect::<Vec<&str>>())[1]
                .trim()
                .parse()
                .unwrap();
            let height = (s[1].split('=').collect::<Vec<&str>>())[1]
                .trim()
                .parse()
                .unwrap();
            return Ok((width, height));
        }
        if line.is_empty() {
            lines.next();
            continue;
        }
    }
    Err(RsilleErr::new("can't parse width or height".to_string()))
}
