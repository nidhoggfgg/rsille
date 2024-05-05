use crate::{
    term::get_terminal_size,
    utils::{get_pos, to_rsille_err, RsilleErr},
    Paint,
};

use crossterm::style::Color;
use image::{
    imageops::FilterType::Lanczos3, io::Reader as ImageReader, DynamicImage, GenericImageView,
};

/// Paint the image on canvas
///
/// ## Example
///
/// paint the image
/// ```no_run
/// use rsille::{extra::Imgille, Canvas};
/// let path = "path/to/image";
/// let mut canvas = Canvas::new();
/// let imgille = Imgille::new(path).unwrap();
/// canvas.paint(&imgille, 0, 0).unwrap();
/// canvas.print();
/// ```
///
/// ## NOTE
///
/// You can always paint on *(0, 0)*.
/// But if you want, you can move it to other place on the canvas.
///
/// If your image isn't colorful (like grayscale image), you better set the color to `false`.
/// And give a look at [`thresholds`](#method.thresholds), [`invert`](#method.invert).

#[derive(Debug, Clone)]
pub struct Imgille {
    img: DynamicImage,
    color: bool,
    thresholds: u8,
    invert: bool,
}

impl Imgille {
    /// Construct a new object contains the picture
    ///
    /// Return `err` when can't open the image or can't decode the image
    pub fn new(path: &str) -> Result<Self, RsilleErr> {
        let err = Err(RsilleErr::new(format!("can't open image: {}", path)));
        let img = if let Ok(reader) = ImageReader::open(path) {
            if let Ok(img) = reader.decode() {
                img
            } else {
                return err;
            }
        } else {
            return err;
        };
        Ok(Self {
            img,
            color: true,
            thresholds: 128,
            invert: false,
        })
    }

    /// Open a image file
    pub fn open(&mut self, path: &str) -> Result<(), RsilleErr> {
        let img = ImageReader::open(path)
            .map_err(to_rsille_err)?
            .decode()
            .map_err(to_rsille_err)?;
        self.img = img;
        Ok(())
    }

    /// Set if the image should be painted with color or not
    ///
    /// The default is `true`, but for grayscale image, you better set it to `false`
    pub fn color(&mut self, color: bool) {
        self.color = color;
    }

    /// Set the thresholds for the image
    ///
    /// It's only for the grayscale image, and when the color is `false`.
    /// The default is *128*, but you can set it to other value
    pub fn thresholds(&mut self, thresholds: u8) {
        self.thresholds = thresholds;
    }

    /// If if invert the color of the image
    ///
    /// black to white, white to black.
    /// It's only for the grayscale image, and when the color is `false`.
    pub fn invert(&mut self, invert: bool) {
        self.invert = invert;
    }
}

impl Paint for Imgille {
    fn paint<T>(&self, canvas: &mut crate::Canvas, x: T, y: T)
    where
        T: Into<f64>,
    {
        // some example for resize the image (terminal size is 80*24):
        // 800*240 -> 160*48 (fit the width)
        // 800*120 -> 160*48 (fit the width)
        // 800*480 -> 160*96 (fit the height)
        // 800*960 -> 80*96  (fit the height)
        //  img_height      height
        // ------------ = ---------- and height = 4 * rest_height, width = 2 * rest_width
        //  img_width       width
        //
        // important: never optimize with the fill, it's really hard to use and the algo is really complex
        // can't stand use the fill anymore, even it would be much faster

        // calculate the rest size of the terminal
        let (x, y) = (x.into(), y.into());
        let (rest_width, rest_height) = get_rest_size(x, y);
        let (img_width, img_height) = (self.img.width(), self.img.height());

        // check the image is bigger than the terminal or not
        let img = if img_width > rest_width * 2 || img_height > rest_height * 4 {
            // first, try to resize the image to fit the height of the terminal
            let f = rest_height as f64 * 4.0 / img_height as f64;
            let w = (img_width as f64 * f) as u32;
            if rest_width * 2 < w {
                // if the width of the image is still bigger than the terminal width
                // then resize the image to fit the width of the terminal
                let f = rest_width as f64 * 2.0 / img_width as f64;
                let h = (img_height as f64 * f) as u32;
                self.img.resize_exact(rest_width * 2, h, Lanczos3)
            } else {
                self.img.resize_exact(w, rest_height * 4, Lanczos3)
            }
        } else {
            // the image is so small, no need to resize it
            self.img.clone()
        };

        if !self.color {
            // no color
            let img = img.grayscale();
            let img = img.as_luma8().unwrap();
            let (iw, ih) = (img.width(), img.height());
            for ny in 0..ih {
                for nx in 0..iw {
                    let pixel = img.get_pixel(nx, ny);
                    let if_draw = if self.invert {
                        (pixel.0)[0] > self.thresholds
                    } else {
                        (pixel.0)[0] < self.thresholds
                    };
                    if if_draw {
                        canvas.set(x + nx as f64, y + (ih - ny) as f64);
                    }
                }
            }
        } else {
            // color
            let (iw, ih) = (img.width(), img.height());
            for ny in 0..ih {
                for nx in 0..iw {
                    let pixel = img.get_pixel(nx, ny);
                    canvas.set_colorful(
                        x + nx as f64,
                        y + (ih - ny) as f64,
                        Color::Rgb {
                            r: pixel[0],
                            g: pixel[1],
                            b: pixel[2],
                        },
                    );
                }
            }
        };
    }
}

fn get_rest_size<T>(x: T, y: T) -> (u32, u32)
where
    T: Into<f64>,
{
    // calculate the rest size of the terminal
    let (tw, th) = get_terminal_size();
    let (start_col, start_row) = get_pos(x, y);
    let rest_width = if start_col > 0 {
        tw as u32 - start_col as u32
    } else {
        tw as u32
    };
    let rest_height = if start_row > 0 {
        th as u32 - start_row as u32
    } else {
        th as u32
    };
    (rest_width, rest_height)
}
