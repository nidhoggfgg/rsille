fn main() -> std::io::Result<()> {
    const USAGE: &str = "Usage: deepseek-pet [--quite] [--alt-screen]";
    let mut options = deepseek_pet::Options::default();
    for arg in std::env::args_os().skip(1) {
        match arg.to_str() {
            Some("--quite") => options.capture_mouse = false,
            Some("--alt-screen") => options.alt_screen = true,
            Some("-h" | "--help") => {
                println!("{USAGE}\n\n  --quite       Disable pet mouse capture\n  --alt-screen  Run the pet on the alternate screen");
                return Ok(());
            }
            _ => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, USAGE)),
        }
    }
    options.run()
}
