use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};
use term::crossterm::event::Event;

use crate::{
    attr::{Attr, AttrDisplay},
    style::Stylized,
    traits::Draw,
    DrawErr, DrawUpdate, Update,
};

use super::slot::Slot;

pub struct Panel {
    size: (u16, u16),
    boxes: Vec<Slot>,
    cache: Vec<Option<Vec<Stylized>>>,
}

impl Panel {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            size: (width, height),
            boxes: Vec::new(),
            cache: Vec::new(),
        }
    }

    pub fn push<T>(&mut self, thing: T, attr: Attr, enable_cache: bool)
    where
        T: DrawUpdate + Send + Sync + 'static,
    {
        self.boxes.push(Slot {
            attr,
            thing: Box::new(thing),
            updated: false,
            enable_cache,
        });
        self.cache.push(None);
    }

    pub fn refresh_cache(&mut self) -> Result<(), DrawErr> {
        let result = self
            .boxes
            .par_iter_mut()
            .map(|b| if b.enable_cache { Some(b.draw()) } else { None })
            .collect::<Vec<_>>();
        for (i, r) in result.into_iter().enumerate() {
            if let Some(Ok(d)) = r {
                self.cache[i] = Some(d);
            }
        }
        Ok(())
    }

    fn draw_impl(&self, boxes: &[&Vec<Stylized>]) -> Result<Vec<Stylized>, DrawErr> {
        let mut result = vec![Stylized::space(); (self.size.0 * self.size.1) as usize];
        let (panel_width, panel_height) = self.size;
        let (mut offset_col, mut offset_row) = (0, 0);
        let (mut end_offset_col, mut end_offset_row) = (0, 0);

        let mut state = RenderState::Block;

        for (i, b) in self.boxes.iter().enumerate() {
            if b.attr.float {
                todo!()
            }
            let data = boxes[i];
            // the size of the data, width * height = data.len()
            let (render_width, render_height) = b.size().ok_or(DrawErr)?;

            match state {
                RenderState::Block => {
                    offset_row = end_offset_row;
                    match b.attr.display {
                        AttrDisplay::Block => {
                            state = RenderState::Block;
                        }
                        AttrDisplay::Inline => {
                            state = RenderState::Inline;
                        }
                    }
                }
                RenderState::Inline => match b.attr.display {
                    AttrDisplay::Block => {
                        offset_col = 0;
                        offset_row = end_offset_row;
                        state = RenderState::Block;
                    }
                    AttrDisplay::Inline => {
                        offset_col = end_offset_col;
                        state = RenderState::Inline;
                    }
                },
            }

            let ((real_render_width, real_render_height), (real_box_width, real_box_height)) =
                calc_render_area(
                    (offset_col, offset_row),
                    (panel_width, panel_height),
                    (b.attr.width, b.attr.height),
                    (render_width, render_height),
                );

            // only render the real renderable area, not the whole box
            // other area in the box is already filled with space
            let mut tmp_offset_row = offset_row;
            for i in 0..real_render_height {
                let start = (i * render_width) as usize;
                let end = start + real_render_width as usize;
                let line = &data[start..end];

                let r_start = (tmp_offset_row * panel_width + offset_col) as usize;
                let r_end = r_start + real_render_width as usize;
                result[r_start..r_end].clone_from_slice(line);
                tmp_offset_row += 1;
            }

            end_offset_col = offset_col + real_box_width;
            end_offset_row = offset_row + real_box_height;
        }

        Ok(result)
    }
}

impl Draw for Panel {
    fn draw(&self) -> Result<Vec<Stylized>, DrawErr> {
        let not_cached = self
            .boxes
            .par_iter()
            .enumerate()
            .map(|(i, b)| {
                if self.cache[i].is_none() {
                    Some(b.draw())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut boxes = Vec::with_capacity(self.boxes.len());
        for i in 0..self.boxes.len() {
            if let Some(d) = self.cache[i].as_ref() {
                boxes.push(d);
            } else if let Some(Ok(d)) = not_cached[i].as_ref() {
                boxes.push(d);
            } else if let Some(Err(e)) = not_cached[i].as_ref() {
                return Err(*e);
            } else {
                // unreachable
                continue;
            }
        }

        if boxes.len() != self.boxes.len() {
            return Err(DrawErr);
        }

        self.draw_impl(&boxes)
    }

    #[must_use]
    #[inline]
    fn size(&self) -> Option<(u16, u16)> {
        Some(self.size)
    }
}

impl Update for Panel {
    fn update(&mut self, events: &[Event]) -> Result<bool, DrawErr> {
        let changes = self
            .boxes
            .par_iter_mut()
            .map(|b| b.update(events))
            .collect::<Result<Vec<_>, DrawErr>>()?;

        let changed = changes.iter().any(|x| *x);

        Ok(changed)
    }
}

fn calc_render_area(
    offset: (u16, u16),
    panel_size: (u16, u16),
    box_size: (u16, u16),
    render_size: (u16, u16),
) -> ((u16, u16), (u16, u16)) {
    // already out of panel
    if offset.0 >= panel_size.0 || offset.1 >= panel_size.1 {
        return ((0, 0), (0, 0));
    }

    let free_width = panel_size.0 - offset.0;
    let free_height = panel_size.1 - offset.1;

    // the real renderable area is the smallest of free, box, and render
    let real_render_width = free_width.min(box_size.0.min(render_size.0));
    let real_render_height = free_height.min(box_size.1.min(render_size.1));

    // the real box area is the smallest of free and box
    let real_box_width = free_width.min(box_size.0);
    let real_box_height = free_height.min(box_size.1);

    (
        (real_render_width, real_render_height),
        (real_box_width, real_box_height),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
enum RenderState {
    Block,
    Inline,
}
