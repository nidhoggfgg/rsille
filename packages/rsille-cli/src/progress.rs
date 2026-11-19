use std::io::{self, Write};

/// Print a progress bar for a percentage from 0 to 100.
#[derive(clap::Args, Debug)]
pub(crate) struct ProgressArgs {
    /// Percentage complete (inclusive).
    #[arg(value_parser = clap::value_parser!(u8).range(0..=100))]
    percent: u8,
    /// Number of cells inside the bar.
    #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u8).range(1..=200))]
    width: u8,
}

pub(crate) fn run(args: ProgressArgs) -> io::Result<()> {
    render(args, io::stdout().lock())
}

fn render(args: ProgressArgs, mut out: impl Write) -> io::Result<()> {
    let width = usize::from(args.width);
    let filled = width * usize::from(args.percent) / 100;
    writeln!(
        out,
        "[{}{}] {}%",
        "#".repeat(filled),
        "-".repeat(width - filled),
        args.percent
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        args: ProgressArgs,
    }

    #[test]
    fn renders_exact_width_without_rounding_up() {
        for (percent, expected) in [
            ("0", "[---] 0%\n"),
            ("99", "[##-] 99%\n"),
            ("100", "[###] 100%\n"),
        ] {
            let cli = Cli::try_parse_from(["progress", percent, "--width", "3"]).unwrap();
            let mut output = Vec::new();
            render(cli.args, &mut output).unwrap();
            assert_eq!(output, expected.as_bytes());
        }
    }

    #[test]
    fn validates_arguments() {
        assert_eq!(
            Cli::try_parse_from(["progress", "50"]).unwrap().args.width,
            20
        );
        for argv in [
            vec!["progress"],
            vec!["progress", "101"],
            vec!["progress", "0", "--width", "0"],
            vec!["progress", "0", "--width", "201"],
        ] {
            assert!(Cli::try_parse_from(argv).is_err());
        }
    }
}
