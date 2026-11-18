"""Bounded macOS GUI smoke tests. Run cargo build, then python3 scripts/check_mock.py."""
import json
import os
from pathlib import Path
import selectors
import signal
import subprocess
import time

BINARY = Path(__file__).resolve().parents[1] / "target/debug/nord-modular-ni-interface"


class Session:
    def __init__(self, args):
        if "--state-file" not in args and "--no-persist" not in args:
            args = [*args, "--no-persist"]
        self.process = subprocess.Popen(
            [str(BINARY), *args], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT, bufsize=0,
            env={**os.environ, 'MK3_HIDDEN_PREVIEW':'1'},
        )
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ)
        self.pending = b""
        self.lines = []

    def wait_for(self, predicate, timeout=12):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            for key, _ in self.selector.select(0.1):
                chunk = key.fileobj.read(65536)
                if not chunk:
                    raise RuntimeError(f"Process exited early: {self.lines[-8:]}")
                self.pending += chunk
                while b"\n" in self.pending:
                    line, self.pending = self.pending.split(b"\n", 1)
                    self.lines.append(line.decode(errors="replace"))
            if predicate():
                return
        raise TimeoutError(f"Timed out: {self.lines[-8:]}")

    def command(self, text):
        self.process.stdin.write((text + "\n").encode())
        self.process.stdin.flush()

    def state(self):
        for line in reversed(self.lines):
            if line.startswith("state: "):
                return json.loads(line[7:])
        return {}

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait()
            print("Stopped unfinished test process")
        self.selector.close()


def check(args, exit_method):
    session = Session(args)
    try:
        session.wait_for(lambda: bool(session.state()))
        if "--hardware-test" not in args:
            assert session.state()["engine"] == "G1 MOCK / NO ENGINE"
            session.command("set filter cutoff 2000")
            session.wait_for(lambda: session.state()["parameters"][0]["value"] == 2000)
            session.command("inspect")
            session.wait_for(lambda: sum(line.startswith("mock DOM") for line in session.lines) == 2)
            right = json.loads(next(line.split(": ", 1)[1] for line in session.lines if line.startswith("mock DOM right")))
            assert "CUTOFF" in right["text"] and "2.00 kHz" in right["text"], right
            assert all(knob["bottom"] <= 240 for knob in right["knobs"]), right
            print("Typed controller fixture and rendered parameter units PASS")
        if "--hardware-test" in args:
            session.command("turn 8 0.1\nturn 8 -0.1\npad 16\nbutton top_8\nbutton group_h")
            session.wait_for(lambda: session.state().get("groups", [0] * 8)[7] == 1)
            original = session.state()
            assert original["encoders"][7] == 2
            assert original["positive"][7] and original["negative"][7]
            for command, status in [
                ("disconnect\nturn 8 1", "WAITING FOR MK3"),
                ("reconnect", "ACTIVE"),
                ("handover\npad 16", "OTHER APP HAS CONTROL"),
                ("takeover", "ACTIVE"),
            ]:
                previous_lines = len(session.lines)
                session.command(command)
                session.wait_for(lambda: len(session.lines) > previous_lines
                                 and session.state()["connection"]["status"] == status)
                assert session.state()["encoders"] == original["encoders"]
                assert session.state()["pads"] == original["pads"]
                assert session.state()["groups"] == original["groups"]
            assert session.state()["connection"]["reconnects"] == 1
            assert session.state()["connection"]["handovers"] == 1
            session.command("inspect")
            session.wait_for(lambda: sum(line.startswith("mock DOM") for line in session.lines) == 2)
            for line in session.lines:
                if not line.startswith("mock DOM"):
                    continue
                side, raw = line.split(": ", 1)
                dom = json.loads(raw)
                assert dom["width"] == 480 and dom["height"] == 272, dom
                assert dom["bridge"] == "function", dom
                assert "SIMULATED INPUT" in dom["text"], dom
                assert "ACTIVE · RETURNS 1" in dom["text"], dom
                assert all(cell["bottom"] <= 230 for cell in dom["cells"]), dom
                if "left" in side:
                    assert "E8 · 2" in dom["text"], dom
                else:
                    assert "P16 · 1" in dom["text"] and "T8:1" in dom["text"], dom
                    assert "GH:1" in dom["text"], dom
            print("Simulated reconnect/handover preserves counters, blocks inactive input; DOM/layout PASS")
        if exit_method == "quit":
            session.command("quit")
        else:
            session.process.send_signal(signal.SIGINT)
        assert session.process.wait(timeout=5) == 0
        print(f"{' '.join(args)} / {exit_method}: exit 0 PASS")
    finally:
        session.close()


if __name__ == "__main__":
    check(["--mock"], "quit")
    check(["--mock", "--hardware-test"], "quit")
    check(["--mock", "--hardware-test"], "sigint")
