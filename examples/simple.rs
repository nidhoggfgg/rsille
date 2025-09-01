use std::{cell::RefCell, io::stdout, rc::Rc};

use rsille::{
    render,
    tui::{node::Node, Document, Element},
};

fn main() {
    let mut doc = Document::new();
    let mut root = Element::p("root".to_string());
    root.children.push(Node::Text("line1 ".to_string()));
    root.children.push(Node::Text("still line1".to_string()));

    let mut sub = Element::p("sub".to_string());
    sub.children.push(Node::Text("line2".to_string()));
    root.children
        .push(Node::Element(Rc::new(RefCell::new(sub))));
    doc.set_root(root);

    let mut render = render::Builder::default()
        .size((20, 20))
        .clear(false)
        .append_newline(true)
        .build_render(doc, stdout());

    render.render().unwrap();
}
