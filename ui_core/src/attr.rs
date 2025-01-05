#[derive(Clone, Debug)]
pub struct Attr {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub display: AttrDisplay,
    pub float: bool,
}

#[derive(Debug, Clone)]
pub enum AttrDisplay {
    Block,
    Inline,
}
