"""Verify the local app/signature and offline source inputs without opening its GUI."""
import hashlib
import json
import os
import signal
import socket
import time
from pathlib import Path
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    output = ROOT / 'build/package'
    manifest = json.loads((output / 'manifest.json').read_text())
    app = output / ('Modular Machine ' + manifest['version'] + '.app')
    project = app / 'Contents/Resources/project'
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
    assert sha256(project / 'target/debug/modular-machine-mk3') == manifest['controllerSha256']
    assert sha256(project / 'build/g1-emu/G1-Emu.app/Contents/MacOS/G1-Emu') == manifest['nativeSha256']
    assert not manifest['proprietaryNordRomIncluded'] and not manifest['userBanksIncluded'] and not manifest['userPatchLibraryIncluded']
    assert sorted(p.name for p in (project / 'patches').iterdir()) == ['MK3-AudioIn.pch', 'MK3-Bridge.pch', 'MK3-MIDI.pch']
    assert not list(project.rglob('*.bin')) and not list(project.rglob('*.rom'))
    for plugin in manifest.get('plugins',[]):
        import plistlib
        bundle=output/plugin['path']
        subprocess.run(['codesign','--verify','--deep','--strict',str(bundle)],check=True)
        with (bundle/'Contents/Info.plist').open('rb') as file:info=plistlib.load(file)
        assert sha256(bundle/'Contents/MacOS'/info['CFBundleExecutable'])==plugin['sha256']
    with tempfile.TemporaryDirectory(prefix='mk3-package-') as temporary:
        work = Path(temporary)
        env = {**os.environ, 'MODULAR_MACHINE_STATE_DIR': str(work / 'state')}
        help_text = subprocess.check_output(['/usr/bin/python3', str(project / 'scripts/run_packaged.py'), '--help'], env=env, text=True)
        assert '--patch-dir' in help_text
        assert not (project / 'g1-state').exists()
        # Exercise the packaged launcher/controller directly, without opening Terminal or an app window.
        from check_g1 import Rpc, wait
        with socket.socket() as probe:
            probe.bind(('127.0.0.1', 0))
            port = probe.getsockname()[1]
        runtime_env = {**env, 'MK3_HIDDEN_PREVIEW': '1', 'G1_MIDI_CLIENT': 'MK3-Package-Test', 'G1_GAIN_DB': '-12', 'G1_MODEL': 'regular'}
        with (work / 'host.log').open('w') as host_log, (work / 'controller.log').open('w') as controller_log:
            host = subprocess.Popen([str(project / 'build/g1-emu/g1mk3check'), str(ROOT / 'roms/NORD-MODULAR-RACK-VER-3.03.BIN'), str(work / 'flash.bin'), str(project / 'patches/MK3-Bridge.pch'), str(port), '60'], env=runtime_env, stdout=host_log, stderr=subprocess.STDOUT)
            controller = None
            try:
                def connect():
                    try:
                        return Rpc(port)
                    except OSError:
                        return None
                rpc = wait(connect, lambda r: r is not None)
                wait(lambda: rpc.call('status'), lambda status: status.get('running') and len(status.get('knobs', [])) == 18)
                rpc.close()
                time.sleep(.1)
                controller = subprocess.Popen(['/usr/bin/python3', str(project / 'scripts/run_packaged.py'), '--mock', '--port', str(port), '--rom', str(ROOT / 'roms/NORD-MODULAR-RACK-VER-3.03.BIN'), '--flash', str(work / 'flash.bin'), '--patch-dir', str(project / 'patches')], env=runtime_env, stdin=subprocess.PIPE, stdout=controller_log, stderr=subprocess.STDOUT)
                wait(lambda: (work / 'controller.log').read_text(), lambda text: 'G1-EMU / LIVE' in text, timeout=20)
                controller.stdin.write(b'quit\n')
                controller.stdin.flush()
                assert controller.wait(timeout=10) == 0, (work / 'controller.log').read_text()
                assert host.poll() is None, 'controller exit stopped emulator'
                assert (work / 'state/controller-session.log').is_file()
            finally:
                if controller and controller.poll() is None:
                    controller.terminate()
                    controller.wait(timeout=10)
                if host.poll() is None:
                    host.send_signal(signal.SIGTERM)
                    host.wait(timeout=10)
        print('PASS packaged hidden launcher attaches to real engine; quit preserves synth and writes external logs', flush=True)
        source = output / manifest['sourceArchive']['name']
        assert sha256(source) == manifest['sourceArchive']['sha256']
        with tarfile.open(source) as archive:
            names = archive.getnames()
            assert len(names) == len(set(names)), 'duplicate source members'
            for member in archive.getmembers():
                path = Path(member.name)
                assert not path.is_absolute() and '..' not in path.parts and member.isfile(), member.name
                assert path.parts[0] == 'modular-machine-source'
            archive.extractall(work)
        source_root = work / 'modular-machine-source'
        cargo_env = {**env, 'CARGO_HOME': str(work / 'empty-cargo-home')}
        subprocess.run(['cargo', 'metadata', '--locked', '--offline', '--format-version', '1'], cwd=source_root / 'project', env=cargo_env, check=True, stdout=subprocess.DEVNULL)
        native = source_root / 'native'
        command = ['cmake', '-S', str(native / 'g1-emu'), '-B', str(work / 'native-build'), '-DCMAKE_BUILD_TYPE=Release', '-DCMAKE_OSX_ARCHITECTURES=arm64', '-DGEARMULATOR_DIR=' + str(native / 'gearmulator'), '-DG1_DSP56300_DIR=' + str(native / 'dsp56300'), '-DG1_JUCE_DIR=' + str(native / 'juce'), '-DG1_BACKEND=juce', '-DG1_LTO=OFF']
        subprocess.run(command, check=True, stdout=subprocess.DEVNULL)
    print('PASS local app signature, matching binaries, external-data bootstrap, source digest, isolated offline Cargo resolution and native CMake configure')


if __name__ == '__main__':
    main()
