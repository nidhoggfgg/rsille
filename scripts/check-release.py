#!/usr/bin/env python3
"""Verify the single self-contained rsille publish archive and a clean consumer.

Requires Python 3.12+ and Cargo. Nothing is uploaded. Pass --offline to use cached
third-party dependencies; first-time checks may need network access.
"""
import argparse
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import tomllib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=root))
    target = Path(metadata["target_directory"])
    packages = {package["name"]: package for package in metadata["packages"]}
    assert {name for name, package in packages.items() if package["publish"] != []} == {"rsille"}
    version = packages["rsille"]["version"]

    def cargo(*arguments, cwd=root):
        command = ["cargo", *arguments]
        if args.offline:
            command.append("--offline")
        subprocess.run(command, cwd=cwd, check=True)

    # Cargo's verification builds the archive without any private workspace crates.
    cargo("package", "-p", "rsille", "--allow-dirty", "--all-features", "--registry", "crates-io")
    archive = target / "package" / f"rsille-{version}.crate"
    with tempfile.TemporaryDirectory(prefix="rsille-release-check-") as folder:
        extracted = Path(folder)
        with tarfile.open(archive) as bundle:
            bundle.extractall(extracted, filter="data")
        source = extracted / f"rsille-{version}"
        manifest = source / "Cargo.toml"
        data = tomllib.loads(manifest.read_text())
        private_packages = set(packages) - {"rsille"}
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            for name, dependency in data.get(section, {}).items():
                assert "path" not in dependency, (name, dependency)
                assert dependency.get("package", name) not in private_packages, (name, dependency)
        assert set(source.rglob("Cargo.toml")) == {manifest}, "only the rsille manifest belongs in the archive"
        assert not (source / "packages").exists(), "archive must not contain workspace crate manifests"
        for filename in ("README.md", "LICENCE", "MIGRATION_V3.md", "src/render/api.rs", "src/canvas/api.rs", "src/tui/api.rs"):
            assert (source / filename).is_file(), filename
        # Cargo archives normalize file timestamps. A reused target directory
        # can otherwise consider an older build of this version up to date.
        cargo("clean", "--manifest-path", str(manifest), "-p", "rsille",
              "--target-dir", str(target / "release-check-build"), cwd=source)
        for features in ([], ["--all-features"]):
            cargo("test", "--manifest-path", str(manifest),
                  "--target-dir", str(target / "release-check-build"), *features, cwd=source)

        # Only rsille is a local dependency; all modules come from the archive.
        consumer = extracted / "consumer"
        (consumer / "src").mkdir(parents=True)
        (consumer / "Cargo.toml").write_text('''[package]
name = "rsille-release-consumer"
version = "0.0.0"
edition = "2021"
[dependencies]
rsille = { path = "../rsille-RSILLE_VERSION" }
tokio = { version = "1", features = ["rt", "time", "net"] }
[features]
syntax-highlighting = ["rsille/syntax-highlighting"]
'''.replace("RSILLE_VERSION", version))
        workflow = root / "packages/tui/examples/basics/workflow.rs"
        program = workflow.read_text().replace("use tui::", "use rsille::tui::")
        program = program.replace('    let exit = if std::env::args()', '    check_canvas();\n    let exit = if std::env::args()')
        program += '''
fn check_canvas() {
    use rsille::tui::prelude::*;
    let mut app = App::new(())
        .update(|_, (), _| {})
        .view(|_| canvas(|surface: &mut rsille::canvas::Canvas, _| { surface.set(0.0, 0.0); }))
        .build().size(8, 2);
    app.step(&[]).unwrap();
    let frame: &rsille::render::buffer::Buffer = app.frame();
    assert!(frame.content().iter().any(|cell| matches!(cell.content.as_char(), Some('⠁'..='⣿'))));
    let mut app = App::new(())
        .update(|_, (), _| {})
        .view(|_| canvas(|surface, ctx| {
            assert_eq!(ctx.encoding(), Encoding::HalfBlock);
            surface.set_color(0, 0, Color::Rgb(0, 0, 255));
            surface.set_color(0, 1, Color::Rgb(255, 0, 0));
        }).encoding(Encoding::HalfBlock))
        .build().size(1, 1);
    app.step(&[]).unwrap();
    let cell = &app.frame().content()[0].content;
    assert_eq!(cell.as_char(), Some('▀'));
    let colors = cell.style.colors.unwrap();
    assert_eq!(colors.foreground, Some(Color::Rgb(255, 0, 0).into()));
    assert_eq!(colors.background, Some(Color::Rgb(0, 0, 255).into()));
}
'''
        (consumer / "src/main.rs").write_text(program)
        for features in ([], ["--all-features"]):
            command = ["cargo", "run", "--manifest-path", str(consumer / "Cargo.toml"),
                       "--target-dir", str(target / "release-check-build"), *features]
            if args.offline:
                command.append("--offline")
            subprocess.run([*command, "--", "--headless"], cwd=consumer, check=True)
        extensions = (root / "packages/tui/examples/basics/extensions.rs").read_text()
        (consumer / "src/main.rs").write_text(extensions.replace("use tui::", "use rsille::tui::"))
        for features in ([], ["--all-features"]):
            command = ["cargo", "run", "--manifest-path", str(consumer / "Cargo.toml"),
                       "--target-dir", str(target / "release-check-build"), *features]
            if args.offline:
                command.append("--offline")
            subprocess.run([*command, "--", "--headless"], cwd=consumer, check=True)
    print("The rsille archive, extracted tests and clean consumer passed with default/all features; nothing uploaded.")


if __name__ == "__main__":
    main()
