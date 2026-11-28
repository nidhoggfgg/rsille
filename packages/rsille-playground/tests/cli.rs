use std::process::{Command, Stdio};

#[test]
fn no_arguments_prints_help() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rsille"));
    let output = command.output().unwrap();
    let help = command.arg("--help").output().unwrap();

    assert!(output.status.success());
    assert!(help.status.success());
    assert!(output.stderr.is_empty());
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(text.contains("Usage: rsille"));
    for sub in [
        "voxel",
        "clockwork",
        "desk",
        "hud",
        "pet",
        "draw",
        "jellyfish",
        "doctor",
    ] {
        assert!(text.contains(sub), "help should list {sub}");
    }
    assert!(!text.contains("ai-babies"));
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
fn interactive_demos_require_terminal_without_headless() {
    for sub in ["voxel", "clockwork", "desk", "hud", "jellyfish"] {
        let output = Command::new(env!("CARGO_BIN_EXE_rsille"))
            .arg(sub)
            .output()
            .unwrap();
        assert!(!output.status.success(), "{sub} should fail without a TTY");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("interactive terminal"),
            "{sub} stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn doctor_plain_and_headless_subcommands_succeed() {
    let plain = Command::new(env!("CARGO_BIN_EXE_rsille"))
        .args(["doctor", "--plain"])
        .output()
        .unwrap();
    assert!(plain.status.success());
    let report = String::from_utf8_lossy(&plain.stdout);
    assert!(report.contains("rsille doctor"));
    assert!(report.contains("rsille voxel"));

    for sub in [
        "voxel",
        "clockwork",
        "desk",
        "hud",
        "draw",
        "jellyfish",
        "doctor",
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_rsille"))
            .args([sub, "--headless"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{sub} --headless failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
