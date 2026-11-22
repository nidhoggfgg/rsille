#!/usr/bin/env python3
"""Verify the core and syntax-highlighting extension archives and a downstream application.

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
    archives = ("rsille-render", "rsille-canvas", "rsille-tui", "rsille", "rsille-syntect")
    selection = [arg for name in archives for arg in ("-p", name)]
    for name in archives:
        registries = packages[name]["publish"]
        assert registries is None or "crates-io" in registries, f"{name} must be publishable"

    def cargo(*arguments, cwd, program_args=()):
        command = ["cargo", *arguments]
        if args.offline:
            command.append("--offline")
        if program_args:
            command.extend(["--", *program_args])
        subprocess.run(command, cwd=cwd, check=True)

    with tempfile.TemporaryDirectory(prefix="rsille-release-check-") as folder:
        extracted = Path(folder).resolve()
        patches = []
        members = []
        config = extracted / "archives.toml"
        for name in archives:
            # Resolve unpublished dependencies from the archives already verified.
            config.write_text("[patch.crates-io]\n" + "\n".join(patches))
            cargo("package", "-p", name, "--allow-dirty", "--all-features",
                  "--registry", "crates-io", "--config", str(config), cwd=root)
            directory = f'{name}-{packages[name]["version"]}'
            with tarfile.open(target / "package" / f"{directory}.crate") as bundle:
                bundle.extractall(extracted, filter="data")
            source = extracted / directory
            manifest = source / "Cargo.toml"
            data = tomllib.loads(manifest.read_text())
            assert set(source.rglob("Cargo.toml")) == {manifest}
            assert (source / "src/lib.rs").is_file()
            assert data["package"]["name"] == name
            for table in (data, *data.get("target", {}).values()):
                for kind in ("dependencies", "dev-dependencies", "build-dependencies"):
                    for dep in table.get(kind, {}).values():
                        assert "path" not in dep and "git" not in dep, (name, dep)
            members.append(directory)
            patches.append(f'{name} = {{ path = {json.dumps(str(source))} }}')

        consumer = extracted / "consumer"
        (consumer / "src").mkdir(parents=True)
        (extracted / "Cargo.toml").write_text(
            '[workspace]\nresolver = "3"\nmembers = ' + json.dumps([*members, "consumer"])
            + '\n[patch.crates-io]\n' + '\n'.join(patches) + '\n')
        (consumer / "Cargo.toml").write_text(f'''[package]
name = "rsille-release-consumer"
version = "0.0.0"
edition = "2021"
[dependencies]
rsille = {{ version = "={packages["rsille"]["version"]}", default-features = false }}
tokio = {{ version = "1", features = ["rt", "time", "net"] }}
[features]
raster = ["rsille/raster"]
''')
        workflow = root / "packages/rsille-tui/examples/basics/workflow.rs"
        program = workflow.read_text().replace("use rsille_tui::", "use rsille::tui::")
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
        extensions = (workflow.parent / "extensions.rs").read_text().replace("use rsille_tui::", "use rsille::tui::")
        main_rs = consumer / "src/main.rs"
        main_rs.write_text(program)
        build_args = ["--target-dir", str(target / "release-check-build")]
        for features in (["--no-default-features"], ["--all-features"]):
            cargo("test", *selection, "--lib", *build_args, *features, cwd=extracted)
            cargo("test", *selection, "--doc", *build_args, *features, cwd=extracted)
            for example in (program, extensions):
                main_rs.write_text(example)
                cargo("run", *build_args, *features, cwd=consumer, program_args=["--headless"])
    print("Verified the core archives, rsille-syntect archive and headless consumer.")


if __name__ == "__main__":
    main()
