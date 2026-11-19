mod config;
mod jellyfish;
mod multiline;
mod multiselect;
mod progress;
mod style;
mod terminal;
mod theme;
mod ui;

use clap::{Parser, Subcommand, ValueEnum};
use config::{Action, ButtonSpec, Config, InputSpec, Node, SelectSpec, TableSpec};
use serde_json::Value;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;

const MAX_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Parser)]
#[command(
    name = "rsille",
    version,
    about = "Terminal interfaces for scripts (Unix and Windows, fullscreen or inline)"
)]
struct Args {
    #[arg(
        long,
        global = true,
        value_enum,
        default_value = "json",
        help = "Result format for interactive prompts (progress always prints a bar)"
    )]
    output: Output,
    /// Render below the current cursor without switching to the alternate screen
    #[arg(long, global = true, conflicts_with = "fullscreen")]
    inline: bool,
    /// Render on the alternate screen; overrides the YAML mode
    #[arg(long, global = true, conflicts_with = "inline")]
    fullscreen: bool,
    /// Color preset; overrides the YAML theme (default: dark)
    #[arg(long, global = true, value_enum)]
    theme: Option<theme::ThemePreset>,
    /// Structural style preset; overrides YAML style (default: classic)
    #[arg(long, global = true, value_enum)]
    style: Option<style::StylePreset>,

    /// Physical pixels per character cell, e.g. 8x18
    #[arg(long, global = true)]
    cell_pixels: Option<String>,
    /// Allow cell metrics that equal the terminal grid dimensions
    #[arg(long, global = true)]
    force_cell_pixels: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum, PartialEq)]
enum Output {
    Json,
    Plain,
}

#[derive(Subcommand)]
enum Command {
    /// Select zero or more options
    #[command(alias = "multiselect")]
    MultiSelect(multiselect::MultiSelectArgs),
    /// Read multiline text from stdin until EOF
    Multiline(multiline::MultilineArgs),
    /// Print a progress bar for a percentage
    Progress(progress::ProgressArgs),
    /// Select one value; arrows navigate, Enter submits
    Select {
        #[arg(long, default_value = "Select")]
        title: String,
        #[arg(required = true)]
        options: Vec<String>,
    },
    /// Enter text; Enter submits
    Input {
        #[arg(long, default_value = "Input")]
        label: String,
        #[arg(long, default_value = "")]
        value: String,
    },
    /// Enter confirms; Esc or Ctrl-C cancels
    Confirm {
        #[arg(long, default_value = "Confirm?")]
        title: String,
    },
    /// View a JSON object array from a file or redirected stdin
    Table {
        file: Option<PathBuf>,
        #[arg(long, default_value = "Table")]
        title: String,
        #[arg(long)]
        selectable: bool,
        #[arg(long, value_delimiter = ',')]
        columns: Vec<String>,
    },
    /// Display Typst math (without dollar delimiters); Enter completes viewing
    Formula {
        source: String,
        #[arg(long, default_value_t = 18.0)]
        font_size: f64,
        #[arg(long, default_value_t = 2.0)]
        pixels_per_point: f32,
    },
    /// Explore the bundled demos
    #[command(subcommand)]
    Demo(Demo),
    /// Run a version 1 YAML interface
    Run {
        file: PathBuf,
        #[arg(long, help = "JSON file, or - to explicitly read stdin")]
        data: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum Demo {
    /// Plot functions interactively in a Sixel terminal
    FunctionPlotter,
    /// Draw the three AI Babies characters and exit
    AiBabies,
    /// Watch the jellyfish animation
    Jellyfish,
}

struct Failure {
    code: u8,
    message: String,
}
fn invalid(message: impl Into<String>) -> Failure {
    Failure {
        code: 2,
        message: message.into(),
    }
}
fn runtime(message: impl Into<String>) -> Failure {
    Failure {
        code: 1,
        message: message.into(),
    }
}

fn read_limited(reader: impl Read) -> Result<String, Failure> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| invalid(format!("cannot read input: {e}")))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(invalid("input exceeds 4 MiB"));
    }
    String::from_utf8(bytes).map_err(|_| invalid("input must be UTF-8"))
}

