#!/usr/bin/env python3
"""Verify automatic Sixel cell metrics and input ownership using a Unix PTY.

This tests protocol transport, not a real terminal's Sixel visual rendering.
Run from any directory; --binary skips the debug example build.
"""

import argparse
import errno
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import signal
import struct
import subprocess
import sys
import termios
import time

ROOT = Path(__file__).resolve().parents[1]
QUERY = b"\x1b[16t"
FRAME_END = b"\x1b[?2026l"
TILE = b'\x1bP0;1;0q"1;1;10;20'


def check(binary, mode):
    master, slave = pty.openpty()
    pixels = (1100, 720) if mode == "os" else (0, 0)
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 36, 110, *pixels))
    screen = bytearray()
    output_master, output_slave = pty.openpty() if mode == "mismatched-tty" else (master, slave)

    def attach_tty():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)

    # Inspect restoration before the controlling session exits (also on macOS).
    wrapper = """
import subprocess, sys, termios
original = termios.tcgetattr(0)
child = subprocess.run(sys.argv[1:])
restored = termios.tcgetattr(0) == original
print(f"RSILLE_TTY_RESTORED={restored}", flush=True)
sys.exit(child.returncode if restored else 1)
"""
    args = [str(binary), "--sixel"]
    if mode == "explicit":
        args += ["--cell-pixels", "10x20"]
    process = subprocess.Popen(
        [sys.executable, "-c", wrapper, *args],
        stdin=slave, stdout=output_slave, stderr=output_slave,
        preexec_fn=attach_tty, cwd=ROOT,
    )

    def pump():
        if select.select([output_master], [], [], 0.02)[0]:
            try:
                data = os.read(output_master, 65536)
                screen.extend(data)
                assert len(screen) < 8 * 1024 * 1024, "unbounded output"
                return bool(data)
            except OSError as error:
                if error.errno != errno.EIO:
                    raise
        return False

    def wait_until(predicate, description, timeout=5):
        deadline = time.monotonic() + timeout
        while not predicate():
            pump()
            if time.monotonic() >= deadline:
                raise AssertionError(
                    f"{mode}: timed out waiting for {description}; "
                    f"output suffix: {bytes(screen[-1200:])!r}"
                )

    def plain_text():
        data = re.sub(rb"\x1bP.*?\x1b\\", b"", bytes(screen), flags=re.S)
        return re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", data)

    try:
        if mode in ("query", "resize", "timeout", "invalid"):
            wait_until(lambda: QUERY in screen, "cell-size query")
            if mode in ("query", "resize"):
                # Ordinary input before/during the reply must survive. A pasted
                # reply-shaped string must not be mistaken for a measurement.
                os.write(master, b"e\x1b[200~\x1b[6;88;99t\x1b[201~")
                os.write(master, b"\x1b[<0;110;36M\x1b[<0;110;36m")
                for fragment in (b"\x1b[6;", b"20;", b"10t"):
                    os.write(master, fragment)
                    time.sleep(0.005)
            elif mode == "invalid":
                os.write(master, b"\x1b[6;0;10t")

        if mode in ("timeout", "invalid", "mismatched-tty"):
            wait_until(lambda: process.poll() is not None, "bounded failure", timeout=3)
            while pump():
                pass
            assert process.returncode != 0
            assert TILE not in screen
            assert b"CSI 16 t query" in screen
            assert b"RSILLE_TTY_RESTORED=True" in screen
            assert screen.count(QUERY) == (0 if mode == "mismatched-tty" else 1)
            if mode == "mismatched-tty":
                assert b"same controlling terminal" in screen
                assert not select.select([master], [], [], 0.05)[0], "queried the unrelated terminal"
            print(f"PASS {mode}: actionable error and raw-mode restoration")
            return

        wait_until(lambda: FRAME_END in screen, "first complete frame")
        assert TILE in screen, "must render actual 10x20 tiles without rounding"
        if mode in ("query", "resize"):
            assert b"Braille" in plain_text(), "preflight key was swallowed"
        expected_queries = 1 if mode in ("query", "resize") else 0
        assert screen.count(QUERY) == expected_queries

        # Another input-driven frame must reuse measurements, not probe again.
        frames = screen.count(FRAME_END)
        os.write(master, b"e")
        wait_until(lambda: screen.count(FRAME_END) > frames, "input after query")
        assert screen.count(QUERY) == expected_queries

        if mode == "resize":
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 32, 100, 0, 0))
            wait_until(lambda: screen.count(QUERY) == 2, "query after grid change")
            os.write(master, b"\x1b[6;24;12t")
            wait_until(lambda: b'\x1bP0;1;0q"1;1;12;24' in screen, "resized tiles")
            expected_queries = 2

        os.write(master, b"q")
        wait_until(lambda: process.poll() is not None, "clean keyboard exit")
        while pump():
            pass
        assert process.returncode == 0, bytes(screen[-1200:])
        assert screen.count(QUERY) == expected_queries
        assert b"RSILLE_TTY_RESTORED=True" in screen
        assert b"\x1b[?1049l" in screen, "alternate screen was not restored"
        print(f"PASS {mode}: metrics, input, caching and terminal restoration")
    finally:
        if process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        process.wait(timeout=5)
        os.close(master)
        os.close(slave)
        if output_master != master:
            os.close(output_master)
            os.close(output_slave)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, help="Use an already built sixel_windows example")
    args = parser.parse_args()
    binary = args.binary
    if binary is None:
        subprocess.run(
            ["cargo", "build", "--features", "sixel", "--example", "sixel_windows"],
            cwd=ROOT, check=True, timeout=120,
        )
        binary = ROOT / "target/debug/examples/sixel_windows"
    for mode in ("explicit", "os", "query", "resize", "timeout", "invalid", "mismatched-tty"):
        check(binary.resolve(), mode)


if __name__ == "__main__":
    main()
