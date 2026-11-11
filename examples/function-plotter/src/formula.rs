//! Presentation only: meval remains the authority on valid plotting input.
use meval::tokenizer::{Operation as Op, Token};
use rsille::raster::{RasterCanvas, Rgb};
use std::{cell::RefCell, sync::Arc};

const BG: Rgb = Rgb(18, 22, 30);

#[derive(Clone)]
struct Layout {
    w: u32,
    h: u32,
    axis: u32,
    ink: Vec<(u32, u32)>,
    compound: bool,
}
impl Layout {
    fn blank(w: u32, h: u32, axis: u32) -> Self {
        Self {
            w,
            h,
            axis,
            ink: Vec::new(),
            compound: false,
        }
    }
    fn put(&mut self, other: &Self, x: u32, y: u32) {
        self.ink
            .extend(other.ink.iter().map(|&(a, b)| (a + x, b + y)));
    }
    fn line(&mut self, x: u32, y: u32, w: u32, h: u32) {
        for b in y..y + h {
            for a in x..x + w {
                self.ink.push((a, b));
            }
        }
    }
}
fn text(value: &str, s: u32) -> Option<Layout> {
    let mut out = Layout::blank(value.chars().count() as u32 * 6 * s, 7 * s, 3 * s);
    for (i, c) in value.chars().enumerate() {
        let rows = glyph(c)?;
        for (y, row) in rows.iter().enumerate() {
            for x in 0..5 {
                if row & (1 << (4 - x)) != 0 {
                    out.line(i as u32 * 6 * s + x * s, y as u32 * s, s, s);
                }
            }
        }
    }
    Some(out)
}
fn join(items: Vec<Layout>, gap: u32) -> Layout {
    let axis = items.iter().map(|a| a.axis).max().unwrap_or(0);
    let below = items.iter().map(|a| a.h - a.axis).max().unwrap_or(0);
    let mut out = Layout::blank(0, axis + below, axis);
    for item in items {
        out.put(&item, out.w, axis - item.axis);
        out.w += item.w + gap;
    }
    out.w = out.w.saturating_sub(gap);
    out
}
fn parens(a: Layout, s: u32) -> Layout {
    let mut out = Layout::blank(a.w + 8 * s, a.h + 2 * s, a.axis + s);
    out.put(&a, 4 * s, s);
    for y in 0..out.h {
        let x = if y < s || y >= out.h - s {
            2 * s
        } else if y < 2 * s || y >= out.h - 2 * s {
            s
        } else {
            0
        };
        out.line(x, y, s, 1);
        out.line(out.w - s - x, y, s, 1);
    }
    out
}
#[derive(Clone)]
enum Node {
    Atom(String),
    Unary(Op, Box<Node>),
    Binary(Op, Box<Node>, Box<Node>),
    Call(String, Vec<Node>),
}
fn parse(source: &str) -> Option<Node> {
    let expr = source.parse::<meval::Expr>().ok()?;
    // Bound recursive layout work independently of plotting validity.
    if expr.len() > 128 {
        return None;
    }
    let mut stack = Vec::new();
    for token in expr.iter() {
        let node = match token {
            Token::Number(n) => Node::Atom(n.to_string()),
            Token::Var(v) => Node::Atom(
                match v.as_str() {
                    "pi" => "π",
                    "theta" => "θ",
                    _ => v,
                }
                .into(),
            ),
            Token::Unary(op) => Node::Unary(*op, Box::new(stack.pop()?)),
            Token::Binary(op) => {
                let b = stack.pop()?;
                let a = stack.pop()?;
                Node::Binary(*op, Box::new(a), Box::new(b))
            }
            Token::Func(name, Some(n)) => {
                if *n > stack.len() {
                    return None;
                }
                let args = stack.split_off(stack.len() - n);
                Node::Call(name.clone(), args)
            }
            _ => return None,
        };
        stack.push(node);
    }
    if stack.len() == 1 {
        stack.pop()
    } else {
        None
    }
}
fn layout(node: &Node, s: u32) -> Option<Layout> {
    let out = match node {
        Node::Atom(a) => text(a, s)?,
        Node::Unary(op, a) => {
            let a = layout(a, s)?;
            let a = if a.compound { parens(a, s) } else { a };
            let mut out = join(
                vec![text(if *op == Op::Minus { "-" } else { "+" }, s)?, a],
                s,
            );
            out.compound = true;
            out
        }
        Node::Binary(op, a, b) => {
            let a = layout(a, s)?;
            let b = layout(
                b,
                if *op == Op::Pow {
                    (s * 2 / 3).max(1)
                } else {
                    s
                },
            )?;
            match op {
                Op::Div => {
                    let w = a.w.max(b.w) + 4 * s;
                    let mut out = Layout::blank(w, a.h + b.h + 5 * s, a.h + 2 * s);
                    out.put(&a, (w - a.w) / 2, 0);
                    out.line(0, a.h + 2 * s, w, s);
                    out.put(&b, (w - b.w) / 2, a.h + 5 * s);
                    out
                }
                Op::Pow => {
                    let a = if a.compound { parens(a, s) } else { a };
                    let rise = b.h.max(3 * s);
                    let mut out = Layout::blank(a.w + b.w + s, a.h + rise, a.axis + rise);
                    out.put(&a, 0, rise);
                    out.put(&b, a.w + s, 0);
                    // Parenthesize a power used as the base of another power.
                    out.compound = true;
                    out
                }
                _ => {
                    let a = if a.compound { parens(a, s) } else { a };
                    let b = if b.compound { parens(b, s) } else { b };
                    let symbol = match op {
                        Op::Plus => "+",
                        Op::Minus => "-",
                        Op::Times => "·",
                        Op::Rem => "%",
                        _ => return None,
                    };
                    let mut out = join(vec![a, text(symbol, s)?, b], s);
                    out.compound = true;
                    out
                }
            }
        }
        Node::Call(name, args) => {
            let args: Vec<_> = args.iter().map(|a| layout(a, s)).collect::<Option<_>>()?;
            if name == "sqrt" && args.len() == 1 {
                let a = &args[0];
                let mut out = Layout::blank(a.w + 9 * s, a.h + 3 * s, a.axis + 3 * s);
                out.put(a, 9 * s, 3 * s);
                out.line(7 * s, 0, a.w + 2 * s, s);
                for y in 0..out.h {
                    let x = if y < out.h - 3 * s { 7 * s } else { 4 * s };
                    out.line(x, y, s, 1);
                }
                for i in 0..4 * s {
                    out.line(i, out.h - 4 * s + i / 2, s, s);
                }
                out
            } else {
                let mut separated = Vec::new();
                for a in args {
                    if !separated.is_empty() {
                        separated.push(text(",", s)?);
                    }
                    separated.push(a);
                }
                join(vec![text(name, s)?, parens(join(separated, s), s)], s)
            }
        }
    };
    if out.w > 8192 || out.h > 2048 || out.ink.len() > 500_000 {
        None
    } else {
        Some(out)
    }
}