fn read_path(path: &std::path::Path) -> Result<String, Failure> {
    if path == std::path::Path::new("-") {
        if io::stdin().is_terminal() {
            return Err(invalid("stdin must be redirected when using -"));
        }
        read_limited(io::stdin().lock())
    } else {
        let file = std::fs::File::open(path)
            .map_err(|e| invalid(format!("cannot open {}: {e}", path.display())))?;
        read_limited(file)
    }
}

fn json(text: &str) -> Result<Value, Failure> {
    serde_json::from_str(text).map_err(|e| {
        invalid(format!(
            "invalid JSON at line {}, column {} ({:?})",
            e.line(),
            e.column(),
            e.classify()
        ))
    })
}

fn prepare(args: Args) -> Result<(Config, Output, bool), Failure> {
    // Clap may not detect global conflicts across the subcommand boundary.
    if args.inline && args.fullscreen {
        return Err(invalid("--fullscreen cannot be used with --inline"));
    }
    let mut data = Value::Null;
    let confirm = matches!(&args.command, Command::Confirm { .. });
    if args.output == Output::Plain
        && matches!(
            &args.command,
            Command::Table { .. }
                | Command::Run { .. }
                | Command::Formula { .. }
                | Command::Demo(_)
        )
    {
        return Err(invalid(
            "--output plain only supports select, input and confirm",
        ));
    }
    let (title, layout) = match args.command {
        Command::Formula {
            source,
            font_size,
            pixels_per_point,
        } => (
            String::new(),
            Node::Formula(config::FormulaSpec {
                source,
                font_size,
                pixels_per_point,
            }),
        ),
        Command::Select { title, options } => (
            String::new(),
            Node::Select(SelectSpec {
                id: "value".into(),
                label: title,
                options,
                selected: 0,
            }),
        ),
        Command::Input { label, value } => (
            String::new(),
            Node::Input(InputSpec {
                id: "value".into(),
                label,
                value,
            }),
        ),
        Command::Confirm { title } => (
            String::new(),
            Node::Column(vec![
                Node::Text(title),
                Node::Row(vec![
                    Node::Button(ButtonSpec {
                        label: "Confirm".into(),
                        action: Action::Submit,
                    }),
                    Node::Button(ButtonSpec {
                        label: "Cancel".into(),
                        action: Action::Cancel,
                    }),
                ]),
            ]),
        ),
        Command::Table {
            file,
            title,
            selectable,
            columns,
        } => {
            let text = match file {
                Some(path) => read_path(&path)?,
                None if !io::stdin().is_terminal() => read_limited(io::stdin().lock())?,
                None => return Err(invalid("table requires a JSON file or redirected stdin")),
            };
            data = json(&text)?;
            let rows = data
                .as_array()
                .ok_or_else(|| invalid("table data must be an array of JSON objects"))?
                .clone();
            (
                title,
                Node::Table(TableSpec {
                    id: selectable.then(|| "value".into()),
                    source: None,
                    rows,
                    selectable,
                    columns,
                }),
            )
        }
        Command::Demo(_)
        | Command::MultiSelect(_)
        | Command::Multiline(_)
        | Command::Progress(_) => return Err(invalid("expected a form command")),
        Command::Run { file, data: path } => {
            if file == std::path::Path::new("-") {
                return Err(invalid(
                    "configuration must be a file; stdin is reserved for --data -",
                ));
            }
            if let Some(path) = path {
                data = json(&read_path(&path)?)?;
            }
            let mut config = config::parse(&read_path(&file)?, &data).map_err(invalid)?;
            if args.fullscreen {
                config.mode = "fullscreen".into();
            } else if args.inline {
                config.mode = "inline".into();
            }
            if let Some(style) = args.style {
                config.style = style;
            }
            if let Some(theme) = args.theme {
                config.theme = theme;
            }

            if args.cell_pixels.is_some() {
                config.cell_pixels = args.cell_pixels;
            }
            config.force_cell_pixels |= args.force_cell_pixels;
            validate_graphics(&config)?;
            return Ok((config, args.output, false));
        }
    };
    let mut config = Config {
        version: 1,
        sixel: false,
        cell_pixels: args.cell_pixels,
        force_cell_pixels: args.force_cell_pixels,
        theme: args.theme.unwrap_or_default(),
        style: args.style.unwrap_or_default(),
        title,
        mode: if args.fullscreen {
            "fullscreen"
        } else {
            "inline"
        }
        .into(),
        layout,
    };
    config::validate(&mut config, &data).map_err(invalid)?;
    validate_graphics(&config)?;
    Ok((config, args.output, confirm))
}

