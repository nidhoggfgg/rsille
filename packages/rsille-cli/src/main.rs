mod config;
mod terminal;
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
    about = "Terminal interfaces for scripts (Unix, fullscreen)"
)]
struct Args {
    #[arg(
        long,
        global = true,
        value_enum,
        default_value = "json",
        help = "Plain output is only available for select, input and confirm"
    )]
    output: Output,
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
    /// Run a version 1 YAML interface
    Run {
        file: PathBuf,
        #[arg(long, help = "JSON file, or - to explicitly read stdin")]
        data: Option<PathBuf>,
    },
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
    let mut data = Value::Null;
    let confirm = matches!(&args.command, Command::Confirm { .. });
    if args.output == Output::Plain
        && matches!(&args.command, Command::Table { .. } | Command::Run { .. })
    {
        return Err(invalid(
            "--output plain only supports select, input and confirm",
        ));
    }
    let (title, layout) = match args.command {
        Command::Select { title, options } => (
            title.clone(),
            Node::Select(SelectSpec {
                id: "value".into(),
                label: title,
                options,
                selected: 0,
            }),
        ),
        Command::Input { label, value } => (
            label.clone(),
            Node::Input(InputSpec {
                id: "value".into(),
                label,
                value,
            }),
        ),
        Command::Confirm { title } => (
            title.clone(),
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
        Command::Run { file, data: path } => {
            if file == std::path::Path::new("-") {
                return Err(invalid(
                    "configuration must be a file; stdin is reserved for --data -",
                ));
            }
            if let Some(path) = path {
                data = json(&read_path(&path)?)?;
            }
            let config = config::parse(&read_path(&file)?, &data).map_err(invalid)?;
            if config.mode != "fullscreen" {
                return Err(invalid(
                    "only mode: fullscreen is supported in this release",
                ));
            }
            return Ok((config, args.output, false));
        }
    };
    let mut config = Config {
        version: 1,
        title,
        mode: "fullscreen".into(),
        layout,
    };
    config::validate(&mut config, &data).map_err(invalid)?;
    Ok((config, args.output, confirm))
}

fn execute(args: Args) -> Result<u8, Failure> {
    let (config, output, confirm) = prepare(args)?;
    let Some(mut result) = terminal::run(config).map_err(runtime)? else {
        return Ok(130);
    };
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

fn main() -> std::process::ExitCode {
    match execute(Args::parse()) {
        Ok(code) => code.into(),
        Err(error) => {
            eprintln!("rsille: {}", config::display_text(&error.message));
            error.code.into()
        }
    }
}
