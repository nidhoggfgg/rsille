use std::fs::File;
use std::io::Write;

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

    todo!()
}
