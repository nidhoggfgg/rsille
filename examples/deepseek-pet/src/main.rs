use clap::Parser;

fn main() -> std::io::Result<()> {
    deepseek_pet::Options::parse().run()
}
