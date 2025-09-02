use crate::{
    DrawErr,
    area::{Position, Size},
    style::Stylized,
};

#[derive(Debug, Clone, Default)]
pub struct Buffer {
    size: Size,
    content: Vec<Cell>,
}

// every position in the buffer is absolute
impl Buffer {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            content: vec![Cell::space(); (size.width * size.height) as usize],
        }
    }

    pub fn index(&self, pos: Position) -> Option<usize> {
        if self.size.less_any((pos.x, pos.y).into()) {
            return None;
        }

        Some((pos.y * self.size.width + pos.x) as usize)
    }

    pub fn is_occupied(&self, pos: Position) -> bool {
        let i = self.index_unchecked(pos);
        if i < self.content.len() {
            self.content[i].is_occupied
        } else {
            false
        }
    }

    pub fn get(&self, pos: Position) -> Option<&Stylized> {
        let i = self.index_unchecked(pos);
        if i < self.content.len() {
            if self.content[i].is_occupied {
                None
            } else {
                Some(&self.content[i].content)
            }
        } else {
            None
        }
    }

    pub fn set(&mut self, pos: Position, content: Stylized) -> Result<usize, DrawErr> {
        let i = self.index_unchecked(pos);
        if i >= self.content.len() {
            return Err(DrawErr);
        }

        if self.content[i].is_occupied {
            return Err(DrawErr);
        }

        self.set_forced(pos, content)
    }

    pub fn set_forced(&mut self, pos: Position, content: Stylized) -> Result<usize, DrawErr> {
        let i = self.index_unchecked(pos);
        let width = content.width();
        if i + width > self.content.len() || i == self.content.len() {
            return Err(DrawErr);
        }

        if self.content[i].is_occupied
            && let Some(owner) = self.content[i].owner
        {
            for j in owner..i {
                self.content[j] = Cell::space();
            }
        }

        self.content[i] = Cell::new(content);
        for j in i + 1..i + width {
            self.content[j].is_occupied = true;
            self.content[j].owner = Some(i);
        }
        Ok(width)
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn content(&self) -> &[Cell] {
        &self.content
    }

    fn index_unchecked(&self, pos: Position) -> usize {
        (pos.y * self.size.width + pos.x) as usize
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cell {
    pub content: Stylized,
    pub is_occupied: bool,
    pub owner: Option<usize>,
}

impl Cell {
    pub fn raw(c: char) -> Self {
        Self {
            content: Stylized::raw(c),
            is_occupied: false,
            owner: None,
        }
    }

    pub fn new(content: Stylized) -> Self {
        Self {
            content,
            is_occupied: false,
            owner: None,
        }
    }

    pub fn space() -> Self {
        Self {
            content: Stylized::space(),
            is_occupied: false,
            owner: None,
        }
    }

    pub fn width(&self) -> usize {
        self.content.width()
    }

    pub fn width_cjk(&self) -> usize {
        self.content.width_cjk()
    }

    pub fn queue(&self, buffer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.content.queue(buffer)
    }

    pub fn has_color(&self) -> bool {
        self.content.has_color()
    }

    pub fn has_attr(&self) -> bool {
        self.content.has_attr()
    }
}
