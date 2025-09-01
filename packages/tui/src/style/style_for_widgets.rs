use crate::style::{ElementSize, Style, StyleDisplay};

impl Style {
    pub fn p() -> Self {
        Self {
            display: StyleDisplay::Block,
            width: ElementSize::Auto,
            height: ElementSize::Auto,
            float: false,
            ..Default::default()
        }
    }

    pub fn div() -> Self {
        Self {
            display: StyleDisplay::Block,
            width: ElementSize::Auto,
            height: ElementSize::Auto,
            float: false,
            ..Default::default()
        }
    }
}
