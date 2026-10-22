#!/usr/bin/env python3
"""Linux PTY smoke check for the workflow's sync and caller-owned Tokio entries."""
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import struct
import subprocess
import termios
import time


def check(binary, arguments):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
    initial = termios.tcgetattr(slave)

    def attach_terminal():
        os.setsid()
        fcntl.ioctl(slave, termios.TIOCSCTTY, 0)

    process = subprocess.Popen(
        [str(binary), *arguments], stdin=slave, stdout=slave, stderr=slave,
        preexec_fn=attach_terminal, env={**os.environ, "TERM": "xterm-256color"},
    )
    output = bytearray()
    paste_at = None
    sent_exit = False
    cursor_replies = 0
    started = time.monotonic()
    try:
        while time.monotonic() - started < 12:
            if select.select([master], [], [], 0.05)[0]:
                try:
                    block = os.read(master, 65536)
                except OSError:
                    break
                if not block:
                    break
                output.extend(block)
                queries = output.count(b"\x1b[6n")
                if queries > cursor_replies:
                    os.write(master, b"\x1b[1;1R" * (queries - cursor_replies))
                    cursor_replies = queries
                if paste_at is None and b"Exit" in re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", output):
                    os.write(master, b"\x1b[200~" + "中文👨‍👩‍👧‍👦".encode() + b"\x1b[201~")
                    paste_at = time.monotonic()
            if paste_at is not None and not sent_exit and time.monotonic() - paste_at > 0.8:
                os.write(master, b"\t\r")
                sent_exit = True
            if process.poll() is not None:
                # Drain the final state and restoration sequences after process exit.
                while select.select([master], [], [], 0)[0]:
                    output.extend(os.read(master, 65536))
                break
        assert process.poll() == 0, (arguments, "did not exit cleanly", output.decode(errors="replace"))
        assert "Exit: Requested; saved: 中文👨‍👩‍👧‍👦".encode() in output, output.decode(errors="replace")
        assert b"\x1b[?2004h" in output and b"\x1b[?2004l" in output, "bracketed paste cleanup"
        assert b"\x1b[?25h" in output, "cursor restoration"
        assert b"\x1b[?1049h" not in output, "inline mode entered the alternate screen"
        assert termios.tcgetattr(slave) == initial, "raw terminal settings were not restored"
    finally:
        if process.poll() is None:
            process.kill()
        process.wait()
        os.close(master)
        os.close(slave)
    print(f"PTY workflow passed: {arguments or ['sync']}")


def main():
    root = Path(__file__).resolve().parents[1]
    subprocess.run(["cargo", "build", "-p", "tui", "--example", "workflow"], cwd=root, check=True)
    import json
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=root))
    binary = Path(metadata["target_directory"]) / "debug/examples/workflow"
    check(binary, [])
    check(binary, ["--async"])


if __name__ == "__main__":
    main()
