//! Black-box protocol validation; interactive coverage lives in pty_test.py.
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rsille-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self, name: &str, text: &str) -> String {
        let path = self.0.join(name);
        fs::write(&path, text).unwrap();
        path.to_str().unwrap().to_owned()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rsille"));
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // A new session has no controlling terminal, even when Cargo has one.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Redirection alone does not detach the inherited interactive console.
        command.creation_flags(windows_sys::Win32::System::Threading::DETACHED_PROCESS);
    }
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!("CLI did not fail promptly: {args:?}: {output:?}");
        }
        thread::sleep(Duration::from_millis(10));
    }
    child.wait_with_output().unwrap()
}
fn failure(args: &[&str], code: i32) {
    let output = run(args);
    assert_eq!(output.status.code(), Some(code), "{args:?}: {output:?}");
    assert!(
        output.stdout.is_empty(),
        "errors must not emit a result: {output:?}"
    );
    assert!(!output.stderr.is_empty());
    assert!(
        !output.stderr.contains(&0x1b),
        "diagnostics must not contain ANSI"
    );
}

#[test]
fn help_and_version_need_no_terminal() {
    for args in [&["--help"][..], &["--version"][..]] {
        let output = run(args);
        assert!(output.status.success());
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}
#[test]
fn valid_prompt_without_terminal_fails_quickly() {
    failure(&["input"], 1);
    failure(&["input", "--inline"], 1);
    let fixture = Fixture::new();
    let path = fixture.file(
        "inline.yaml",
        "version: 1\nmode: inline\nlayout: {text: hi}",
    );
    failure(&["run", &path], 1);
}
#[test]
fn argument_and_plain_errors_precede_terminal_setup() {
    for args in [
        vec!["select"],
        vec!["--output", "other", "input"],
        vec!["--output", "plain", "table"],
        vec!["--output", "plain", "run", "missing.yaml"],
        vec!["run", "-"],
    ] {
        failure(&args, 2);
    }
}
#[test]
fn invalid_configuration_is_rejected_before_terminal_setup() {
    let fixture = Fixture::new();
    for layout in [
        json!({"unknown": "x"}),
        json!({"input": {"id":"x", "label":"X", "typo":true}}),
        json!({"button": {"label":"Run", "action":"shell"}}),
        json!({"select": {"id":"x", "label":"X", "options":[]}}),
        json!({"table": {"selectable":true}}),
        json!({"table": {"source":"data.missing"}}),
        json!({"table": {"rows":[1]}}),
        json!({"column":[{"input":{"id":"x","label":"X"}},{"checkbox":{"id":"x","label":"X"}}]}),
    ] {
        let path = fixture.file(
            "invalid.yaml",
            &json!({"version":1,"layout":layout}).to_string(),
        );
        failure(&["run", &path], 2);
    }
    for text in [
        "version: 2\nlayout: {text: hi}",
        "version: 1\nmode: other\nlayout: {text: hi}",
        "version: 1\ntypo: true\nlayout: {text: hi}",
        "version: [",
    ] {
        let path = fixture.file("invalid.yaml", text);
        failure(&["run", &path], 2);
    }
}
#[test]
fn invalid_data_and_resource_limits() {
    let fixture = Fixture::new();
    for text in [
        "{".to_owned(),
        "{}".to_owned(),
        "[1]".to_owned(),
        " ".repeat(4 * 1024 * 1024 + 1),
        json!(vec![json!({}); 10_001]).to_string(),
    ] {
        let path = fixture.file("data.json", &text);
        failure(&["table", &path], 2);
    }
    let mut nested = json!(null);
    for _ in 0..33 {
        nested = json!({"nested": nested});
    }
    let path = fixture.file("data.json", &json!([nested]).to_string());
    failure(&["table", &path], 2);
    let mut layout = json!({"text":"deep"});
    for _ in 0..32 {
        layout = json!({"column":[layout]});
    }
    for text in [
        json!({"version":1,"layout":layout}).to_string(),
        " ".repeat(4 * 1024 * 1024 + 1),
        json!({"version":1,"layout":{"column":vec![json!({"text":""});10_000]}}).to_string(),
    ] {
        let path = fixture.file("config.yaml", &text);
        failure(&["run", &path], 2);
    }
}
#[test]
fn bundled_deployment_example_passes_validation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
    failure(
        &[
            "run",
            root.join("deploy.yaml").to_str().unwrap(),
            "--data",
            root.join("services.json").to_str().unwrap(),
        ],
        1,
    );
}
