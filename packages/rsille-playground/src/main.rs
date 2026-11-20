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

#[derive(Clone, Copy, clap::ValueEnum)]
enum PetCharacter {
    Deepseek,
}

#[derive(Subcommand)]
enum Experience {
    /// Start a terminal pet session
    Pet {
        #[arg(value_enum, default_value = "deepseek")]
        character: PetCharacter,
        #[command(flatten)]
        options: deepseek_pet::Options,
    },
    /// Explore the AI Babies animation
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
            Self::Pet {
                character: PetCharacter::Deepseek,
                options,
            } => Ok(options.run()?),
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
            "1" | "4" | "draw" | "function-plotter" => Experience::FunctionPlotter,
            "2" | "pet" => Experience::Pet {
                character: PetCharacter::Deepseek,
                options: deepseek_pet::Options::default(),
            },
            "3" | "ai-babies" => Experience::AiBabies,
            "5" | "jellyfish" => Experience::Jellyfish,
            "q" | "quit" => return Ok(()),
            _ => {
                println!("Choose one of the listed experiences.");
                continue;
            }
        }
        .run()?;
    }
}
