use crate::{term::get_terminal_size, utils::RsilleErr, Paint};

use image::{
    imageops::FilterType::Lanczos3, io::Reader as ImageReader, DynamicImage, GenericImageView,
};

#[derive(Debug, Clone)]
pub struct Imgille {
    img: DynamicImage,
}

impl Imgille {
    pub fn new(path: &str) -> Result<Self, RsilleErr> {
        let err = Err(RsilleErr::new(format!("can't open image: {}", path)));
        let img = if let Ok(reader) = ImageReader::open(&path) {
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
        let (tw, th) = get_terminal_size()?;
        // 实际等效大小 width * 2, height * 4
        let (tw, th) = ((tw - x as usize) as u32 * 2, (th - y as usize) as u32 * 4);
        let (iw, ih) = (self.img.width(), self.img.height());
        let img = if iw > tw || ih > th {
            // 终端容不下不缩放的图片，先缩放图片
            let r1 = th as f32 / tw as f32;
            let r2 = self.img.height() as f32 / self.img.width() as f32;
            // 按照长或者宽缩放，避免超出终端的大小
            if r1 > r2 {
                let f = tw as f32 / self.img.width() as f32;
                let h = (self.img.height() as f32 * f) as u32;
                self.img.resize(tw, h, Lanczos3)
            } else {
                let f = th as f32 / self.img.height() as f32;
                let w = (self.img.width() as f32 * f) as u32;
                self.img.resize(w, th, Lanczos3)
            }
        } else {
            // 图片较小，可以直接容下
            self.img.clone()
        };
        let (iw, ih) = (img.width(), img.height());
        for ny in 0..ih {
            for nx in 0..iw {
                #[cfg(not(feature = "color"))]
                canvas.set(x + nx as f64, y + ny as f64);
                #[cfg(feature = "color")]
                {
                    use crate::color::TermColor;
                    let pixel = img.get_pixel(nx, ny);
                    canvas.set_colorful(
                        x + nx as f64,
                        y + ny as f64,
                        TermColor::Crgb(pixel[0], pixel[1], pixel[2]),
                    );
                }
            }
        }
        Ok(())
    }
}
