#[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct Attr {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub display: AttrDisplay,
    pub float: bool,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Copy)]
pub enum AttrDisplay {
    Block,
    Inline,
}
