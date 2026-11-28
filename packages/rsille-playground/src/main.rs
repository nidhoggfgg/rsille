mod clockwork;
mod deepseek_pet;
mod desk;
mod doctor;
mod function_plotter;
mod hud;
mod jellyfish;
mod launcher;
mod voxel;

use clap::{CommandFactory, Parser, Subcommand};
use std::error::Error;
use std::io::{self, IsTerminal};

#[derive(Parser)]
#[command(
    name = "rsille",
    version,
    about = "Explore the rsille terminal UI, graphics & live shell overlay playground"
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
    /// First-person 3D voxel sandbox in terminal cells (works in any True Color terminal)
    #[command(visible_alias = "craft", disable_help_flag = true)]
    Voxel {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// 132 mechanical parts assembling into animated 3D creatures
    #[command(disable_help_flag = true)]
    Clockwork {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Desktop-style TUI workspace with draggable floating windows, effects & themes
    #[command(visible_alias = "lumen", disable_help_flag = true)]
    Desk {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Floating live shell HUD with directory tree, task checklist, command timer & macros
    Hud {
        #[command(flatten)]
        options: hud::Options,
    },
    /// Start a live shell terminal pet session (Kitty / Sixel)
    Pet {
        #[arg(value_enum, default_value = "deepseek")]
        character: PetCharacter,
        #[command(flatten)]
        options: deepseek_pet::Options,
    },
    /// Plot 2D & rotating 3D math functions inline (Kitty / Sixel)
    #[command(visible_alias = "draw", disable_help_flag = true)]
    FunctionPlotter {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Watch bioluminescent jellyfish drift in a deep-sea aquarium
    #[command(disable_help_flag = true)]
    Jellyfish {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Probe terminal capabilities (Kitty, Sixel, cell pixels) & compare all rendering backends
    #[command(disable_help_flag = true)]
    Doctor {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

impl Experience {
    fn run(self) -> Result<(), Box<dyn Error>> {
        match self {
            Self::Voxel { args } => voxel::run(&args),
            Self::Clockwork { args } => clockwork::run(&args),
            Self::Desk { args } => desk::run(&args),
            Self::Hud { options } => options.run(),
            Self::Pet {
                character: PetCharacter::Deepseek,
                options,
            } => {
                require_terminal()?;
                Ok(options.run()?)
            }
            Self::FunctionPlotter { args } => function_plotter::run(args),
            Self::Jellyfish { args } => jellyfish::run(&args),
            Self::Doctor { args } => doctor::run(&args),
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
    if io::stdin().is_terminal() && io::stdout().is_terminal() {
        return launcher::run();
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

    #[test]
    fn hud_parses_tab_anchor_todos_and_child_command() {
        let args = [
            "rsille",
            "hud",
            "--tab",
            "tasks",
            "--anchor",
            "bottom-right",
            "--todo",
            "Ship v0.2",
            "--",
            "bash",
            "-l",
        ];
        let Some(Experience::Hud { options }) = Args::try_parse_from(args).unwrap().experience
        else {
            panic!("expected a hud session");
        };
        assert_eq!(options.tab, hud::HudTab::Tasks);
        assert_eq!(options.anchor, hud::HudAnchor::BottomRight);
        assert_eq!(options.todo, vec!["Ship v0.2"]);
        assert_eq!(
            options.command,
            ["bash", "-l"].map(std::ffi::OsString::from)
        );
    }

    #[test]
    fn aliases_parse_to_expected_experiences() {
        assert!(matches!(
            Args::try_parse_from(["rsille", "craft", "--headless"])
                .unwrap()
                .experience,
            Some(Experience::Voxel { .. })
        ));
        assert!(matches!(
            Args::try_parse_from(["rsille", "lumen", "--headless"])
                .unwrap()
                .experience,
            Some(Experience::Desk { .. })
        ));
        assert!(matches!(
            Args::try_parse_from(["rsille", "draw", "--headless"])
                .unwrap()
                .experience,
            Some(Experience::FunctionPlotter { .. })
        ));
    }
}
