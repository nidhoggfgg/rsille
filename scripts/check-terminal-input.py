#!/usr/bin/env python3
"""Exercise the Minecraft terminal adapter with a small Unix PTY emulator.

This checks protocol negotiation, crossterm decoding and terminal restoration.
It does not replace physical-key testing in an actual terminal emulator.
"""

import argparse
import errno
import fcntl
import json
import os
from pathlib import Path
import platform
import pty
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time


ROOT = Path(__file__).resolve().parents[1]


def check(binary, output, mode):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
    log = output / f"{mode}.log"
    screen = bytearray()

    def attach_tty():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)

    # Keep the controlling session alive until termios is inspected. macOS
    # revokes the PTY when its session leader exits, even if we retain the slave.
    wrapper = """
import subprocess, sys, termios
original = termios.tcgetattr(0)
child = subprocess.run(sys.argv[1:])
restored = termios.tcgetattr(0) == original
print(f"RSILLE_TTY_RESTORED={restored}", flush=True)
sys.exit(child.returncode if restored else 1)
"""
    process = subprocess.Popen(
        [sys.executable, "-c", wrapper, str(binary), "--input-log", str(log)],
        stdin=slave, stdout=slave, stderr=slave,
        preexec_fn=attach_tty, cwd=ROOT,
    )

    def pump():
        if select.select([master], [], [], 0.02)[0]:
            try:
                data = os.read(master, 65536)
                screen.extend(data)
                return bool(data)
            except OSError as error:
                if error.errno != errno.EIO:
                    raise
        return False

    def wait_until(predicate, description, timeout=8):
        deadline = time.monotonic() + timeout
        while not predicate():
            pump()
            if time.monotonic() >= deadline:
                raise AssertionError(f"{mode}: timed out waiting for {description}")

    def text():
        return log.read_text() if log.exists() else ""

    def send(data):
        os.write(master, data)

    try:
        wait_until(lambda: b"\x1b[?u\x1b[c" in screen, "keyboard query")
        if mode == "enhanced":
            send(b"\x1b[?0u\x1b[?1;2c")
        elif mode == "legacy":
            send(b"\x1b[?1;2c")
        # In timeout mode the terminal deliberately does not answer.
        wait_until(lambda: "\n" in text(), "capability notification")
        assert f"key_releases: {str(mode == 'enhanced').lower()}" in text()
        assert "focus_events: true" in text()
        wait_until(lambda: b"\x1b[?1004h" in screen, "focus reporting setup")
        if mode == "enhanced":
            assert b"\x1b[>11u" in screen
            send(b"\x1b[119;1:1u\x1b[1;1:1C")
            send(b"\x1b[119;1:2u\x1b[1;1:2C")
            send(b"\x1b[1;1:3C\x1b[119;1:3u")
            wait_until(lambda: text().count("kind: Release") >= 2, "key releases")
            for code in ["Char('w')", "Right"]:
                for kind in ["Press", "Repeat", "Release"]:
                    assert any(f"code: {code}," in line and f"kind: {kind}," in line
                               for line in text().splitlines()), (code, kind)
        else:
            assert b"\x1b[>11u" not in screen
            send(b"wd\x1b[C")
            wait_until(lambda: "code: Right," in text(), "legacy keys")
        send(b"\x1b[O\x1b[I\x1b[200~hello\x1b[201~")
        wait_until(lambda: 'Paste("hello")' in text(), "focus and paste")
        assert "FocusLost" in text() and "FocusGained" in text()
        if mode == "enhanced":
            send(b"\x1b[113;1:3u")
            wait_until(lambda: "code: Char('q')" in text(), "quit release")
            deadline = time.monotonic() + 0.1
            while time.monotonic() < deadline:
                pump()
            assert process.poll() is None, "releasing Q must not quit"
            send(b"\x1b[113;1:1u")
        else:
            send(b"q")
        wait_until(lambda: process.poll() is not None, "clean exit")
        # Drain the final cursor, protocol and screen restoration commands.
        while pump():
            pass
        assert process.returncode == 0, process.returncode
        assert b"RSILLE_TTY_RESTORED=True" in screen, "raw mode was not restored"
        assert b"\x1b[?1004l" in screen, "focus reporting was not restored"
        assert b"\x1b[?2004l" in screen, "paste reporting was not restored"
        assert b"\x1b[?25h" in screen, "cursor was not restored"
        leave = screen.rfind(b"\x1b[?1049l")
        assert leave >= 0, "alternate screen was not restored"
        if mode == "enhanced":
            pop = screen.rfind(b"\x1b[<1u")
            assert 0 <= pop < leave, "keyboard mode must pop on its original screen"
        else:
            assert b"\x1b[<1u" not in screen
        print(f"PASS {mode}: negotiation, decoding, focus, paste, quit and restoration")
    finally:
        if process.poll() is None:
            import signal
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        (output / f"{mode}.ansi").write_bytes(screen)
        os.close(master)
        os.close(slave)
        process.wait(timeout=5)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, help="Use an already built Minecraft binary")
    parser.add_argument("--output-dir", type=Path, help="Keep decoded logs and terminal output")
    args = parser.parse_args()
    binary = args.binary
    if binary is None:
        manifest = str(ROOT / "examples/minecraft/Cargo.toml")
        subprocess.run(["cargo", "build", "--manifest-path", manifest], check=True)
        metadata = json.loads(subprocess.check_output([
            "cargo", "metadata", "--no-deps", "--format-version", "1",
            "--manifest-path", manifest,
        ]))
        binary = Path(metadata["target_directory"]) / "debug/rsille-minecraft"
    print(f"PTY transport verification: {platform.platform()}")
    with tempfile.TemporaryDirectory(prefix="rsille-input-") as temporary:
        output = args.output_dir or Path(temporary)
        output.mkdir(parents=True, exist_ok=True)
        for mode in ["enhanced", "legacy", "timeout"]:
            check(binary.resolve(), output, mode)


if __name__ == "__main__":
    main()
