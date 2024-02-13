use crate::{term::get_terminal_size, utils::RsilleErr, Paint};

use image::{
    imageops::FilterType::Lanczos3, io::Reader as ImageReader, DynamicImage, GenericImageView,
};

/// Paint the image on canvas
///
/// ## Example
///
/// paint the image
/// ```no_run
/// use rsille::{Canvas, Imgille};
/// let path = "path/to/image";
/// let mut canvas = Canvas::new();
/// let imgille = Imgille::new(path).unwrap();
/// canvas.paint(&imgille, 0.0, 0.0).unwrap();
/// println!("{}", canvas.frame());
/// ```
///
/// ## NOTE
///
/// you can always paint on (0.0, 0.0).
/// but if you want, you can move it to other place on the canvas!
///
/// when the image is big (like 3840*3840), please use release build

#[derive(Debug, Clone)]
pub struct Imgille {
    img: DynamicImage,
}

impl Imgille {
    /// construct a new object contains the picture
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
        Ok(Self { img })
    }
}

impl Paint for Imgille {
    fn paint(&self, canvas: &mut crate::Canvas, x: f64, y: f64) -> Result<(), RsilleErr> {
        // for a fully filled image the braille code equal to a █
        let (tw, th) = get_terminal_size()?;
        let (tw, th) = ((tw - x as usize) as u32, (th - y as usize) as u32);
        let (iw, ih) = (self.img.width(), self.img.height());
        let img = if iw > tw || ih > th {
            let f = th as f32 / self.img.height() as f32;
            let w = (self.img.width() as f32 * f) as u32 * 2;
            if tw < th || tw < w {
                let f = tw as f32 / self.img.width() as f32;
                let h = (self.img.height() as f32 * f / 2.0) as u32;
                self.img.resize_exact(tw, h, Lanczos3)
            } else {
                self.img.resize_exact(w, th, Lanczos3)
            }
        } else {
            self.img.clone()
        };
        let (iw, ih) = (img.width(), img.height());
        for ny in 0..ih {
            for nx in 0..iw {
                use crate::color::TermColor;
                let pixel = img.get_pixel(nx, ny);
                canvas.set_fill_colorful(
                    x + nx as f64,
                    y + ny as f64,
                    TermColor::Crgb(pixel[0], pixel[1], pixel[2]),
                );
            }
        }
        Ok(())

        // those code for not fully filled image
        // let (tw, th) = get_terminal_size()?;
        // // 实际等效大小 width * 2, height * 4
        // let (tw, th) = ((tw - x as usize) as u32 * 2, (th - y as usize) as u32 * 4);
        // let (iw, ih) = (self.img.width(), self.img.height());
        // let img = if iw > tw || ih > th {
        //     // 终端容不下不缩放的图片，先缩放图片
        //     let r1 = th as f32 / tw as f32;
        //     let r2 = self.img.height() as f32 / self.img.width() as f32;
        //     // 按照长或者宽缩放，避免超出终端的大小
        //     if r1 > r2 {
        //         let f = tw as f32 / self.img.width() as f32;
        //         let h = (self.img.height() as f32 * f) as u32;
        //         self.img.resize(tw, h, Lanczos3)
        //     } else {
        //         let f = th as f32 / self.img.height() as f32;
        //         let w = (self.img.width() as f32 * f) as u32;
        //         self.img.resize(w, th, Lanczos3)
        //     }
        // } else {
        //     // 图片较小，可以直接容下
        //     self.img.clone()
        // };
        // let (iw, ih) = (img.width(), img.height());
        // for ny in 0..ih {
        //     for nx in 0..iw {
        //         use crate::color::TermColor;
        //         let pixel = img.get_pixel(nx, ny);
        //         canvas.set_colorful(
        //             x + nx as f64,
        //             y + ny as f64,
        //             TermColor::Crgb(pixel[0], pixel[1], pixel[2]),
        //         );
        //     }
        // }
        // Ok(())
    }
}
