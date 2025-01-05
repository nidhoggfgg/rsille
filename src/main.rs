use canvas::Canvas;
use terminal::{
    attr::{Attr, AttrDisplay},
    panel::Panel,
    reactive::Reactive,
    terminal::Terminal,
};
use tokio::sync::watch;

#[tokio::main]
pub async fn main() {
    let mut panel = Panel::new(200, 30);

    let canvas = Canvas::new();

    let (tx, rx) = watch::channel(0);

    let reactive_canvas = Reactive::new(canvas).watch(rx, |canvas, v| {
        canvas.clear();

        for x in 0..1800 {
            let x = x as f64 + *v as f64;
            canvas.set(x / 10.0, 15.0 + x.to_radians().sin() * 10.0);
        }
    });

    panel.push(
        reactive_canvas,
        Attr {
            id: "canvas".into(),
            width: 200,
            height: 30,
            display: AttrDisplay::Block,
            float: false,
        },
    );

    let mut terminal = Terminal::new(panel);

    let sender_task = tokio::spawn(async move {
        for i in 0..1800 {
            tx.send(i).unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
    });

    let terminal_task = tokio::spawn(async move {
        terminal.run().await.unwrap();
    });

    let v = tokio::join!(sender_task, terminal_task);
    println!("{:?}", v);
}
