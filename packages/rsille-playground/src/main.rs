use clap::{Parser, Subcommand};
use std::error::Error;
use std::io::{self, IsTerminal, Write};

#[derive(Parser)]
#[command(
    name = "rsille",
    version,
    about = "Explore the rsille terminal graphics playground"
)]
struct Args {
    #[command(subcommand)]
    experience: Option<Experience>,
}

#[derive(Subcommand)]
enum Experience {
    /// Explore the AI Babies animation
    #[command(visible_alias = "pet")]
    AiBabies,
    /// Plot functions in a Sixel terminal
    #[command(visible_alias = "draw")]
    FunctionPlotter,
    /// Watch the jellyfish swim
    Jellyfish,
}

impl Experience {
    fn run(self) -> Result<(), Box<dyn Error>> {
        match self {
            _ if !io::stdin().is_terminal() || !io::stdout().is_terminal() => {
                Err("This experience needs an interactive terminal.".into())
            }
            Self::AiBabies => ai_babies::run(),
            Self::FunctionPlotter => rsille_function_plotter::run(),
            Self::Jellyfish => rsille_jellyfish::run(),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    if let Some(experience) = Args::parse().experience {
        return experience.run();
    }
    loop {
        println!("rsille playground\n  1  draw\n  2  pet\n  3  ai-babies\n  4  function-plotter\n  5  jellyfish");
        print!("Choose a number or name (q to quit)> ");
        io::stdout().flush()?;
        let mut choice = String::new();
        if io::stdin().read_line(&mut choice)? == 0 {
            return Ok(());
        }
        match choice.trim() {
            "1" | "draw" | "function-plotter" => Experience::FunctionPlotter,
            "2" | "pet" | "ai-babies" => Experience::AiBabies,
            "3" | "jellyfish" => Experience::Jellyfish,
            "q" | "quit" => return Ok(()),
            _ => {
                println!("Choose one of the listed experiences.");
                continue;
            }
        }
        .run()?;
    }
}
