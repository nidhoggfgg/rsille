//! Offline RGB previews using the same formula and graph renderers as the REPL.
use std::{error::Error, fs, path::Path};

use rsille::raster::{RasterCanvas, Rgb};

use crate::Plot;

const WIDTH: u32 = 480;
const GRAPH_HEIGHT: u32 = 240;
const GAP: u32 = 12;
const CELL_HEIGHT: u16 = 20;
const FRAMES: usize = 90;
const PRESETS: [&str; 4] = [
    "surface(sin(x)*cos(y), -pi, pi, -pi, pi)",
    "surface((x^2-y^2)/3, -2, 2, -2, 2)",
    "param3(cos(t), sin(t), t/4, 0, 6*pi)",
    "param3((2+cos(3*t))*cos(2*t), (2+cos(3*t))*sin(2*t), sin(3*t), 0, 2*pi)",
];

struct Preview {
    plots: Vec<Plot>,
    bounds: Vec<[f64; 4]>,
    formulas: Vec<std::sync::Arc<RasterCanvas>>,
    formula_height: u32,
}

impl Preview {
    fn new() -> Result<Self, Box<dyn Error>> {
        let plots = PRESETS
            .iter()
            .map(|source| Plot::parse(source))
            .collect::<Result<Vec<_>, _>>()?;
        let bounds = plots
            .iter()
            .map(|plot| plot.auto_bounds(crate::X_RANGE))
            .collect();
        let formula_height = plots
            .iter()
            .map(|plot| u32::from(plot.formula.height(CELL_HEIGHT)) * u32::from(CELL_HEIGHT))
            .max()
            .unwrap();
        let formulas = plots
            .iter()
            .map(|plot| {
                if u32::from(plot.formula.width((1, CELL_HEIGHT))) > WIDTH {
                    return Err("Preview formula exceeds panel width");
                }
                plot.formula
                    .image((WIDTH, formula_height), CELL_HEIGHT)
                    .ok_or("Preview formula rendering failed")
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            plots,
            bounds,
            formulas,
            formula_height,
        })
    }

    fn frame(&self, angle: f64) -> Result<RasterCanvas, Box<dyn Error>> {
        let panel_height = self.formula_height + GRAPH_HEIGHT;
        let mut canvas = RasterCanvas::new(
            WIDTH * 2 + GAP * 3,
            panel_height * 2 + GAP * 3,
            Rgb(18, 22, 30),
        )?;
        for (index, plot) in self.plots.iter().enumerate() {
            let x = GAP + (index as u32 % 2) * (WIDTH + GAP);
            let y = GAP + (index as u32 / 2) * (panel_height + GAP);
            paste(&mut canvas, &self.formulas[index], x, y);
            let graph = plot.image_at(self.bounds[index], (WIDTH, GRAPH_HEIGHT), angle);
            paste(&mut canvas, &graph, x, y + self.formula_height);
        }
        Ok(canvas)
    }
}

fn paste(target: &mut RasterCanvas, source: &RasterCanvas, x: u32, y: u32) {
    for row in 0..source.height() {
        for column in 0..source.width() {
            target.set_pixel(
                (x + column) as i32,
                (y + row) as i32,
                source.pixels()[(row * source.width() + column) as usize],
            );
        }
    }
}

fn ppm(canvas: &RasterCanvas) -> Vec<u8> {
    let mut bytes = format!("P6\n{} {}\n255\n", canvas.width(), canvas.height()).into_bytes();
    bytes.reserve(canvas.pixels().len() * 3);
    for &Rgb(r, g, b) in canvas.pixels() {
        bytes.extend_from_slice(&[r, g, b]);
    }
    bytes
}

pub fn command(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 2 {
        return Err("Usage: --frames DIRECTORY | --snapshot FILE.ppm".into());
    }
    let preview = Preview::new()?;
    match args[0].as_str() {
        "--frames" => {
            let directory = Path::new(&args[1]);
            // Refuse an existing directory so a typo cannot overwrite earlier frames.
            fs::create_dir_all(
                directory
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or(Path::new(".")),
            )?;
            fs::create_dir(directory)?;
            for index in 0..FRAMES {
                // Exclude the repeated endpoint for a seamless full-turn loop.
                let angle = std::f64::consts::TAU * index as f64 / FRAMES as f64;
                fs::write(
                    directory.join(format!("{index:04}.ppm")),
                    ppm(&preview.frame(angle)?),
                )?;
            }
            println!(
                "Saved {FRAMES} frames (6 seconds at 15 FPS) to {}",
                directory.display()
            );
        }
        "--snapshot" => {
            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&args[1])?;
            file.write_all(&ppm(&preview.frame(0.0)?))?;
            println!("Saved preview to {}", args[1]);
        }
        _ => return Err("Unknown export option".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ppm_preserves_rgb_and_dimensions() {
        let mut canvas = RasterCanvas::new(2, 1, Rgb(1, 2, 3)).unwrap();
        canvas.set_pixel(1, 0, Rgb(4, 5, 6));
        assert_eq!(ppm(&canvas), b"P6\n2 1\n255\n\x01\x02\x03\x04\x05\x06");
    }

    #[test]
    fn preview_rotates_and_returns_to_start() {
        let preview = Preview::new().unwrap();
        let start = preview.frame(0.0).unwrap();
        let rotated = preview.frame(1.0).unwrap();
        let end = preview.frame(std::f64::consts::TAU).unwrap();
        assert_eq!(start.width(), WIDTH * 2 + GAP * 3);
        assert_ne!(start.pixels(), rotated.pixels());
        // Floating-point rotation can move a few edge pixels at a full turn.
        let differences = start
            .pixels()
            .iter()
            .zip(end.pixels())
            .filter(|(a, b)| a != b)
            .count();
        assert!(differences < start.pixels().len() / 100);
    }
}
