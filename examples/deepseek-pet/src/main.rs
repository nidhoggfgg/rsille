fn main() -> std::io::Result<()> {
    let mut args = std::env::args_os().skip(1);
    match (args.next(), args.next()) {
        (None, None) => deepseek_pet::run(),
        (Some(arg), None) if arg == "--mouse" => deepseek_pet::run_with_mouse_capture(true),
        (Some(arg), None) if arg == "-h" || arg == "--help" => {
            println!("Usage: deepseek-pet [--mouse]\n\n  --mouse  Capture the mouse to interact with the whale");
            Ok(())
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Usage: deepseek-pet [--mouse]",
        )),
    }
}
