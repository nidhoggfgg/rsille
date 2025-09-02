use render::area::Size;
use render::chunk::Chunk;
use render::{Draw, DrawErr};

use crate::style::Style;
use crate::tag::Tag;
use crate::{node::Node, Element};

#[derive(Debug, Clone)]
pub struct Document {
    pub root: Option<Node>,
    pub elements: Vec<Element>,
}

impl Document {
    pub fn new() -> Self {
        Self {
            root: None,
            elements: Vec::new(),
        }
    }

    pub fn get_element_by_id(&self, id: &str) -> Option<&Element> {
        self.elements.iter().find(|e| e.id == id)
    }

    pub fn get_element_by_id_mut(&mut self, id: &str) -> Option<&mut Element> {
        self.elements.iter_mut().find(|e| e.id == id)
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Draw for Document {
    fn draw(&mut self, chunk: Chunk) -> Result<Size, DrawErr> {
        let root = self.root.as_ref().ok_or(DrawErr)?;
        root.draw_impl(chunk, self)
    }
}

impl Document {
    pub fn from_html(html: &str) -> Result<Self, DrawErr> {
        let dom = tl::parse(html, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();

        let mut tree = Document {
            elements: Vec::new(),
            root: None,
        };

        let root_index = tree.elements.len();
        tree.elements.push(Element {
            id: "".to_string(),
            tag_name: Tag::Html,
            style: Style::default(),
            children: Vec::new(),
        });
        tree.root = Some(Node::Element(root_index));

        dom.children().iter().for_each(|handle| {
            let node = handle.get(parser).unwrap();
            build_doc(node, parser, &mut tree, Some(root_index))
        });
        Ok(tree)
    }
}

fn build_doc(
    node: &tl::Node,
    parser: &tl::Parser,
    doc: &mut Document,
    parent: Option<usize>,
) {
    match node {
        tl::Node::Tag(tag) => {
            let id = tag
                .attributes()
                .get("id")
                .flatten()
                .map(|b| String::from_utf8_lossy(b.as_bytes()).to_string())
                .unwrap_or_default();
            let index = doc.elements.len();
            doc.elements.push(Element {
                id,
                tag_name: tag.name().into(),
                style: Style::default(),
                children: Vec::new(),
            });
            if let Some(parent) = parent {
                let p = &mut doc.elements[parent];
                p.children.push(Node::Element(index));
            }
            for child_handle in tag.children().top().iter() {
                let child_node = child_handle.get(parser).unwrap();
                build_doc(child_node, parser, doc, Some(index));
            }
        }
        tl::Node::Raw(bytes) => {
            let text = bytes.as_utf8_str().trim().to_string();
            if text.is_empty() {
                return;
            }
            if let Some(parent) = parent {
                let p = &mut doc.elements[parent];
                p.children.push(Node::Text(text));
            }
        }
        tl::Node::Comment(comment) => {
            if let Some(parent) = parent {
                let p = &mut doc.elements[parent];
                p.children
                    .push(Node::Comment(comment.as_utf8_str().to_string()));
            }
        }
    }
}
