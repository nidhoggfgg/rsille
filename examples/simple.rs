use std::{cell::RefCell, io::stdout, rc::Rc};

use rsille::{
    render,
    term::crossterm::style::Color,
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

    let mut sub2 = Element::p("sub2".to_string());
    sub2.children.push(Node::Text("line3".to_string()));
    root.children
        .push(Node::Element(Rc::new(RefCell::new(sub2))));

    doc.set_root(root);
    let r = doc.get_element_by_id("sub").unwrap();
    r.upgrade().unwrap().borrow_mut().style.color = Some(Color::Cyan);

    let mut render = render::Builder::default()
        .size((20, 20))
        .clear(false)
        .append_newline(true)
        .build_render(doc, stdout());

    render.render().unwrap();
}