fn validate_graphics(config: &Config) -> Result<(), Failure> {
    if config.force_cell_pixels && config.cell_pixels.is_none() {
        return Err(invalid("--force-cell-pixels requires --cell-pixels"));
    }
    if let Some(value) = &config.cell_pixels {
        let (width, height) = value
            .split_once('x')
            .ok_or_else(|| invalid("--cell-pixels requires WxH"))?;
        let pixels = rsille::render::raster_output::CellPixels {
            width: width
                .parse()
                .map_err(|_| invalid("invalid cell pixel width"))?,
            height: height
                .parse()
                .map_err(|_| invalid("invalid cell pixel height"))?,
        };
        rsille::render::raster_output::CellPixels::resolve(Some(pixels))
            .map_err(|e| invalid(e.to_string()))?;
    }
    Ok(())
}

fn execute(args: Args) -> Result<u8, Failure> {
    if args.inline && args.fullscreen {
        return Err(invalid("--fullscreen cannot be used with --inline"));
    }
    match args.command {
        Command::MultiSelect(ref prompt) => return multiselect::run(prompt, &args),
        Command::Progress(prompt) => {
            return progress::run(prompt)
                .map(|_| 0)
                .map_err(|e| runtime(e.to_string()))
        }
        Command::Multiline(prompt) => {
            let value = multiline::run(prompt).map_err(|e| runtime(e.to_string()))?;
            let mut stdout = io::stdout().lock();
            match args.output {
                Output::Plain => write!(stdout, "{value}"),
                Output::Json => writeln!(
                    stdout,
                    "{}",
                    serde_json::json!({"action":"submit","values":{"value":value}})
                ),
            }
            .map_err(|e| runtime(e.to_string()))?;
            return Ok(0);
        }
        _ => (),
    }
    if let Command::Demo(demo) = args.command {
        if args.output == Output::Plain {
            return Err(invalid(
                "--output plain only supports select, input and confirm",
            ));
        }
        match demo {
            Demo::Jellyfish => jellyfish::run(),
            Demo::FunctionPlotter => rsille_function_plotter::run(),
            Demo::AiBabies => {
                if !io::stdout().is_terminal() {
                    return Err(runtime("ai-babies requires terminal stdout"));
                }
                ai_babies::run()
            }
        }
        .map_err(|e| runtime(e.to_string()))?;
        return Ok(0);
    }
    let (config, output, confirm) = prepare(args)?;
    let is_static = config.is_static();
    let Some(mut result) = terminal::run(config).map_err(runtime)? else {
        return Ok(130);
    };
    if is_static {
        return Ok(0);
    }
    if confirm {
        result["values"]["value"] = Value::Bool(true);
    }
    let text = match output {
        Output::Json => serde_json::to_string(&result).map_err(|e| runtime(e.to_string()))?,
        Output::Plain => match &result["values"]["value"] {
            Value::String(text) => text.clone(),
            Value::Bool(value) => value.to_string(),
            _ => return Err(runtime("expected a single result value")),
        },
    };
    // The terminal session has been restored before any result is emitted.
    writeln!(io::stdout().lock(), "{text}")
        .map_err(|e| runtime(format!("cannot write result: {e}")))?;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formula_arguments_and_cell_metrics_are_validated() {
        for argv in [
            vec!["rsille", "formula", "x", "--output", "plain"],
            vec!["rsille", "formula", "x", "--font-size", "0"],
            vec!["rsille", "formula", "x", "--pixels-per-point", "17"],
            vec!["rsille", "input", "--cell-pixels", "8"],
            vec!["rsille", "input", "--cell-pixels", "0x18"],
            vec!["rsille", "input", "--cell-pixels", "8xno"],
            vec!["rsille", "input", "--force-cell-pixels"],
        ] {
            let result = prepare(Args::try_parse_from(argv).unwrap());
            assert!(matches!(result, Err(Failure { code: 2, .. })));
        }
        let args = Args::try_parse_from([
            "rsille",
            "formula",
            "x^2",
            "--cell-pixels",
            "8x18",
            "--inline",
        ])
        .unwrap();
        let (config, _, _) = prepare(args).unwrap_or_else(|e| panic!("{}", e.message));
        assert!(config.sixel);
        assert_eq!(config.mode, "inline");
        assert!(matches!(config.layout, Node::Formula(_)));
    }

    #[test]
    fn defaults_to_inline_and_infers_formula_graphics() {
        for argv in [
            vec!["rsille", "formula", "x^2"],
            vec!["rsille", "formula", "x^2", "--fullscreen"],
            vec![
                "rsille",
                "input",
                "--cell-pixels",
                "8x18",
                "--force-cell-pixels",
            ],
        ] {
            let fullscreen = argv.contains(&"--fullscreen");
            let formula = argv.contains(&"formula");
            let (config, _, _) = prepare(Args::try_parse_from(argv).unwrap())
                .unwrap_or_else(|e| panic!("{}", e.message));
            assert_eq!(
                config.mode,
                if fullscreen { "fullscreen" } else { "inline" }
            );
            assert_eq!(config.sixel, formula);
            assert_eq!(config.is_static(), formula && !fullscreen);
        }
        assert!(Args::try_parse_from(["rsille", "formula", "x", "--sixel"]).is_err());
        for argv in [
            vec!["rsille", "--inline", "input", "--fullscreen"],
            vec!["rsille", "--fullscreen", "input", "--inline"],
        ] {
            if let Ok(args) = Args::try_parse_from(argv) {
                assert!(matches!(prepare(args), Err(Failure { code: 2, .. })));
            }
        }
        assert!(Args::try_parse_from(["rsille", "input", "--inline", "--fullscreen"]).is_err());
    }

    #[test]
    fn cli_mode_overrides_yaml_only_when_explicit() {
        let path = std::env::temp_dir().join(format!("rsille-mode-{}.yaml", std::process::id()));
        for mode in ["inline", "fullscreen"] {
            std::fs::write(
                &path,
                format!("version: 1\nmode: {mode}\nlayout: {{formula: {{source: x}}}}"),
            )
            .unwrap();
            for (flag, expected) in [
                (None, mode),
                (Some("--inline"), "inline"),
                (Some("--fullscreen"), "fullscreen"),
            ] {
                for before in [false, true] {
                    let mut argv = vec!["rsille"];
                    if before {
                        argv.extend(flag);
                    }
                    argv.extend(["run", path.to_str().unwrap()]);
                    if !before {
                        argv.extend(flag);
                    }
                    let (config, _, _) = prepare(Args::try_parse_from(argv).unwrap())
                        .unwrap_or_else(|e| panic!("{}", e.message));
                    assert_eq!(config.mode, expected);
                    assert!(config.sixel);
                    assert_eq!(config.is_static(), expected == "inline");
                }
            }
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn style_names_match_cli_and_yaml() {
        use style::StylePreset;
        for preset in StylePreset::value_variants() {
            let value = preset.to_possible_value().unwrap();
            let name = value.get_name();
            for argv in [
                vec!["rsille", "--style", name, "input"],
                vec!["rsille", "input", "--style", name],
            ] {
                let args = Args::try_parse_from(argv).unwrap();
                let (config, _, _) = prepare(args).unwrap_or_else(|e| panic!("{}", e.message));
                assert_eq!(config.style, *preset);
            }
            let config = config::parse(
                &format!("version: 1\nstyle: {name}\nlayout: {{text: Hello}}"),
                &Value::Null,
            )
            .unwrap();
            assert_eq!(config.style, *preset);
        }
        assert!(Args::try_parse_from(["rsille", "input", "--style", "unknown"]).is_err());
        assert!(config::parse(
            "version: 1\nstyle: unknown\nlayout: {text: Hello}",
            &Value::Null
        )
        .is_err());
    }

    #[test]
    fn style_override_preserves_color_selection() {
        let path = std::env::temp_dir().join(format!("rsille-style-{}.yaml", std::process::id()));
        std::fs::write(
            &path,
            "version: 1\nstyle: retro\ntheme: light\nlayout: {text: Hello}",
        )
        .unwrap();
        for (extra, expected) in [
            (vec![], style::StylePreset::Retro),
            (vec!["--style", "classic"], style::StylePreset::Classic),
        ] {
            let mut argv = vec!["rsille", "run", path.to_str().unwrap()];
            argv.extend(extra);
            let (config, _, _) = prepare(Args::try_parse_from(argv).unwrap())
                .unwrap_or_else(|e| panic!("{}", e.message));
            assert_eq!(config.style, expected);
            assert_eq!(config.theme, theme::ThemePreset::Light);
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn theme_names_match_cli_and_yaml() {
        use theme::ThemePreset;
        for preset in ThemePreset::value_variants() {
            let value = preset.to_possible_value().unwrap();
            let name = value.get_name();
            for argv in [
                vec!["rsille", "--theme", name, "input"],
                vec!["rsille", "input", "--theme", name],
            ] {
                let args = Args::try_parse_from(argv).unwrap();
                let (config, _, _) = prepare(args).unwrap_or_else(|e| panic!("{}", e.message));
                assert_eq!(config.theme, *preset);
            }
            let config = config::parse(
                &format!("version: 1\ntheme: {name}\nlayout: {{text: Hello}}"),
                &Value::Null,
            )
            .unwrap();
            assert_eq!(config.theme, *preset);
            assert_eq!(preset.build().name, name);
        }
        let args = Args::try_parse_from(["rsille", "input"]).unwrap();
        assert_eq!(
            prepare(args)
                .unwrap_or_else(|e| panic!("{}", e.message))
                .0
                .theme,
            ThemePreset::Dark
        );
        assert!(Args::try_parse_from(["rsille", "input", "--theme", "unknown"]).is_err());
        assert!(config::parse(
            "version: 1\ntheme: unknown\nlayout: {text: Hello}",
            &Value::Null
        )
        .is_err());
    }

    #[test]
    fn cli_theme_overrides_yaml_only_when_explicit() {
        use theme::ThemePreset;
        let path = std::env::temp_dir().join(format!("rsille-theme-{}.yaml", std::process::id()));
        std::fs::write(&path, "version: 1\ntheme: dracula\nlayout: {text: Hello}").unwrap();
        for (extra, expected) in [
            (vec![], ThemePreset::Dracula),
            (vec!["--theme", "light"], ThemePreset::Light),
            (vec!["--theme", "dark"], ThemePreset::Dark),
        ] {
            let mut argv = vec!["rsille", "run", path.to_str().unwrap()];
            argv.extend(extra);
            let args = Args::try_parse_from(argv).unwrap();
            let (config, _, _) = prepare(args).unwrap_or_else(|e| panic!("{}", e.message));
            assert_eq!(config.theme, expected);
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn quick_prompts_do_not_duplicate_their_heading() {
        for inline in [false, true] {
            for command in [
                vec!["select", "--title", "Choose environment", "dev", "prod"],
                vec!["input", "--label", "Project name"],
                vec!["confirm", "--title", "Continue deployment?"],
            ] {
                let mut argv = vec!["rsille"];
                if inline {
                    argv.push("--inline");
                }
                argv.extend(command);
                let args = Args::try_parse_from(argv).unwrap();
                let (config, _, _) =
                    prepare(args).unwrap_or_else(|error| panic!("{}", error.message));
                assert!(config.title.is_empty());
                match config.layout {
                    Node::Select(spec) => assert_eq!(spec.label, "Choose environment"),
                    Node::Input(spec) => assert_eq!(spec.label, "Project name"),
                    Node::Column(nodes) => {
                        assert!(
                            matches!(&nodes[0], Node::Text(text) if text == "Continue deployment?")
                        );
                    }
                    _ => panic!("unexpected prompt layout"),
                }
            }
        }
    }
}

fn main() -> std::process::ExitCode {
    match execute(Args::parse()) {
        Ok(code) => code.into(),
        Err(error) => {
            eprintln!("rsille: {}", config::display_text(&error.message));
            error.code.into()
        }
    }
}
