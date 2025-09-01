use std::{cell::RefCell, rc::Rc};

use render::style::StylizedLine;
use render::{area::Size, chunk::Chunk, Draw, DrawErr};

use crate::style::StyleDisplay;
use crate::{Document, Element};

#[derive(Debug, Clone)]
pub enum Node {
    Element(Rc<RefCell<Element>>),

    // text
    Text(String),
}

impl Node {
    pub fn register(&self, doc: &mut Document) {
        if let Node::Element(element) = self {
            doc.register(element.borrow().id.clone(), element.clone());
            element.borrow().register(doc);
        }
    }

    pub fn display(&self) -> StyleDisplay {
        match self {
            Node::Element(element) => element.borrow().style.display,
            Node::Text(_) => StyleDisplay::Inline,
        }
    }
}

impl Draw for Node {
    fn draw(&mut self, mut chunk: Chunk) -> Result<Size, DrawErr> {
        match self {
            Node::Element(element) => element.borrow_mut().draw(chunk),
            Node::Text(text) => {
                let lines: Vec<StylizedLine> = text.lines().map(StylizedLine::from).collect();
                let mut max_width = 0u16;
                let mut height = 0u16;
                for (y, line) in lines.iter().enumerate() {
                    let y = y as u16;
                    if y >= chunk.area().size().height {
                        break;
                    }
                    let mut real_x = 0u16;
                    for c in line.content.iter().flat_map(|t| t.into_iter()) {
                        if real_x >= chunk.area().size().width {
                            break;
                        }
                        if let Ok(l) = chunk.set(real_x, y, c) {
                            real_x += l as u16;
                        } else {
                            break;
                        }
                    }
                    if real_x > max_width {
                        max_width = real_x;
                    }
                    height = y + 1;
                }
                Ok((max_width, height).into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use render::{
        area::{Area, Position, Size},
        buffer::{Buffer, Cell},
        chunk::Chunk,
    };

    #[test]
    fn test_text_single_line() {
        let mut node = Node::Text("hello".to_string());
        let mut buffer = Buffer::new(Size {
            width: 10,
            height: 1,
        });
        let area = Area::new(
            Position::default(),
            Size {
                width: 10,
                height: 1,
            },
        );
        let chunk = Chunk::new(&mut buffer, area).unwrap();
        let size = node.draw(chunk).unwrap();
        assert_eq!(size.width, 5);
        assert_eq!(size.height, 1);

        let content = buffer.content();
        assert_eq!(content.len(), 10);
        assert_eq!(
            content,
            vec![
                Cell::raw('h'),
                Cell::raw('e'),
                Cell::raw('l'),
                Cell::raw('l'),
                Cell::raw('o'),
                Cell::space(),
                Cell::space(),
                Cell::space(),
                Cell::space(),
                Cell::space(),
            ]
        );
    }

    #[test]
    fn test_text_multi_line() {
        let mut node = Node::Text("hello\nworld".to_string());
        let mut buffer = Buffer::new(Size {
            width: 7,
            height: 2,
        });
        let area = Area::new(
            Position::default(),
            Size {
                width: 7,
                height: 2,
            },
        );
        let chunk = Chunk::new(&mut buffer, area).unwrap();
        let size = node.draw(chunk).unwrap();
        assert_eq!(size.width, 5);
        assert_eq!(size.height, 2);

        let content = buffer.content();
        assert_eq!(content.len(), 14);
        assert_eq!(
            content,
            vec![
                Cell::raw('h'),
                Cell::raw('e'),
                Cell::raw('l'),
                Cell::raw('l'),
                Cell::raw('o'),
                Cell::space(),
                Cell::space(),
                Cell::raw('w'),
                Cell::raw('o'),
                Cell::raw('r'),
                Cell::raw('l'),
                Cell::raw('d'),
                Cell::space(),
                Cell::space(),
            ]
        );
    }

    #[test]
    fn test_text_truncation_width() {
        let mut node = Node::Text("helloworld".to_string());
        let mut buffer = Buffer::new(Size {
            width: 5,
            height: 1,
        });
        let area = Area::new(
            Position::default(),
            Size {
                width: 5,
                height: 1,
            },
        );
        let chunk = Chunk::new(&mut buffer, area).unwrap();
        let size = node.draw(chunk).unwrap();
        assert_eq!(size.width, 5);
        assert_eq!(size.height, 1);

        let content = buffer.content();
        assert_eq!(content.len(), 5);
        assert_eq!(
            content,
            vec![
                Cell::raw('h'),
                Cell::raw('e'),
                Cell::raw('l'),
                Cell::raw('l'),
                Cell::raw('o'),
            ]
        );
    }

    #[test]
    fn test_text_truncation_height() {
        let mut node = Node::Text("hello\nworld\nfoo".to_string());
        let mut buffer = Buffer::new(Size {
            width: 5,
            height: 2,
        });
        let area = Area::new(
            Position::default(),
            Size {
                width: 5,
                height: 2,
            },
        );
        let chunk = Chunk::new(&mut buffer, area).unwrap();
        let size = node.draw(chunk).unwrap();
        assert_eq!(size.width, 5);
        assert_eq!(size.height, 2);

        let content = buffer.content();
        assert_eq!(content.len(), 10);
        assert_eq!(
            content,
            vec![
                Cell::raw('h'),
                Cell::raw('e'),
                Cell::raw('l'),
                Cell::raw('l'),
                Cell::raw('o'),
                Cell::raw('w'),
                Cell::raw('o'),
                Cell::raw('r'),
                Cell::raw('l'),
                Cell::raw('d'),
            ]
        );
    }

    #[test]
    fn test_empty_text() {
        let mut node = Node::Text("".to_string());
        let mut buffer = Buffer::new(Size {
            width: 3,
            height: 1,
        });
        let area = Area::new(
            Position::default(),
            Size {
                width: 3,
                height: 1,
            },
        );
        let chunk = Chunk::new(&mut buffer, area).unwrap();
        let size = node.draw(chunk).unwrap();
        assert_eq!(size.width, 0);
        assert_eq!(size.height, 0);

        let content = buffer.content();
        assert_eq!(content.len(), 3);
        assert_eq!(content, vec![Cell::space(), Cell::space(), Cell::space(),]);
    }
}
