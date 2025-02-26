#[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct Attr {
    pub id: String,
    pub width: u16,
    pub height: u16,
    pub display: AttrDisplay,
    pub float: bool,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Copy, Default)]
pub enum AttrDisplay {
    #[default]
    Block,
    Inline,
    Hidden,
}
