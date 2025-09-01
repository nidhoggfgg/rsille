use render::{area::Size, chunk::Chunk, Draw, DrawErr};

use crate::style::Style;
use crate::style::StyleDisplay;
use crate::{node::Node, tag::Tag, Document};

#[derive(Debug, Clone)]
pub struct Element {
    pub id: String,
    pub tag_name: Tag,
    pub style: Style,

    pub children: Vec<Node>,
}

impl Element {
    pub fn new(tag_name: &str, id: String) -> Self {
        Self {
            id,
            tag_name: tag_name.into(),
            style: Style::default(),
            children: Vec::new(),
        }
    }

    pub fn p(id: String) -> Self {
        Self {
            id,
            tag_name: Tag::P,
            style: Style::default(),
            children: Vec::new(),
        }
    }

    pub fn register(&self, doc: &mut Document) {
        for child in self.children.iter() {
            child.register(doc);
        }
    }
}

impl Draw for Element {
    fn draw(&mut self, mut chunk: Chunk) -> Result<Size, DrawErr> {
        if self.style.display == StyleDisplay::Hidden {
            return Ok(Size::default());
        }
        let (mut total_width, mut total_height) = (0, 0);
        let (mut curr_x, mut curr_y) = (0, 0);
        let (mut line_height, mut line_width) = (0, 0);
        let mut last_display = StyleDisplay::Block;

        let available_size = chunk.area().size();

        for child in &mut self.children {
            let display = child.display();

            // display: block then move to new line
            if display == StyleDisplay::Block || last_display == StyleDisplay::Block {
                total_height += line_height;
                curr_y += line_height;
                curr_x = 0;
                line_height = 0;
                line_width = 0;
            }

            if curr_x > available_size.width {
                continue;
            }
            if curr_y > available_size.height {
                break;
            }

            // shrink the chunk to the right position
            let now_chunk = chunk.shrink(curr_y, 0, curr_x, 0)?;
            let size = child.draw(now_chunk)?;

            if size.height > line_height {
                line_height = size.height;
            }
            line_width += size.width;
            curr_x += size.width;

            if line_width > total_width {
                total_width = line_width;
            }
            last_display = display;
        }
        total_height += line_height;

        Ok((total_width, total_height).into())
    }
}
