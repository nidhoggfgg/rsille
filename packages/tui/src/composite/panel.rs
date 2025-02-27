use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use render::{Draw, DrawErr, Update};
use term::{event::Event, style::Stylized};

use crate::{
    attr::{Attr, AttrDisplay},
    widgets::Widget,
};

use super::slot::Slot;

pub struct Panel {
    pub boxes: Vec<Slot>,
    pub size: (u16, u16),
    // internal cache
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

    pub fn push<T>(&mut self, thing: T)
    where
        T: Widget + Send + Sync + 'static,
    {
        let slot = Slot {
            thing: Box::new(thing),
            updated: false,
        };
        self.boxes.push(slot);
        self.cache.push(None);
    }

    pub fn refresh_cache(&mut self) -> Result<(), DrawErr> {
        let result = self
            .boxes
            .par_iter_mut()
            .enumerate()
            .map(|(i, b)| {
                if b.updated || self.cache[i].is_none() {
                    Some(b.draw())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for (_i, r) in result.into_iter().enumerate() {
            if let Some(Ok(_d)) = r {
                // self.cache[i] = Some(d);
                todo!()
            }
        }
        Ok(())
    }
}

impl Draw for Panel {
    fn draw(&mut self) -> Result<render::DrawChunk, render::DrawErr> {
        todo!()
    }
}

impl Update for Panel {
    fn on_events(&mut self, _events: &[Event]) -> Result<(), DrawErr> {
        // dispatch event
        // todo!()
        Ok(())
    }

    fn update(&mut self) -> Result<bool, DrawErr> {
        let changes = self
            .boxes
            .par_iter_mut()
            .map(|b| b.update())
            .collect::<Result<Vec<_>, DrawErr>>()?;

        let changed = changes.iter().any(|x| *x);

        Ok(changed)
    }
}

#[allow(unused)]
pub fn draw_boxes(
    boxes: &[(&Vec<Stylized>, (u16, u16), &Attr)],
    size: (u16, u16),
) -> Result<Vec<Stylized>, DrawErr> {
    let (panel_width, panel_height) = size;
    let mut result = vec![Stylized::space(); (panel_width * panel_height) as usize];
    let (mut offset_col, mut offset_row) = (0, 0);
    let (mut end_offset_col, mut end_offset_row) = (0, 0);

    let mut state = RenderState::Block;

    for (data, render_size, attr) in boxes.iter() {
        if attr.float {
            todo!()
        }
        // the size of the data, width * height = data.len()
        let (render_width, render_height) = render_size.to_owned();

        match state {
            RenderState::Block => {
                offset_row = end_offset_row;
                match attr.display {
                    AttrDisplay::Block => {
                        state = RenderState::Block;
                    }
                    AttrDisplay::Inline => {
                        state = RenderState::Inline;
                    }
                    AttrDisplay::Hidden => {}
                }
            }
            RenderState::Inline => match attr.display {
                AttrDisplay::Block => {
                    offset_col = 0;
                    offset_row = end_offset_row;
                    state = RenderState::Block;
                }
                AttrDisplay::Inline => {
                    offset_col = end_offset_col;
                    state = RenderState::Inline;
                }
                AttrDisplay::Hidden => {}
            },
        }

        let ((real_render_width, real_render_height), (real_box_width, real_box_height)) =
            calc_render_area(
                (offset_col, offset_row),
                (panel_width, panel_height),
                (attr.width, attr.height),
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

pub fn calc_render_area(
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
