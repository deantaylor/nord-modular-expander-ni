"""Real-ROM regression for editor-style encoder writes, audio and safety barriers.
Uses isolated state and the project-owned sine patch; never attaches to the user synth.
"""
import json
import os
from pathlib import Path
import signal
import socket
import statistics
import subprocess
import time

from check_g1 import ROOT, Rpc, analyze, wait


def main():
    folder = ROOT / 'g1-state' / ('encoder-path-' + str(time.time_ns()))
    folder.mkdir()
    patch = folder / 'Encoder.pch'
    original = (ROOT / 'patches/MK3-Bridge.pch').read_text()
    patch.write_text(original.replace('1 2 0 1\n[/KnobMapDump]', '1 2 0 1\n2 1 0 2\n[/KnobMapDump]'))
    with socket.socket() as probe:
        probe.bind(('127.0.0.1', 0))
        port = probe.getsockname()[1]
    log = (folder / 'host.log').open('w')
    host = subprocess.Popen(
        [str(ROOT / 'build/g1-emu/g1mk3check'), str(ROOT / 'roms/NORD-MODULAR-RACK-VER-3.03.BIN'),
         str(folder / 'flash.bin'), str(patch), str(port), '40'],
        stdout=log, stderr=subprocess.STDOUT,
        env={**os.environ, 'G1_MODEL': 'regular', 'G1_GAIN_DB': '0', 'G1_RECORD': '60', 'G1_PRESETS_TRACE': '1'},
    )
    rpc = None
    try:
        def connect():
            try:
                return Rpc(port)
            except (OSError, ValueError):
                return None
        rpc = wait(connect, lambda value: value is not None)
        state = wait(lambda: rpc.call('status'), lambda s: s.get('requestedVoices') is not None
                     and len(s['knobs']) == 18 and s['knobs'][2]['assigned'], timeout=25)
        bindings = [k['binding'] for k in state['knobs']]

        def set_value(index, value):
            rpc.call('set_knobs', targets=[dict(index=index, value=value, binding=bindings[index])])
            deadline=time.monotonic()+3
            while time.monotonic()<deadline:
                status=rpc.call('status')
                if status['knobs'][index]['value']==value:return status
                time.sleep(.001)
            raise AssertionError('Parameter readback timed out')

        try:
            rpc.call('set_knobs', targets=[dict(index=0, value=70, binding='stale-assignment')])
        except RuntimeError as error:
            assert 'Stale' in str(error), error
        else:
            raise AssertionError('Stale assignment accepted')
        rpc.call('function', pressed=True)
        held_value = rpc.call('status')['knobs'][0]['value']
        rpc.call('set_knobs', targets=[dict(index=0, value=held_value+1, binding=bindings[0])])
        time.sleep(.15)
        assert rpc.call('status')['knobs'][0]['value'] == held_value
        rpc.call('function', pressed=False)
        set_value(2, 31)
        set_value(2, 93)
        print('PASS stale bindings, held Function gating and morph-value writes', flush=True)

        samples = []
        for value in [65, 64] * 6:
            started = time.monotonic()
            set_value(0, value)
            samples.append((time.monotonic() - started) * 1000)
        assert statistics.median(samples) < 50, samples
        (folder / 'timing.json').write_text(json.dumps({'roundtrip_ms': samples}, indent=2))
        print('Encoder confirmation median %.1f ms, max %.1f ms' %
              (statistics.median(samples), max(samples)), flush=True)
        time.sleep(1)
        set_value(0, 76)  # E4 -> E5, exactly as the existing audio integration check.
        time.sleep(1)
        set_value(1, 60)
        time.sleep(1)
    finally:
        if rpc:
            rpc.close()
        if host.poll() is None:
            host.send_signal(signal.SIGTERM)
            host.wait(timeout=10)
        log.close()
    assert host.returncode == 0, (folder / 'host.log').read_text()[-2000:]
    text = (folder / 'host.log').read_text()
    assert text.count('-> parameter') >= 16, 'Direct writes were not exercised'
    analyze(folder / 'output.wav')
    print('PASS editor parameter route and real audio; evidence:', folder)


if __name__ == '__main__':
    main()
