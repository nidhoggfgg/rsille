use canvas::Canvas;
use ui::{
    attr::{Attr, AttrDisplay},
    interactive::Interactive,
    panel::Panel,
    runtime::Runtime,
};

fn main() {
    let mut panel = Panel::new(100, 50);

    let canvas = Canvas::new();

    let mut interactive_canvas = Interactive::new(canvas);
    interactive_canvas.register_mouse_event(|canvas, mouse| {
        canvas.clear();
        let (row, column) = (mouse.row, mouse.column);
        let v = (row * column) as f64;
        for i in 0..7200 {
            let x = i as f64;
            let y = 15.0 + (x + v).to_radians().sin() * 50.0;
            canvas.set_f64(x / 10.0, y);
        }
    });

    panel.push(
        interactive_canvas,
        Attr {
            id: "canvas".to_string(),
            width: 100,
            height: 50,
            display: AttrDisplay::Block,
            float: false,
        },
    );

    let runtime = Runtime::new(panel);

    runtime.run();
}