pub struct Formula {
    rows: Vec<(String, Option<Node>, usize)>,
    cache: RefCell<Option<((u32, u32, u32), Arc<RasterCanvas>)>>,
}
impl Formula {
    pub fn new(source: &str) -> Self {
        let mut rows = Vec::new();
        for (color, source) in source.split(';').enumerate() {
            let source = source.trim();
            let geometric = ["polar", "param"].iter().find_map(|name| {
                let body = source
                    .strip_prefix(name)?
                    .trim()
                    .strip_prefix('(')?
                    .strip_suffix(')')?;
                let mut depth = 0i32;
                let args: Vec<_> = body
                    .split(|c| {
                        match c {
                            '(' => depth += 1,
                            ')' => depth -= 1,
                            _ => {}
                        };
                        c == ',' && depth == 0
                    })
                    .map(str::trim)
                    .collect();
                Some((*name, args))
            });
            if let Some((name, args)) = geometric {
                let count = if name == "param" { 2 } else { 1 };
                if args.len() == count + 2 {
                    for i in 0..count {
                        let prefix = if name == "polar" {
                            "r(θ)"
                        } else if i == 0 {
                            "x(t)"
                        } else {
                            "y(t)"
                        };
                        let original = format!("{prefix} = {}", args[i]);
                        let node =
                            parse(args[i]).map(|n| Node::Call(format!("@{prefix}"), vec![n]));
                        rows.push((original, node, color));
                    }
                    let domain = format!(
                        "{} in [{}, {}]",
                        if name == "polar" { "theta" } else { "t" },
                        args[count],
                        args[count + 1]
                    );
                    rows.push((domain.clone(), Some(Node::Atom(domain)), color));
                    continue;
                }
            }
            rows.push((source.to_owned(), parse(source), color));
        }
        Self {
            rows,
            cache: RefCell::new(None),
        }
    }
    pub fn text_rows(&self) -> impl Iterator<Item = (&str, usize)> {
        self.rows.iter().map(|(s, _, c)| (s.as_str(), *c))
    }
    fn layouts(&self, s: u32) -> Option<Vec<(Layout, usize)>> {
        self.rows
            .iter()
            .map(|(_, node, c)| {
                let l = match node {
                    Some(Node::Call(name, args)) if name.starts_with('@') => join(
                        vec![text(&format!("{} =", &name[1..]), s)?, layout(&args[0], s)?],
                        s,
                    ),
                    Some(n) => layout(n, s)?,
                    None => return None,
                };
                Some((l, *c))
            })
            .collect()
    }
    pub fn height(&self, cell_height: u16) -> u16 {
        let s = scale(cell_height);
        let h = self
            .layouts(s)
            .map(|rows| rows.iter().map(|(l, _)| l.h + 4 * s).sum::<u32>() + 2 * s)
            .unwrap_or(self.rows.len() as u32 * u32::from(cell_height));
        h.div_ceil(u32::from(cell_height).max(1)).clamp(1, 12) as u16
    }
    pub fn image(&self, size: (u32, u32), cell_height: u16) -> Option<Arc<RasterCanvas>> {
        let s = scale(cell_height);
        let key = (size.0, size.1, s);
        if let Some((old, image)) = self.cache.borrow().as_ref() {
            if *old == key {
                return Some(Arc::clone(image));
            }
        }
        let rows = self.layouts(s)?;
        if rows.iter().any(|(l, _)| l.w + 4 * s > size.0)
            || rows.iter().map(|(l, _)| l.h + 4 * s).sum::<u32>() + 2 * s > size.1
        {
            return None;
        }
        let mut canvas = RasterCanvas::new(size.0, size.1, BG).ok()?;
        let mut y = 2 * s;
        for (l, c) in rows {
            for (x, dy) in l.ink {
                canvas.set_pixel((x + 2 * s) as i32, (y + dy) as i32, crate::plot::COLORS[c]);
            }
            y += l.h + 4 * s;
        }
        let image = Arc::new(canvas);
        *self.cache.borrow_mut() = Some((key, Arc::clone(&image)));
        Some(image)
    }
}
fn scale(height: u16) -> u32 {
    (u32::from(height) / 8).clamp(2, 5)
}

