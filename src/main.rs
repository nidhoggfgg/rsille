use canvas::Canvas;
use ui::{
    attr::{Attr, AttrDisplay},
    interactive::Interactive,
    panel::Panel,
    reactive::Reactive,
    runtime::Runtime,
};

#[tokio::main]
async fn main() {
    let mut panel = Panel::new(200, 50);

    let canvas = Canvas::new();

    let mut interactive_canvas = Interactive::new(canvas);
    interactive_canvas.register_mouse_event(|canvas, mouse| {
        canvas.clear();
        let (row, column) = (mouse.row, mouse.column);
        let v = (row * column) as f64;
        for p in 0..50 {
            let p = p as f64;
            for i in 0..7200 {
                let x = i as f64;
                let y = 15.0 + p * 2.0 + (x + v).to_radians().sin() * 50.0;
                canvas.set_f64(x / 10.0, y);
            }
        }
    });

    let canvas = Canvas::new();

    let mut reactive_canvas = Reactive::new(canvas);
    let tx = reactive_canvas.watch(0.0, |canvas, v| {
        canvas.clear();
        for i in 0..7200 {
            let x = i as f64;
            let y = 15.0 + (x + v).to_radians().sin() * 50.0;
            canvas.set_f64(x / 10.0, y);
        }
    });

    let rc2 = reactive_canvas.clone();
    let rc3 = reactive_canvas.clone();
    panel.push(
        interactive_canvas,
        Attr {
            id: "canvas".to_string(),
            width: 50,
            height: 50,
            display: AttrDisplay::Inline,
            float: false,
        },
        false,
    );
    panel.push(
        reactive_canvas,
        Attr {
            id: "reactive_canvas".to_string(),
            width: 50,
            height: 50,
            display: AttrDisplay::Inline,
            float: false,
        },
        false,
    );
    panel.push(
        rc2,
        Attr {
            id: "rc2".to_string(),
            width: 50,
            height: 50,
            display: AttrDisplay::Inline,
            float: false,
        },
        false,
    );
    panel.push(
        rc3,
        Attr {
            id: "rc3".to_string(),
            width: 50,
            height: 50,
            display: AttrDisplay::Inline,
            float: false,
        },
        false,
    );

    tokio::spawn(async move {
        for i in 0..7200 {
            tx.send(i as f64).unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    });

    let mut runtime = Runtime::new(panel);
    runtime.set_max_event_per_frame(3);

    runtime.run();
}
