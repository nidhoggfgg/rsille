use std::process::{Command, Stdio};

#[test]
fn no_arguments_prints_help() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rsille"));
    let output = command.output().unwrap();
    let help = command.arg("--help").output().unwrap();

    assert!(output.status.success());
    assert!(help.status.success());
    assert!(output.stderr.is_empty());
    assert!(String::from_utf8_lossy(&help.stdout).contains("Usage: rsille"));
    assert_eq!(output.stdout, help.stdout);
}

#[test]
fn no_arguments_with_piped_stdin_prints_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_rsille"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let help = Command::new(env!("CARGO_BIN_EXE_rsille"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, help.stdout);
}

#[test]
fn fullscreen_demo_requires_terminal() {
    let output = Command::new(env!("CARGO_BIN_EXE_rsille"))
        .arg("ai-babies")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("interactive terminal"));
}
