//! Multi-select quick prompt built from the same controls as YAML forms.
use crate::config::{self, Action, ButtonSpec, CheckboxSpec, Config, Node};
use crate::{invalid, runtime, Args, Failure, Output};
use serde_json::{json, Value};
use std::io::{self, Write};

#[derive(Debug, clap::Args)]
pub(crate) struct MultiSelectArgs {
    #[arg(long, default_value = "Select options")]
    title: String,
    /// Options in display and result order
    #[arg(required = true, num_args = 1..)]
    options: Vec<String>,
}

impl MultiSelectArgs {
    fn layout(&self) -> Node {
        let mut nodes = vec![Node::Text(self.title.clone())];
        nodes.extend(self.options.iter().enumerate().map(|(index, label)| {
            Node::Checkbox(CheckboxSpec {
                id: index.to_string(),
                label: label.clone(),
                checked: false,
            })
        }));
        nodes.push(Node::Row(vec![
            Node::Button(ButtonSpec {
                label: "Submit".into(),
                action: Action::Submit,
            }),
            Node::Button(ButtonSpec {
                label: "Cancel".into(),
                action: Action::Cancel,
            }),
        ]));
        Node::Column(nodes)
    }

    fn write_result(
        &self,
        values: &Value,
        output: Output,
        mut writer: impl Write,
    ) -> io::Result<()> {
        let mut selected = Vec::new();
        for (index, label) in self.options.iter().enumerate() {
            if values[index.to_string()].as_bool().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "missing checkbox state")
            })? {
                selected.push(label);
            }
        }
        match output {
            Output::Json => writeln!(
                writer,
                "{}",
                json!({"action": "submit", "values": {"value": selected}})
            ),
            Output::Plain => {
                for label in selected {
                    writeln!(writer, "{label}")?;
                }
                Ok(())
            }
        }
    }
}

pub(crate) fn run(prompt: &MultiSelectArgs, args: &Args) -> Result<u8, Failure> {
    let mut config = Config {
        version: 1,
        sixel: false,
        cell_pixels: args.cell_pixels.clone(),
        force_cell_pixels: args.force_cell_pixels,
        theme: args.theme.unwrap_or_default(),
        style: args.style.unwrap_or_default(),
        title: String::new(),
        mode: if args.fullscreen {
            "fullscreen"
        } else {
            "inline"
        }
        .into(),
        layout: prompt.layout(),
    };
    config::validate(&mut config, &Value::Null).map_err(invalid)?;
    crate::validate_graphics(&config)?;
    let Some(result) = crate::terminal::run(config).map_err(runtime)? else {
        return Ok(130);
    };
    prompt
        .write_result(&result["values"], args.output, io::stdout().lock())
        .map_err(|e| runtime(e.to_string()))?;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        prompt: MultiSelectArgs,
    }

    #[test]
    fn requires_options() {
        assert!(Cli::try_parse_from(["cmd"]).is_err());
    }

    #[test]
    fn preserves_positions_and_duplicates() {
        let prompt = Cli::parse_from(["cmd", "b", "skip", "a", "b"]).prompt;
        let values = json!({"3": true, "2": true, "1": false, "0": true});
        let mut out = Vec::new();
        prompt
            .write_result(&values, Output::Json, &mut out)
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&out).unwrap(),
            json!({"action":"submit","values":{"value":["b","a","b"]}})
        );
        out.clear();
        prompt
            .write_result(&values, Output::Plain, &mut out)
            .unwrap();
        assert_eq!(out, b"b\na\nb\n");
    }

    #[test]
    fn empty_plain_selection_writes_nothing() {
        let prompt = Cli::parse_from(["cmd", "a"]).prompt;
        let mut out = Vec::new();
        prompt
            .write_result(&json!({"0":false}), Output::Plain, &mut out)
            .unwrap();
        assert!(out.is_empty());
    }
}
