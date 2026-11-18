fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.is_empty() {
        rsille_jellyfish::run()
    } else {
        rsille_jellyfish::export_command(&args)
    }
}
