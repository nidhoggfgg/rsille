use clap::Args;
use std::io::{self, IsTerminal, Read, Write};

#[derive(Debug, Args)]
pub struct MultilineArgs {
    /// Prompt shown when reading from a terminal.
    #[arg(long, default_value = "Input")]
    pub label: String,
}

/// Collect UTF-8 input through EOF without writing to result stdout.
/// An empty submission is an empty string, not cancellation.
pub fn run(args: MultilineArgs) -> io::Result<String> {
    let stdin = io::stdin();
    let label = stdin.is_terminal().then_some(args.label.as_str());
    collect(stdin.lock(), io::stderr().lock(), label)
}

fn collect(
    mut input: impl Read,
    mut prompt: impl Write,
    label: Option<&str>,
) -> io::Result<String> {
    if let Some(label) = label {
        let eof = if cfg!(windows) {
            "Ctrl-Z then Enter"
        } else {
            "Ctrl-D"
        };
        writeln!(
            prompt,
            "{label} (Enter adds a line; {eof} on an empty line submits)"
        )?;
        prompt.flush()?;
    }
    let mut text = String::new();
    input.read_to_string(&mut text)?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redirected_text_is_returned_verbatim_without_a_prompt() {
        for text in ["", "  中\r\n\nlast ", "line\n"] {
            let mut prompt = Vec::new();
            assert_eq!(collect(text.as_bytes(), &mut prompt, None).unwrap(), text);
            assert!(prompt.is_empty());
        }
    }

    #[test]
    fn interactive_prompt_is_separate_from_text() {
        let mut prompt = Vec::new();
        assert_eq!(
            collect(&b"note"[..], &mut prompt, Some("Notes")).unwrap(),
            "note"
        );
        assert!(String::from_utf8(prompt).unwrap().starts_with("Notes ("));
    }

    #[test]
    fn rejects_non_utf8_input() {
        let err = collect(&[0xff][..], io::sink(), None).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
}
