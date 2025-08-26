use std::io::stdout;

use rsille::{
    render,
    tui::{
        layout::grid::Grid,
        widgets::{Boxed, Text},
    },
};

fn main() {
    let widgets: [[Box<dyn rsille::tui::Widget + Send + Sync>; 4]; 3] = [
        [
            Box::new(Boxed::new(Text::new("aa\naa"))),
            Box::new(Boxed::new(Text::new("bb\nbb"))),
            Box::new(Text::new("cc\ncc")),
            Box::new(Text::new("dd\ndd")),
        ],
        [
            Box::new(Text::new("aa\naa")),
            Box::new(Text::new("bb\nbb")),
            Box::new(Text::new("cc\ncc")),
            Box::new(Text::new("dd\ndd")),
        ],
        [
            Box::new(Text::new("aa\naa")),
            Box::new(Text::new("bb\nbb")),
            Box::new(Text::new("cc\ncc")),
            Box::new(Text::new("dd\ndd")),
        ],
    ];
    let grid = Grid::<4, 3>::with_simple_border(widgets);
    let mut render = render::Builder::new()
        .size((30, 15))
        .build_render(grid, stdout());

    render.render().unwrap();
}
