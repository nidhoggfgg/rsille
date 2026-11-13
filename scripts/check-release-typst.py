#!/usr/bin/env python3
"""Verify rsille-typst's archive against registry dependencies, without uploading.

Requires Python 3.12+ and Cargo. Pass --offline to use cached dependencies.
Tests and both headless demo modes run outside the repository workspace so a
local rsille checkout cannot mask incompatibility with the published core.
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
    package = next(p for p in metadata["packages"] if p["name"] == "rsille-typst")
    assert package["publish"] is None or "crates-io" in package["publish"], \
        "rsille-typst must be publishable to crates.io"
    version = package["version"]

    def cargo(*arguments, cwd=root, program_args=()):
        command = ["cargo", *arguments]
        if args.offline:
            command.append("--offline")
        if program_args:
            command.extend(["--", *program_args])
        subprocess.run(command, cwd=cwd, check=True)

    cargo("package", "-p", "rsille-typst", "--locked", "--registry", "crates-io")
    archive = target / "package" / f"rsille-typst-{version}.crate"
    with tempfile.TemporaryDirectory(prefix="rsille-typst-release-check-") as folder:
        extracted = Path(folder)
        with tarfile.open(archive) as bundle:
            bundle.extractall(extracted, filter="data")
        source = extracted / f"rsille-typst-{version}"
        manifest = source / "Cargo.toml"
        data = tomllib.loads(manifest.read_text())
        assert data["package"]["name"] == "rsille-typst"
        assert data["package"]["version"] == version
        assert set(source.rglob("Cargo.toml")) == {manifest}, \
            "archive must contain only the extension manifest"
        assert "patch" not in data and "replace" not in data
        dependency_tables = [data, *data.get("target", {}).values()]
        for table in dependency_tables:
            for section in ("dependencies", "dev-dependencies", "build-dependencies"):
                for name, dependency in table.get(section, {}).items():
                    assert "path" not in dependency and "git" not in dependency, (name, dependency)
        assert data["dependencies"]["rsille"].get("version"), \
            "the archive must use a versioned registry dependency on rsille"
        for filename in ("README.md", "README_cn.md", "LICENCE", "src/lib.rs",
                         "tests/render.rs", "examples/formula.rs"):
            assert (source / filename).is_file(), filename

        build_target = str(target / "release-typst-check-build")
        # Archive timestamps are normalized; invalidate this package to avoid
        # accidentally reusing an older build of the same release version.
        cargo("clean", "-p", "rsille-typst", "--target-dir", build_target, cwd=source)
        cargo("test", "--locked", "--target-dir", build_target, cwd=source)
        cargo("test", "--all-targets", "--locked", "--target-dir", build_target, cwd=source)
        for options in (("--headless",), ("--headless", "--cell-pixels", "10x20")):
            cargo("run", "--example", "formula", "--locked", "--target-dir", build_target,
                  cwd=source, program_args=options)
    print("rsille-typst archive, extracted tests and headless demos passed against registry dependencies; nothing uploaded.")


if __name__ == "__main__":
    main()
