#!/usr/bin/env python3
"""Unix black-box tests, no third-party packages. Build rsille before running.

python3 packages/rsille-cli/tests/pty_test.py --binary target/debug/rsille
stdin is a real pipe; stdout/stderr are separate regular files, never the PTY.
"""
import argparse
import json
import os
from pathlib import Path
import select
import signal
import struct
import subprocess
import tempfile
import time
import unittest

if os.name != "posix":
    raise SystemExit("PTY tests require Unix; Windows is unsupported")
import fcntl
import pty
import termios

BINARY = None
EXAMPLES = Path(__file__).resolve().parents[1] / "examples"


class CliPtyTests(unittest.TestCase):
    def session(self, args, keys=b"\r", data=b"", expected=0, resize=False):
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
        before = termios.tcgetattr(slave)
        screen = bytearray()
        process = None

        def drain(timeout=0.02):
            if select.select([master], [], [], timeout)[0]:
                screen.extend(os.read(master, 65536))

        def attach():
            os.setsid()
            fcntl.ioctl(slave, termios.TIOCSCTTY, 0)

        try:
            with tempfile.TemporaryFile() as out, tempfile.TemporaryFile() as err:
                process = subprocess.Popen(
                    [BINARY, *args], stdin=subprocess.PIPE, stdout=out, stderr=err,
                    pass_fds=(slave,), preexec_fn=attach,
                    env={**os.environ, "TERM": "xterm-256color"},
                )
                process.stdin.write(data)
                process.stdin.close()
                deadline = time.monotonic() + 5
                while b"\x1b[?1049h" not in screen or termios.tcgetattr(slave)[3] & termios.ICANON:
                    drain()
                    self.assertIsNone(process.poll(), "exited before entering fullscreen")
                    self.assertLess(time.monotonic(), deadline, "no fullscreen/raw mode")
                # Drain the first frame so a full PTY buffer cannot block input setup.
                settle = time.monotonic() + 0.15
                while time.monotonic() < settle:
                    drain()
                if resize:
                    screen.clear()
                    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
                    os.kill(process.pid, signal.SIGWINCH)
                    deadline = time.monotonic() + 3
                    while b"\x1b[30;100H" not in screen:
                        drain()
                        self.assertLess(time.monotonic(), deadline, "resize did not redraw new bounds")
                os.write(master, keys)
                deadline = time.monotonic() + 5
                while process.poll() is None:
                    drain()
                    self.assertLess(time.monotonic(), deadline, "keyboard interaction hung")
                while select.select([master], [], [], 0)[0]:
                    drain(0)
                out.seek(0)
                err.seek(0)
                stdout, stderr = out.read(), err.read()
                self.assertEqual(process.returncode, expected, stderr.decode(errors="replace"))
                self.assertEqual(termios.tcgetattr(slave), before, "termios not restored exactly")
                for sequence in (b"\x1b[?1049l", b"\x1b[?25h", b"\x1b[?2004l", b"\x1b[?7h"):
                    self.assertIn(sequence, screen, "missing terminal cleanup")
                self.assertEqual(stderr, b"")
                if expected == 130:
                    self.assertEqual(stdout, b"")
                return stdout
        finally:
            if process is not None:
                if process.poll() is None:
                    process.kill()
                process.wait(timeout=3)
                if process.stdin and not process.stdin.closed:
                    process.stdin.close()
            os.close(master)
            os.close(slave)

    def result(self, args, keys=b"\r", data=b"", resize=False):
        output = self.session(args, keys, data, resize=resize)
        self.assertTrue(output.endswith(b"\n"))
        self.assertEqual(output.count(b"\n"), 1)
        self.assertNotIn(b"\x1b", output)
        value = json.loads(output)
        self.assertEqual(set(value), {"action", "values"})
        self.assertEqual(value["action"], "submit")
        return value["values"]

    def test_pipe_table_returns_single_original_object_after_resize(self):
        rows = [{"name": "api", "hidden": {"n": 1}}, {"name": "工作器\u001b[31m", "hidden": [2, True]}]
        value = self.result(["table", "--selectable", "--columns", "name"], b"\x1b[B\r", json.dumps(rows).encode(), resize=True)
        self.assertEqual(value, {"value": [rows[1]]})

    def test_empty_table_and_view_only(self):
        self.assertEqual(self.result(["table", "--selectable"], data=b"[]"), {"value": []})
        self.assertEqual(self.result(["table"], data=b'[{"name":"api"}]'), {})

    def test_select_input_and_confirm_json(self):
        self.assertEqual(self.result(["select", "dev", "prod"], b"\x1b[B\r"), {"value": "prod"})
        self.assertEqual(self.result(["input"], "中文abc\x7f\r".encode()), {"value": "中文ab"})
        self.assertEqual(self.result(["confirm"]), {"value": True})

    def test_plain(self):
        for args, keys, expected in [
            (["select", "dev", "prod"], b"\x1b[B\r", b"prod\n"),
            (["input", "--value", "hello"], b"\r", b"hello\n"),
            (["confirm"], b"\r", b"true\n"),
        ]:
            with self.subTest(args=args):
                self.assertEqual(self.session(["--output", "plain", *args], keys), expected)

    def test_escape_ctrl_c_and_cancel_button(self):
        for args, keys in [(["input"], b"\x1b"), (["input"], b"\x03"), (["confirm"], b"\t\r"), (["--output", "plain", "confirm"], b"\x03")]:
            with self.subTest(args=args, keys=keys):
                self.session(args, keys, expected=130)

    def test_run_with_pipe_data_and_empty_selection(self):
        # The example has table -> submit -> cancel, so Tab submits without selecting.
        args = ["run", str(EXAMPLES / "deploy.yaml"), "--data", "-"]
        data = (EXAMPLES / "services.json").read_bytes()
        self.assertEqual(self.result(args, b"\t\r", data), {"services": []})

    def test_no_controlling_terminal_fails_fast(self):
        result = subprocess.run([BINARY, "input"], input=b"", capture_output=True, start_new_session=True, timeout=3)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, b"")
        self.assertIn(b"terminal", result.stderr)

    def test_config_error_does_not_change_termios(self):
        master, slave = pty.openpty()
        before = termios.tcgetattr(slave)
        try:
            with tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "bad.yaml"
                path.write_text("version: 99\nlayout: {text: bad}\n")
                def attach():
                    os.setsid()
                    fcntl.ioctl(slave, termios.TIOCSCTTY, 0)
                result = subprocess.run([BINARY, "run", str(path)], input=b"", capture_output=True, pass_fds=(slave,), preexec_fn=attach, timeout=3)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, b"")
                self.assertTrue(result.stderr)
                self.assertEqual(termios.tcgetattr(slave), before)
                self.assertFalse(select.select([master], [], [], 0)[0], "configuration errors must not enter terminal mode")
        finally:
            os.close(master)
            os.close(slave)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    args, remaining = parser.parse_known_args()
    BINARY = str(args.binary.resolve(strict=True))
    unittest.main(argv=[__file__, *remaining])
