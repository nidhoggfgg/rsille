mod ai_babies;
mod deepseek_pet;
mod function_plotter;
mod jellyfish;

use clap::{CommandFactory, Parser, Subcommand};
use std::error::Error;
use std::io::{self, IsTerminal};

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
    #[command(disable_help_flag = true)]
    AiBabies {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Plot functions with Kitty or Sixel graphics
    #[command(visible_alias = "draw", disable_help_flag = true)]
    FunctionPlotter {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Watch the jellyfish swim
    #[command(disable_help_flag = true)]
    Jellyfish {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

impl Experience {
    fn run(self) -> Result<(), Box<dyn Error>> {
        match self {
            Self::Pet {
                character: PetCharacter::Deepseek,
                options,
            } => {
                require_terminal()?;
                Ok(options.run()?)
            }
            Self::AiBabies { args } => ai_babies::run(&args),
            Self::FunctionPlotter { args } => function_plotter::run(args),
            Self::Jellyfish { args } => jellyfish::run(&args),
        }
    }
}

fn require_terminal() -> Result<(), Box<dyn Error>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("This experience needs an interactive terminal.".into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    if let Some(experience) = Args::parse().experience {
        return experience.run();
    }
    Args::command().print_help()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pet_passes_every_argument_after_the_separator_to_the_child() {
        for character in [None, Some("deepseek")] {
            let args = ["rsille", "pet"].into_iter().chain(character).chain([
                "--backend",
                "kitty",
                "--",
                "vim",
                "-Nu",
                "NONE",
                "--",
                "my notes",
            ]);
            let Some(Experience::Pet { options, .. }) =
                Args::try_parse_from(args).unwrap().experience
            else {
                panic!("expected a pet session");
            };
            assert_eq!(
                options.command,
                ["vim", "-Nu", "NONE", "--", "my notes"].map(std::ffi::OsString::from)
            );
        }
    }
}
