pub enum SetAttr {
    Id(String),
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct Attr {
    pub id: String,
    pub width: ElementSize,
    pub height: ElementSize,
    pub display: AttrDisplay,
    pub float: bool,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Copy, Default)]
pub enum ElementSize {
    #[default]
    Auto,
    Fixed(u16),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Copy, Default)]
pub enum AttrDisplay {
    #[default]
    Block,
    Inline,
    Hidden,
}

impl Attr {
    pub fn set(&mut self, a: SetAttr) {
        use SetAttr::*;
        match a {
            Id(s) => self.id = s,
        }
    }
}
