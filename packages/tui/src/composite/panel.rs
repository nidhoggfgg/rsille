use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use render::{style::Stylized, Draw, DrawChunk, DrawErr, Update};
use term::event::Event;

use crate::{
    attr::{Attr, AttrDisplay},
    Widget,
};

use super::slot::Slot;

pub struct Panel {
    pub boxes: Vec<Slot>,
    pub size: (u16, u16),

    draw_areas: Vec<(u16, u16)>,
    cache: Vec<Option<DrawChunk>>,
}

impl Panel {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            size: (width, height),
            boxes: Vec::new(),
            draw_areas: Vec::new(),
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
        let cache = self
            .boxes
            .par_iter_mut()
            .enumerate()
            .map(|(i, b)| {
                if b.updated || self.cache[i].is_none() {
                    (i, Some(b.draw()))
                } else {
                    (i, None)
                }
            })
            .collect::<Vec<_>>();
        for (i, d) in cache {
            if let Some(d) = d {
                let d = d?;
                self.cache[i] = Some(d);
            }
        }
        Ok(())
    }
}

impl Draw for Panel {
    fn draw(&mut self) -> Result<render::DrawChunk, render::DrawErr> {
        let mut boxes = Vec::new();
        for (i, cache) in self.cache.iter().enumerate() {
            if let Some(cache) = cache {
                boxes.push((cache, self.boxes[i].get_attr()));
            }
        }
        let (chunk, areas) = draw_boxes(&boxes, self.size)?;
        self.draw_areas = areas;
        Ok(chunk)
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

// TODO: there are so many bugs
fn draw_boxes(
    boxes: &[(&DrawChunk, &Attr)],
    size: (u16, u16),
) -> Result<(DrawChunk, Vec<(u16, u16)>), DrawErr> {
    let (max_width, max_height) = size;
    let mut result = vec![Stylized::space(); (max_width * max_height) as usize];
    let (mut offset_col, mut offset_row) = (0, 0);
    let (mut end_offset_col, mut end_offset_row) = (0, 0);
    let mut state = RenderState::Block;
    let areas = Vec::new();

    for (chunk, attr) in boxes {
        let data = &chunk.0;
        let width = chunk.1 as u16;
        let height = (data.len() / chunk.1) as u16;

        if attr.float {
            todo!()
        }

        state = match state {
            RenderState::Block => {
                offset_row = end_offset_row;
                match attr.display {
                    AttrDisplay::Block => RenderState::Block,
                    AttrDisplay::Inline => RenderState::Inline,
                    AttrDisplay::Hidden => state,
                }
            }
            RenderState::Inline => match attr.display {
                AttrDisplay::Block => {
                    offset_col = 0;
                    offset_row = end_offset_row;
                    RenderState::Block
                }
                AttrDisplay::Inline => {
                    offset_col = end_offset_col;
                    RenderState::Inline
                }
                AttrDisplay::Hidden => state,
            },
        };

        let ((real_render_width, real_render_height), (real_box_width, real_box_height)) =
            calc_render_area(
                (offset_col, offset_row),
                (max_width, max_height),
                (attr.width, attr.height),
                (width, height),
            );

        let mut now_offset_row = offset_row;
        for i in 0..real_render_height {
            let start = (i * width) as usize;
            let end = start + real_render_width as usize;
            let line = &data[start..end];

            let r_start = (now_offset_row * max_width + offset_col) as usize;
            let r_end = r_start + real_render_width as usize;
            result[r_start..r_end].clone_from_slice(line);
            now_offset_row += 1;
        }

        end_offset_col = offset_col + real_box_width;
        end_offset_row = offset_row + real_box_height;
    }

    Ok((DrawChunk(result, max_width as usize), areas))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
enum RenderState {
    Block,
    Inline,
}

fn calc_render_area(
    offset: (u16, u16),
    max_size: (u16, u16),
    box_size: (u16, u16),
    render_size: (u16, u16),
) -> ((u16, u16), (u16, u16)) {
    // already out of max size
    if offset.0 >= max_size.0 || offset.1 >= max_size.1 {
        return ((0, 0), (0, 0));
    }

    let free_width = max_size.0 - offset.0;
    let free_height = max_size.1 - offset.1;

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
