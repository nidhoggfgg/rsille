#!/usr/bin/env python3
"""Unix black-box tests, no third-party packages. Build rsille before running.

python3 packages/rsille-cli/tests/pty_test.py --binary target/debug/rsille
stdin is a real pipe; stdout/stderr are separate regular files, never the PTY.
"""
import argparse
import json
import os
from pathlib import Path
import re
import select
import signal
import struct
import subprocess
import tempfile
import time
import unittest

if os.name != "posix":
    raise SystemExit("These PTY tests require Unix")
import fcntl
import pty
import termios

BINARY = None
EXAMPLES = Path(__file__).resolve().parents[1] / "examples"


class CliPtyTests(unittest.TestCase):
    def session(self, args, keys=b"\r", data=b"", expected=0, resize=False, inline=False, static_content=None):
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
                if static_content is None:
                    deadline = time.monotonic() + 5
                    ready = b"\x1b[2K" if inline else b"\x1b[?1049h"
                    while ready not in screen or termios.tcgetattr(slave)[3] & termios.ICANON:
                        drain()
                        self.assertIsNone(process.poll(), "exited before entering interactive mode")
                        self.assertLess(time.monotonic(), deadline, "no interactive/raw mode")
                    # Drain the first frame so a full PTY buffer cannot block input setup.
                    settle = time.monotonic() + 0.15
                    while time.monotonic() < settle:
                        drain()
                else:
                    self.assertTrue(inline)
                    self.assertIsNone(keys, "static displays must exit without any keypress")
                    self.assertFalse(resize)
                if resize:
                    screen.clear()
                    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
                    os.kill(process.pid, signal.SIGWINCH)
                    deadline = time.monotonic() + 3
                    resized = b"\x1b[100G" if inline else b"\x1b[30;100H"
                    while resized not in screen:
                        drain()
                        self.assertLess(time.monotonic(), deadline, "resize did not redraw new bounds")
                if keys is not None:
                    os.write(master, keys)
                deadline = time.monotonic() + 5
                while process.poll() is None:
                    drain()
                    self.assertLess(time.monotonic(), deadline, "session did not exit")
                while select.select([master], [], [], 0)[0]:
                    drain(0)
                out.seek(0)
                err.seek(0)
                stdout, stderr = out.read(), err.read()
                self.assertEqual(process.returncode, expected, stderr.decode(errors="replace"))
                self.assertEqual(termios.tcgetattr(slave), before, "termios not restored exactly")
                if inline:
                    for sequence in (b"\x1b[?1049h", b"\x1b[?1049l", b"\x1b[2J"):
                        self.assertNotIn(sequence, screen)
                else:
                    self.assertIn(b"\x1b[?1049l", screen)
                for sequence in (b"\x1b[?25h", b"\x1b[?2004l", b"\x1b[?7h"):
                    self.assertIn(sequence, screen, "missing terminal cleanup")
                self.assertEqual(stderr, b"")
                if static_content is not None:
                    self.assertEqual(stdout, b"", "static output must not emit a JSON result")
                    # Text cells have cursor/style sequences between characters.
                    # Allow per-row erasure during drawing, but never after the
                    # last visible text or Sixel payload (terminal cleanup).
                    visible = bytearray()
                    last_draw_end = 0
                    first_graphics_end = None
                    tokens = re.finditer(
                        rb"\x1bP.*?\x1b\\|\x1b\[[0-?]*[ -/]*[@-~]|\x1b[ -/]*[0-~]|[^\x1b]",
                        bytes(screen), re.DOTALL,
                    )
                    for token in tokens:
                        value = token.group()
                        if value.startswith(b"\x1bP"):
                            self.assertRegex(value, rb"^\x1bP[0-9;]*q", "expected Sixel DCS")
                            visible.extend(b"\x1bP")
                            if first_graphics_end is None:
                                first_graphics_end = token.end()
                            last_draw_end = token.end()
                        elif not value.startswith(b"\x1b"):
                            visible.extend(value)
                            if value >= b" " and value != b"\x7f" and value.strip():
                                last_draw_end = token.end()
                    if static_content == b"\x1bP":
                        # A single frame contains multiple independently encoded tiles.
                        self.assertIsNotNone(first_graphics_end, "formula emitted no Sixel")
                    else:
                        self.assertEqual(visible.count(static_content), 1,
                                         "expected static text exactly once: " + repr(visible))
                    if first_graphics_end is not None:
                        self.assertIsNone(
                            re.search(rb"\x1b\[[0-9;?]*[JK]|\x1bc", bytes(screen[first_graphics_end:])),
                            "graphics were erased after drawing started",
                        )
                    self.assertIsNone(
                        re.search(rb"\x1b\[[0-9;?]*[JK]|\x1bc", bytes(screen[last_draw_end:])),
                        "static cleanup erased already drawn content",
                    )
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
        output = self.session(["--fullscreen", *args], keys, data, resize=resize)
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
                self.assertEqual(self.session(["--fullscreen", "--output", "plain", *args], keys), expected)

    def test_escape_ctrl_c_and_cancel_button(self):
        for args, keys in [(["input"], b"\x1b"), (["input"], b"\x03"), (["confirm"], b"\t\r"), (["--output", "plain", "confirm"], b"\x03")]:
            with self.subTest(args=args, keys=keys):
                self.session(["--fullscreen", *args], keys, expected=130)

    def test_run_with_pipe_data_and_empty_selection(self):
        # The example has table -> submit -> cancel, so Tab submits without selecting.
        args = ["run", str(EXAMPLES / "deploy.yaml"), "--data", "-"]
        data = (EXAMPLES / "services.json").read_bytes()
        self.assertEqual(self.result(args, b"\t\r", data), {"services": []})

    def test_inline_prompts_and_cleanup(self):
        for args, keys, expected in [
            (["input", "--inline"], b"hello\r", {"value": "hello"}),
            (["--inline", "select", "dev", "prod"], b"\x1b[B\r", {"value": "prod"}),
            (["confirm", "--inline"], b"\r", {"value": True}),
            (["table", "--inline", "--selectable"], b"\r", {"value": [{"name": "api"}]}),
        ]:
            with self.subTest(args=args):
                output = self.session(args, keys, data=b'[{"name":"api"}]', inline=True)
                self.assertEqual(json.loads(output)["values"], expected)
        for keys in (b"\x1b", b"\x03"):
            self.session(["input", "--inline"], keys, expected=130, inline=True)
        self.assertEqual(self.session(["input", "--inline", "--output", "plain"], b"ok\r", inline=True, resize=True), b"ok\n")

    def test_default_inline_interaction(self):
        for args, keys, expected in [
            (["input"], b"hello\r", {"value": "hello"}),
            (["select", "dev", "prod"], b"\x1b[B\r", {"value": "prod"}),
            (["confirm"], b"\r", {"value": True}),
            (["table", "--selectable"], b"\r", {"value": [{"name": "api"}]}),
        ]:
            with self.subTest(args=args):
                output = self.session(args, keys, data=b'[{"name":"api"}]', inline=True)
                self.assertEqual(json.loads(output), {"action": "submit", "values": expected})
        for keys in (b"\x1b", b"\x03"):
            self.session(["input"], keys, expected=130, inline=True)
        self.assertEqual(self.session(["input", "--output", "plain"], b"ok\r", inline=True, resize=True), b"ok\n")

    def test_static_formula_exits_and_preserves_graphics(self):
        # The PTY has no pixel metrics or terminal query responder.
        self.session(["formula", "x^2", "--cell-pixels", "8x18"],
                     keys=None, inline=True, static_content=b"\x1bP")

    def test_fullscreen_formula_remains_interactive(self):
        self.assertEqual(self.result(["formula", "x^2", "--cell-pixels", "8x18"]), {})

    def test_static_readonly_table_exits_and_preserves_text(self):
        self.session(["table"], keys=None, data=b'[{"name":"kept-row"}]',
                     inline=True, static_content=b"kept-row")

    def test_static_yaml_exits_and_preserves_content(self):
        layouts = [
            {"text": "retained-static-text"},
            {"column": [
                {"text": "retained-static-text"},
                {"row": [
                    {"column": [{"formula": {"source": "x^2"}}]},
                    {"table": {"rows": [{"name": "nested-row"}], "selectable": False}},
                ]},
            ]},
        ]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "static.yaml"
            for layout in layouts:
                with self.subTest(layout=layout):
                    # JSON is also valid YAML; nesting exercises recursive classification.
                    path.write_text(json.dumps({"version": 1, "layout": layout}))
                    self.session(["run", str(path), "--cell-pixels", "8x18"],
                                 keys=None, inline=True, static_content=b"retained-static-text")

    def test_inline_yaml(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "inline.yaml"
            path.write_text("version: 1\nmode: inline\nlayout: {input: {id: name, label: Name}}\n")
            output = self.session(["run", str(path)], b"ok\r", inline=True)
            self.assertEqual(json.loads(output)["values"], {"name": "ok"})
            self.assertEqual(self.result(["run", str(path)], b"ok\r"), {"name": "ok"})
            path.write_text("version: 1\nmode: fullscreen\nlayout: {input: {id: name, label: Name}}\n")
            output = self.session(["--inline", "run", str(path)], b"ok\r", inline=True)
            self.assertEqual(json.loads(output)["values"], {"name": "ok"})

    def test_static_without_controlling_terminal_fails_fast(self):
        for args, data in [
            (["formula", "x^2", "--cell-pixels", "8x18"], b""),
            (["table"], b'[{"name":"api"}]'),
        ]:
            with self.subTest(args=args):
                result = subprocess.run([BINARY, *args], input=data, capture_output=True,
                                        start_new_session=True, timeout=5)
                self.assertEqual(result.returncode, 1)
                self.assertEqual(result.stdout, b"")
                self.assertIn(b"terminal", result.stderr)

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
