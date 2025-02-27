use std::fs::File;
use std::io::Write;

use tui::composite::{Animative, Panel};
use tui::widgets::Text;

#[tokio::main]
async fn main() {
    let target = Box::new(File::create("rsille.log").expect("Can't create file"));

    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} {} - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.level(),
                record.args()
            )
        })
        .filter(None, log::LevelFilter::Info)
        .target(env_logger::Target::Pipe(target))
        .init();

    let mut panel = Panel::new(100, 100);
    let text_widget = Text::new("1".into());
    let animed = Animative::new(text_widget, |x| {
        let s = x.get_text();
        let number = s.parse::<i32>().unwrap() + 1;
        x.replace(number.to_string());
    });

    panel.push(animed);

    let render = render::Builder::new()
        .set_size((30, 30))
        .enable_all()
        .full_screen()
        .set_frame_limit(10)
        .build(panel)
        .unwrap();

    render.run();
}