// Original 5×7 bitmap alphabet; integer scaling keeps stems crisp at native pixels.
fn glyph(c: char) -> Option<[u8; 7]> {
    Some(match c {
        'a' => [0, 0, 14, 1, 15, 17, 15],
        'b' => [16, 16, 30, 17, 17, 17, 30],
        'c' => [0, 0, 14, 16, 16, 17, 14],
        'd' => [1, 1, 15, 17, 17, 17, 15],
        'e' => [0, 0, 14, 17, 31, 16, 14],
        'f' => [6, 8, 8, 30, 8, 8, 8],
        'g' => [0, 15, 17, 17, 15, 1, 14],
        'h' => [16, 16, 30, 17, 17, 17, 17],
        'i' => [4, 0, 12, 4, 4, 4, 14],
        'j' => [2, 0, 6, 2, 2, 18, 12],
        'k' => [16, 16, 18, 20, 24, 20, 18],
        'l' => [12, 4, 4, 4, 4, 4, 14],
        'm' => [0, 0, 26, 21, 21, 21, 21],
        'n' => [0, 0, 30, 17, 17, 17, 17],
        'o' => [0, 0, 14, 17, 17, 17, 14],
        'p' => [0, 30, 17, 17, 30, 16, 16],
        'q' => [0, 15, 17, 17, 15, 1, 1],
        'r' => [0, 0, 22, 25, 16, 16, 16],
        's' => [0, 0, 15, 16, 14, 1, 30],
        't' => [8, 8, 30, 8, 8, 9, 6],
        'u' => [0, 0, 17, 17, 17, 19, 13],
        'v' => [0, 0, 17, 17, 17, 10, 4],
        'w' => [0, 0, 17, 17, 21, 21, 10],
        'x' => [0, 0, 17, 10, 4, 10, 17],
        'y' => [0, 17, 17, 17, 15, 1, 14],
        'z' => [0, 0, 31, 2, 4, 8, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        'π' => [0, 0, 31, 10, 10, 10, 9],
        'θ' => [14, 17, 17, 31, 17, 17, 14],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '=' => [0, 0, 31, 0, 31, 0, 0],
        '·' => [0, 0, 0, 4, 0, 0, 0],
        '%' => [25, 25, 2, 4, 8, 19, 19],
        '.' => [0, 0, 0, 0, 0, 6, 6],
        ',' => [0, 0, 0, 0, 6, 6, 4],
        '(' => [2, 4, 8, 8, 8, 4, 2],
        ')' => [8, 4, 2, 2, 2, 4, 8],
        '[' => [14, 8, 8, 8, 8, 8, 14],
        ']' => [14, 2, 2, 2, 2, 2, 14],
        '*' => [0, 21, 14, 31, 14, 21, 0],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        '^' => [4, 10, 17, 0, 0, 0, 0],
        ' ' => [0; 7],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn powers_fractions_roots_and_nested_structure() {
        let plain = layout(&parse("x").unwrap(), 2).unwrap();
        for source in [
            "x^2",
            "1/x",
            "sqrt(x)",
            "sqrt((x+1)/(x^2+1))",
            "(x^2)^3",
            "x^(2^3)",
            "-x^2",
            "max(sin(x), cos(x))",
        ] {
            let l = layout(&parse(source).unwrap(), 2).unwrap();
            assert!(l.h > plain.h, "{source}");
            assert!(l.ink.iter().all(|&(x, y)| x < l.w && y < l.h));
        }
        let fraction = layout(&parse("1/x").unwrap(), 2).unwrap();
        assert!((0..fraction.w).all(|x| fraction.ink.contains(&(x, fraction.axis))));
        let power = layout(&parse("x^2").unwrap(), 3).unwrap();
        assert!(power.axis > plain.axis);
    }
    #[test]
    fn geometry_colors_cache_and_graceful_limits() {
        let f = Formula::new("x^2; polar(1+cos(theta),0,2*pi); param(cos(t),sin(t),0,2*pi)");
        let rows: Vec<_> = f.text_rows().collect();
        assert_eq!(rows.len(), 6);
        assert!(rows[1].0.starts_with("r(θ)"));
        assert!(rows[3].0.starts_with("x(t)"));
        assert!(rows[4].0.starts_with("y(t)"));
        assert_eq!(rows[4].1, 2);
        let a = f.image((1000, 500), 16).unwrap();
        assert!(Arc::ptr_eq(&a, &f.image((1000, 500), 16).unwrap()));
        assert!(f.image((10, 10), 16).is_none());
        assert!(Formula::new(&vec!["x"; 150].join("+"))
            .image((1000, 500), 16)
            .is_none());
        assert!(Formula::new("unsupported_name")
            .image((1000, 500), 16)
            .is_none());
    }
}
