//! Presentation only: meval remains the authority on valid plotting input.
use meval::tokenizer::{Operation as Op, Token};
use rsille::raster::{RasterCanvas, Rgb};
use rsille_typst::{MathRenderer, RenderOptions};
use std::{cell::RefCell, sync::Arc};

const BG: Rgb = Rgb(18, 22, 30);

thread_local! {
    // Load bundled fonts once, and only when native formula output is requested.
    static RENDERER: Option<MathRenderer> =
        MathRenderer::new(typst_assets::fonts(), "New Computer Modern Math").ok();
}

// Never pass raw input to Typst: quoted identifiers cannot invoke Typst code.
fn identifier(name: &str) -> String {
    match name {
        "pi" | "theta" => name.into(),
        _ if name.len() == 1 && name.chars().all(|c| c.is_ascii_alphabetic()) => name.into(),
        _ => format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\"")),
    }
}

#[derive(Clone)]
struct Math {
    source: String,
    compound: bool,
}
impl Math {
    fn grouped(&self) -> String {
        if self.compound {
            format!("({})", self.source)
        } else {
            self.source.clone()
        }
    }
}

fn parse(source: &str) -> Option<String> {
    let expr = source.parse::<meval::Expr>().ok()?;
    if expr.len() > 128 {
        return None;
    }
    let mut stack: Vec<Math> = Vec::new();
    for token in expr.iter() {
        let mut compound = false;
        let source = match token {
            Token::Number(n) => n.to_string(),
            Token::Var(v) => identifier(v),
            Token::Unary(op) => {
                let a = stack.pop()?;
                compound = true;
                format!(
                    "{}{}",
                    if *op == Op::Minus { "-" } else { "+" },
                    a.grouped()
                )
            }
            Token::Binary(op) => {
                let b = stack.pop()?;
                let a = stack.pop()?;
                match op {
                    Op::Div => format!("frac({}, {})", a.source, b.source),
                    Op::Pow => {
                        compound = true;
                        format!("{}^({})", a.grouped(), b.source)
                    }
                    _ => {
                        compound = true;
                        let symbol = match op {
                            Op::Plus => "+",
                            Op::Minus => "-",
                            Op::Times => "dot",
                            Op::Rem => "%",
                            _ => return None,
                        };
                        format!("{} {symbol} {}", a.grouped(), b.grouped())
                    }
                }
            }
            Token::Func(name, Some(n)) => {
                if *n > stack.len() {
                    return None;
                }
                let args = stack.split_off(stack.len() - n);
                let args = args
                    .iter()
                    .map(|a| a.source.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                if name == "sqrt" && *n == 1 {
                    format!("sqrt({args})")
                } else if name == "abs" && *n == 1 {
                    format!("abs({args})")
                } else {
                    format!("op({})({args})", identifier(name))
                }
            }
            _ => return None,
        };
        stack.push(Math { source, compound });
    }
    (stack.len() == 1).then(|| stack.pop().unwrap().source)
}

type RenderedRows = Vec<(RasterCanvas, usize)>;

pub struct Formula {
    rows: Vec<(String, usize)>,
    math_rows: Vec<(Option<String>, usize)>,
    // Cache failures too; layout and redraw must not repeatedly compile a formula.
    rendered: RefCell<Option<(u16, Option<RenderedRows>)>>,
    cache: RefCell<Option<((u32, u32, u16), Arc<RasterCanvas>)>>,
}
impl Formula {
    pub fn new(source: &str) -> Self {
        let mut rows = Vec::new();
        let mut math_rows = Vec::new();
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
                    let mut equations = Vec::new();
                    for i in 0..count {
                        let prefix = if name == "polar" {
                            "r(θ)"
                        } else if i == 0 {
                            "x(t)"
                        } else {
                            "y(t)"
                        };
                        let math_prefix = if name == "polar" { "r(theta)" } else { prefix };
                        rows.push((format!("{prefix} = {}", args[i]), color));
                        equations.push(parse(args[i]).map(|s| format!("{math_prefix} = {s}")));
                    }
                    let equations = equations.into_iter().collect::<Option<Vec<_>>>();
                    let math = equations.map(|equations| {
                        if name == "param" {
                            format!("cases({})", equations.join(", "))
                        } else {
                            equations.join(", ")
                        }
                    });
                    let variable = if name == "polar" { "theta" } else { "t" };
                    let domain = format!("{variable} in [{}, {}]", args[count], args[count + 1]);
                    let domain_math = parse(args[count])
                        .zip(parse(args[count + 1]))
                        .map(|(a, b)| format!("{variable} in [{a}, {b}]"));
                    if name == "param" {
                        let (y, _) = rows.pop().unwrap();
                        let (x, _) = rows.pop().unwrap();
                        let width = x.chars().count().max(y.chars().count());
                        rows.push((
                            format!("⎰ {x}{}  {domain}", " ".repeat(width - x.chars().count())),
                            color,
                        ));
                        rows.push((format!("⎱ {y}"), color));
                    } else {
                        rows.last_mut().unwrap().0.push_str(&format!(",  {domain}"));
                    }
                    math_rows.push((
                        math.zip(domain_math)
                            .map(|(equations, domain)| format!("{equations}, quad {domain}")),
                        color,
                    ));
                    continue;
                }
            }
            rows.push((source.to_owned(), color));
            math_rows.push((parse(source), color));
        }
        Self {
            rows,
            math_rows,
            rendered: RefCell::new(None),
            cache: RefCell::new(None),
        }
    }

    pub fn text_rows(&self) -> impl Iterator<Item = (&str, usize)> {
        self.rows.iter().map(|(s, c)| (s.as_str(), *c))
    }

    fn prepare(&self, cell_height: u16) {
        if self
            .rendered
            .borrow()
            .as_ref()
            .is_some_and(|(h, _)| *h == cell_height)
        {
            return;
        }
        let rows = RENDERER.with(|renderer| {
            let renderer = renderer.as_ref()?;
            self.math_rows
                .iter()
                .map(|(math, color)| {
                    let options = RenderOptions {
                        font_size: f64::from(cell_height).clamp(12.0, 40.0),
                        pixels_per_point: 1.0,
                        foreground: *crate::plot::COLORS.get(*color)?,
                        background: BG,
                        max_pixels: 500_000,
                    };
                    let image = renderer.render(math.as_ref()?, &options).ok()?;
                    Some((image, *color))
                })
                .collect::<Option<RenderedRows>>()
        });
        *self.rendered.borrow_mut() = Some((cell_height, rows));
    }

    pub fn width(&self, cell_pixels: (u16, u16)) -> u16 {
        if cell_pixels == (1, 2) {
            return 70;
        }
        self.prepare(cell_pixels.1);
        let rendered = self.rendered.borrow();
        let columns = rendered
            .as_ref()
            .and_then(|(_, rows)| rows.as_ref())
            .map(|rows| {
                let pixels = rows
                    .iter()
                    .map(|(image, _)| image.width())
                    .max()
                    .unwrap_or(0)
                    + 8;
                pixels.div_ceil(u32::from(cell_pixels.0).max(1))
            })
            .unwrap_or_else(|| {
                self.rows
                    .iter()
                    .map(|(text, _)| text.chars().count() as u32)
                    .max()
                    .unwrap_or(1)
            });
        columns.clamp(1, u32::from(u16::MAX)) as u16
    }

    pub fn height(&self, cell_height: u16) -> u16 {
        self.prepare(cell_height);
        let rendered = self.rendered.borrow();
        let h = rendered
            .as_ref()
            .and_then(|(_, rows)| rows.as_ref())
            .map(|rows| rows.iter().map(|(l, _)| l.height() + 4).sum::<u32>() + 4)
            .unwrap_or(self.rows.len() as u32 * u32::from(cell_height));
        h.div_ceil(u32::from(cell_height).max(1))
            .max(self.rows.len() as u32)
            .clamp(1, 12) as u16
    }

    pub fn image(&self, size: (u32, u32), cell_height: u16) -> Option<Arc<RasterCanvas>> {
        let key = (size.0, size.1, cell_height);
        if let Some((old, image)) = self.cache.borrow().as_ref() {
            if *old == key {
                return Some(Arc::clone(image));
            }
        }
        self.prepare(cell_height);
        let rendered = self.rendered.borrow();
        let rows = rendered.as_ref()?.1.as_ref()?;
        if rows.iter().any(|(l, _)| l.width() + 8 > size.0)
            || rows.iter().map(|(l, _)| l.height() + 4).sum::<u32>() + 4 > size.1
        {
            return None;
        }
        let mut canvas = RasterCanvas::new(size.0, size.1, BG).ok()?;
        let mut y = 4;
        for (image, _) in rows {
            for (i, color) in image.pixels().iter().enumerate() {
                canvas.set_pixel(
                    (i as u32 % image.width() + 4) as i32,
                    (y + i as u32 / image.width()) as i32,
                    *color,
                );
            }
            y += image.height() + 4;
        }
        let image = Arc::new(canvas);
        *self.cache.borrow_mut() = Some((key, Arc::clone(&image)));
        Some(image)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_preserves_structure_and_quotes_names() {
        assert_eq!(parse("(x^2)^3").unwrap(), "(x^(2))^(3)");
        assert_eq!(parse("x^(2^3)").unwrap(), "x^(2^(3))");
        assert_eq!(parse("-x^2").unwrap(), "-(x^(2))");
        assert_eq!(parse("1/(x+1)").unwrap(), "frac(1, x + 1)");
        assert_eq!(parse("sqrt(x)").unwrap(), "sqrt(x)");
        assert_eq!(parse("sin(x)").unwrap(), "op(\"sin\")(x)");
        assert_eq!(parse("unsupported_name").unwrap(), "\"unsupported_name\"");
        assert!(parse("#panic(\"bad\")").is_none());
    }

    #[test]
    fn real_typst_rendering_and_graceful_limits() {
        for source in [
            "x^2",
            "1/x",
            "sqrt(x)",
            "sqrt((x+1)/(x^2+1))",
            "max(sin(x), cos(x))",
            "abs(x)",
            "x%2",
            "pi+e",
        ] {
            let f = Formula::new(source);
            let image = f
                .image((1000, 500), 16)
                .unwrap_or_else(|| panic!("{source}"));
            assert!(image.pixels().iter().any(|c| *c != BG), "{source}");
        }
        assert!(Formula::new(&vec!["x"; 150].join("+"))
            .image((1000, 500), 16)
            .is_none());
        assert!(Formula::new("x+").image((1000, 500), 16).is_none());
    }

    #[test]
    fn parametric_equations_share_a_left_brace() {
        let f = Formula::new("param(cos(t),sin(t),0,2*pi); param(t,t^2,-1,1)");
        assert_eq!(f.text_rows().count(), 4);
        assert_eq!(f.math_rows.len(), 2);
        assert_eq!(
            f.math_rows[0].0.as_deref(),
            Some("cases(x(t) = op(\"cos\")(t), y(t) = op(\"sin\")(t)), quad t in [0, 2 dot pi]")
        );
        assert_eq!(
            f.math_rows[1].0.as_deref(),
            Some("cases(x(t) = t, y(t) = t^(2)), quad t in [-1, 1]")
        );
        assert_eq!(f.math_rows[0].1, 0);
        assert_eq!(f.math_rows[1].1, 1);
        assert!(f.image((1000, 500), 16).is_some());
        assert!(f.image((10, 10), 16).is_none());
        let rendered = f.rendered.borrow();
        let rows = rendered.as_ref().unwrap().1.as_ref().unwrap();
        assert_eq!(rows.len(), 2);
        let polar = Formula::new("polar(2,0,2*pi)");
        assert_eq!(
            polar.math_rows[0].0.as_deref(),
            Some("r(theta) = 2, quad theta in [0, 2 dot pi]")
        );
        assert!(polar.image((1000, 500), 16).is_some());
        assert!(
            rows[0].0.height()
                > polar
                    .rendered
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .1
                    .as_ref()
                    .unwrap()[0]
                    .0
                    .height()
        );
        assert!(Formula::new("param(x+,t,0,1)")
            .image((1000, 500), 16)
            .is_none());
    }

    #[test]
    fn geometry_colors_cache_and_resize() {
        let f = Formula::new("x^2; polar(1+cos(theta),0,2*pi); param(cos(t),sin(t),0,2*pi)");
        let rows: Vec<_> = f.text_rows().collect();
        assert_eq!(rows.len(), 4);
        assert!(rows[1].0.starts_with("r(θ)"));
        assert!(rows[1].0.contains("theta in"));
        assert!(rows[2].0.starts_with("⎰ x(t)"));
        assert!(rows[2].0.contains("t in [0, 2*pi]"));
        assert!(rows[3].0.starts_with("⎱ y(t)"));
        assert_eq!(rows[3].1, 2);
        let a = f.image((1000, 500), 16).unwrap();
        assert!(Arc::ptr_eq(&a, &f.image((1000, 500), 16).unwrap()));
        let rendered = f.rendered.borrow();
        let row_pixels = rendered.as_ref().unwrap().1.as_ref().unwrap()[0]
            .0
            .pixels()
            .as_ptr();
        drop(rendered);
        assert!(f.image((10, 10), 16).is_none());
        assert!(f.image((900, 500), 16).is_some());
        assert_eq!(
            row_pixels,
            f.rendered.borrow().as_ref().unwrap().1.as_ref().unwrap()[0]
                .0
                .pixels()
                .as_ptr()
        );
        assert!(!Arc::ptr_eq(&a, &f.image((1000, 500), 24).unwrap()));
    }
}
