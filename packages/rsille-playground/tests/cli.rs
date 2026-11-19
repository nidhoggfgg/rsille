use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn menu_recovers_from_unknown_choice() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rsille"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"invalid\nq\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .matches("Choose a number")
            .count(),
        2
    );
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
