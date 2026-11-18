"""Real-host launcher attachment/preferences/duplicate/exit check, isolated state, no USB."""
import json
from pathlib import Path
import selectors
import signal
import socket
import subprocess
import sys
import time

from run_machine import ROOT, wait_ready


class Session:
    def __init__(self, args):
        self.process = subprocess.Popen([sys.executable, '-B', str(ROOT / 'scripts/run_machine.py'), *args],
                                        cwd=ROOT, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, bufsize=0)
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ)
        self.pending = b''
        self.lines = []

    def live(self):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            for key, _ in self.selector.select(.1):
                chunk = key.fileobj.read(65536)
                if not chunk:
                    raise RuntimeError('Launcher exited early: ' + '\n'.join(self.lines[-10:]))
                self.pending += chunk
                while b'\n' in self.pending:
                    line, self.pending = self.pending.split(b'\n', 1)
                    decoded = line.decode(errors='replace')
                    self.lines.append(decoded)
                    if decoded.startswith('state: ') and json.loads(decoded[7:]).get('engine') == 'G1-EMU / LIVE':
                        return
        raise RuntimeError('Controller did not become live: ' + '\n'.join(self.lines[-10:]))

    def stop(self, terminate=False):
        if terminate:
            self.process.send_signal(signal.SIGTERM)
        else:
            self.process.stdin.write(b'quit\n')
            self.process.stdin.flush()
        tail, _ = self.process.communicate(timeout=15)
        self.lines.extend(tail.decode(errors='replace').splitlines())
        assert self.process.returncode == 0, self.lines[-20:]
        assert any('Controller stopped.' in line for line in self.lines), self.lines[-10:]
        self.selector.close()

    def cleanup(self):
        if self.process.poll() is None:
            self.process.send_signal(signal.SIGTERM)
            try:
                self.process.wait(timeout=15)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
        self.selector.close()


def main():
    folder = ROOT / 'g1-state' / ('launcher-check-' + str(time.time_ns()))
    library = folder / 'Library with spaces'
    library.mkdir(parents=True)
    (library / 'Demo.pch').write_text((ROOT / 'patches/MK3-Bridge.pch').read_text())
    with socket.socket() as probe:
        probe.bind(('127.0.0.1', 0))
        port = probe.getsockname()[1]
    with (folder / 'host.log').open('w') as log:
        host = subprocess.Popen([str(ROOT / 'build/g1-emu/g1mk3check'), str(ROOT / 'roms/NORD-MODULAR-RACK-VER-3.03.BIN'),
                                 str(folder / 'flash.bin'), str(ROOT / 'patches/MK3-Bridge.pch'), str(port), '60'],
                                stdout=log, stderr=subprocess.STDOUT)
        sessions = []
        try:
            wait_ready(port, host)
            # Let the host finish its isolated initial patch upload and the probe retire.
            time.sleep(1)
            preferences = folder / 'preferences.json'
            args = ['--mock', '--no-build', '--preferences', str(preferences), '--port', str(port), '--patch-dir', str(library)]
            first = Session(args)
            sessions.append(first)
            first.live()
            duplicate = subprocess.run([sys.executable, '-B', str(ROOT / 'scripts/run_machine.py'), *args],
                                       capture_output=True, text=True, timeout=5)
            assert duplicate.returncode == 1 and 'already running through the launcher' in duplicate.stderr, duplicate
            first.stop()
            assert host.poll() is None
            saved = json.loads(preferences.read_text())
            assert saved['port'] == port and saved['patch_dir'] == str(library.resolve()), saved
            second = Session(['--mock', '--no-build', '--preferences', str(preferences)])
            sessions.append(second)
            second.live()
            second.stop(terminate=True)
            assert host.poll() is None
            print('PASS: real host attachment + live controller + duplicate rejection + remembered folder/port + quit/SIGTERM; host preserved')
        finally:
            for session in sessions:
                session.cleanup()
            if host.poll() is None:
                host.send_signal(signal.SIGTERM)
                try:
                    host.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    host.kill()
                    host.wait()
            if host.returncode != 0:
                raise RuntimeError('Real host did not finish normally: ' + str(folder / 'host.log'))
    print('Evidence:', folder)


if __name__ == '__main__':
    main()
