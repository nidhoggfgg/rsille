use async_trait::async_trait;

use crate::{
    attr::{Attr, AttrDisplay},
    slot::Slot,
    style::Stylized,
    traits::Draw,
    DrawErr, DrawUpdate, Update,
};

pub struct Panel {
    size: (u32, u32),
    boxes: Vec<Slot>,
}

impl Panel {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            size: (width, height),
            boxes: Vec::new(),
        }
    }

    pub fn push<T>(&mut self, thing: T, attr: Attr)
    where
        T: DrawUpdate + 'static,
    {
        self.boxes.push(Slot {
            attr,
            thing: Box::new(thing),
        });
    }
}

impl Draw for Panel {
    fn draw(&self) -> Vec<Stylized> {
        let mut result = vec![Stylized::space(); (self.size.0 * self.size.1) as usize];

        let (mut offset_col, mut offset_row) = (0_usize, 0_usize);
        for b in &self.boxes {
            let (pos_col, pos_row) = (offset_col, offset_row);

            if b.attr.float {
                todo!()
            }
            let data = b.draw();
            let (width, height) = b.size();

            // 获取可渲染区域
            let real_width = if pos_col as u32 + width > self.size.0 {
                self.size.0 - pos_col as u32
            } else {
                width
            };
            let real_height = if pos_row as u32 + height > self.size.1 {
                self.size.1 - pos_row as u32
            } else {
                height
            };

            let mut tmp_offset_row = offset_row;
            for i in 0..real_height {
                let start = (i * width) as usize;
                let end = start + real_width as usize;
                let line = &data[start..end];

                let r_start = tmp_offset_row * self.size.0 as usize + offset_col;
                let r_end = r_start + real_width as usize;
                result[r_start..r_end].clone_from_slice(line);
                tmp_offset_row += 1;
            }

            match b.attr.display {
                AttrDisplay::Block => {
                    offset_col = 0;
                    offset_row += real_height as usize;
                }
                AttrDisplay::Inline => todo!(),
            }
        }
        result
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }
}

#[async_trait]
impl Update for Panel {
    async fn update(&mut self) -> Result<bool, DrawErr> {
        let mut changed = false;
        for b in self.boxes.iter_mut() {
            match b.update().await {
                Ok(true) => changed = true,
                Ok(false) => (),
                Err(_) => return Err(DrawErr),
            }
        }
        Ok(changed)
    }
}
