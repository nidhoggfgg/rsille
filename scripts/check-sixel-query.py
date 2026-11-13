#!/usr/bin/env python3
"""Verify startup-only Sixel pixel probing and native input using a Unix PTY.

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
TILE = b'\x1bP7;1;0q"1;1;10;20'


def check(binary, mode, formula=None):
    master, slave = pty.openpty()
    os.set_blocking(master, False)
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
    args = [str(binary)]
    if mode == "explicit":
        args += ["--cell-pixels", "10x20"]
    if formula:
        args += ["--", formula]
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
                # Inline startup asks for the cursor origin. Start at the bottom
                # to exercise reservation/scrolling before painting graphics.
                if b"\x1b[6n" in screen and not getattr(pump, "replied", False):
                    os.write(master, b"\x1b[36;1R")
                    pump.replied = True
                assert len(screen) < 8 * 1024 * 1024, "unbounded output"
                return bool(data)
            except OSError as error:
                if error.errno != errno.EIO:
                    raise
        return False

    def send_input(data):
        deadline = time.monotonic() + 1
        while data:
            assert time.monotonic() < deadline, f"{mode}: terminal input write timed out"
            if select.select([], [master], [], 0.02)[1]:
                try:
                    written = os.write(master, data)
                    assert written > 0, f"{mode}: terminal input disconnected"
                    data = data[written:]
                except BlockingIOError:
                    pass

    def wait_until(predicate, description, timeout=5, noise=False):
        deadline = time.monotonic() + timeout
        while not predicate():
            pump()
            if noise and process.poll() is None:
                # Keep input arriving well inside the probe's 500 ms deadline.
                # A stalled/finished reader must never block this test driver.
                try:
                    os.write(master, b"x" * 256)
                except BlockingIOError:
                    pass
            if time.monotonic() >= deadline:
                raise AssertionError(
                    f"{mode}: timed out waiting for {description}; "
                    f"output suffix: {bytes(screen[-1200:])!r}"
                )

    def plain_text(start=0):
        data = re.sub(rb"\x1bP.*?\x1b\\", b"", bytes(screen[start:]), flags=re.S)
        data = re.sub(rb"\x1b[78]", b"", data)
        return re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", data)

    try:
        if mode in ("query", "resize", "timeout", "noise", "invalid"):
            wait_until(lambda: QUERY in screen, "startup cell-size query", timeout=3)
            if mode in ("query", "resize"):
                # The startup scan may consume ordinary input. Only reply
                # matching is tested here, not key/paste preservation.
                for fragment in (b"xyz", b"\x1b[6;", b"20;", b"10t"):
                    send_input(fragment)
                    time.sleep(0.005)
            elif mode == "invalid":
                send_input(b"\x1b[6;0;10t\x1b[6;20;0t\x1b[6;65536;10t\x1b[6;20;t")

        if mode in ("timeout", "noise", "invalid", "mismatched-tty"):
            wait_until(
                lambda: process.poll() is not None,
                "bounded startup failure", timeout=3, noise=mode == "noise",
            )
            while pump():
                pass
            assert process.returncode != 0
            assert TILE not in screen
            assert FRAME_END not in screen, "failed startup must not render a frame"
            assert b"CSI 16 t query" in screen
            assert b"RSILLE_TTY_RESTORED=True" in screen
            assert screen.count(QUERY) == (0 if mode == "mismatched-tty" else 1)
            if mode == "mismatched-tty":
                assert b"same controlling terminal" in screen
                assert not select.select([master], [], [], 0.05)[0], "queried the unrelated terminal"
            else:
                assert b"timed out" in screen, "invalid/noisy input must not complete the probe"
            print(f"PASS {mode}: bounded startup failure and raw-mode restoration")
            return

        wait_until(lambda: FRAME_END in screen, "first complete frame")
        assert TILE in screen, "must render actual 10x20 tiles without rounding"
        expected_queries = 1 if mode in ("query", "resize") else 0
        assert screen.count(QUERY) == expected_queries

        # Only keys sent after startup belong to native Crossterm input. Verify
        # the state change as well as a frame, so an unrelated redraw cannot pass.
        # HalfBlock -> Braille changes the first four letters; differential
        # output may omit an unchanged 'l', so do not require the entire word.
        if formula is None:
            start = len(screen)
            send_input(b"e")
            wait_until(
                lambda: FRAME_END in screen[start:] and b"Brai" in plain_text(start),
                "native key event after the initial frame",
            )
            assert screen.count(QUERY) == expected_queries
        else:
            assert b"Typst / Sixel" in plain_text()
            check_formula_frame(bytes(screen), 36, 110)

        if mode == "resize":
            start = len(screen)
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 6 if formula is not None else 32, 12 if formula is not None else 100, 0, 0))
            # No OS pixels and no second reply: the renderer must retain the
            # explicit 10x20 snapshot returned by startup preflight.
            wait_until(
                lambda: FRAME_END in screen[start:] and TILE in screen[start:],
                "resize redraw with the original 10x20 pixel snapshot",
            )
            assert screen.count(QUERY) == expected_queries, "resize must not query cell pixels"
            if formula is not None:
                check_formula_frame(bytes(screen[start:]), 6, 12)

        send_input(b"\x1b" if formula is not None and mode == "explicit" else b"q")
        wait_until(lambda: process.poll() is not None, "clean keyboard exit")
        while pump():
            pass
        assert process.returncode == 0, bytes(screen[-1200:])
        assert screen.count(QUERY) == expected_queries
        assert b"RSILLE_TTY_RESTORED=True" in screen
        if formula is None:
            assert b"\x1b[?1049l" in screen, "alternate screen was not restored"
        else:
            for forbidden in (b"\x1b[?1049h", b"\x1b[?1049l", b"\x1b[?1000h", b"\x1b[?1002h", b"\x1b[?1003h", b"\x1b[2J"):
                assert forbidden not in screen, ("not normal-screen keyboard-only output", forbidden)
            assert b"\x1b[?25h" in screen, "cursor was not restored"
        print(f"PASS {mode}: startup snapshot, native input and terminal restoration")
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


def check_formula_frame(data, rows, columns):
    # Inline tiles are positioned relative to the saved bottom guard anchor.
    # At 10x20 every tile needs 24 physical scanlines, so at least one row
    # below every tile is required; no fullscreen bottom-prefix workaround.
    data = data.split(FRAME_END, 1)[0]
    tiles = re.findall(
        rb'\x1b8\x1b\[(\d+)F\x1b\[(\d+)G\x1bP7;1;0q"1;1;10;(\d+).*?\x1b\\',
        data, flags=re.S,
    )
    assert len(tiles) == data.count(b"\x1bP"), "unpositioned or malformed DCS"
    assert tiles, "formula did not reach terminal encoder"
    for above_guard, column, height in tiles:
        above_guard, column, height = int(above_guard), int(column), int(height)
        assert 1 <= above_guard < rows and 1 <= column <= columns
        assert height == 20, "inline must retain original cell metrics"
        assert ((height + 5) // 6) * 6 <= (above_guard + 1) * 20
    content_rows = max(int(up) for up, _, _ in tiles) + 1  # includes title
    assert content_rows + 1 <= rows, "missing safe bottom guard"
    if rows == 36:
        assert content_rows < 35, "formula expanded to the whole viewport"
        assert data.count(b"\n") <= content_rows + 1, "excessive row reservation"
    assert b"\x1b[2K" in data, "missing local graphics erasure"
    print(f"PASS formula bytes: {len(tiles)} relative DCS, {content_rows} content rows + guard, viewport {columns}x{rows}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, help="Use an already built example (sixel_windows, or formula with --formula)")
    parser.add_argument(
        "--formula", nargs="?", const="",
        help="Check formula example instead (optional quoted Typst source; default formula if omitted)",
    )
    args = parser.parse_args()
    binary = args.binary
    if binary is None:
        example = "sixel_windows" if args.formula is None else "formula"
        package = [] if args.formula is None else ["-p", "rsille-typst"]
        subprocess.run(
            ["cargo", "build", *package, "--example", example],
            cwd=ROOT, check=True, timeout=120,
        )
        binary = ROOT / "target/debug/examples" / example
    for mode in ("explicit", "os", "query", "resize", "timeout", "noise", "invalid", "mismatched-tty"):
        check(binary.resolve(), mode, args.formula)


if __name__ == "__main__":
    main()
