"""Start or attach to G1-Emu, then run the Maschine controller."""
import argparse
from contextlib import contextmanager
import fcntl
import json
import logging
from logging.handlers import RotatingFileHandler
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
STATE = Path(os.environ.get('MODULAR_MACHINE_STATE_DIR', ROOT / 'g1-state'))
PREFERENCES = STATE / 'controller-preferences.json'


class LaunchError(Exception):
    pass


def read_preferences(path):
    if not path.exists():
        return {}
    try:
        value = json.loads(path.read_text())
        if not isinstance(value, dict) or value.get('version') != 1:
            raise ValueError('unsupported preference format')
        port = value.get('port', 47410)
        if type(port) is not int or not 1024 <= port <= 65535:
            raise ValueError('invalid bridge port')
        for key in ('patch_dir', 'rom', 'flash'):
            if key in value and (not isinstance(value[key], str) or not value[key]):
                raise ValueError(f'invalid {key}')
        return value
    except (OSError, ValueError) as error:
        raise LaunchError(f'Cannot read {path}: {error}. Repair or rename it; it has not been overwritten.') from error


def write_preferences(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode='w', dir=path.parent, prefix='.controller-preferences-', delete=False) as file:
            temporary = Path(file.name)
            json.dump({'version': 1, **value}, file, indent=2)
            file.write('\n')
            file.flush()
            os.fsync(file.fileno())
        os.replace(temporary, path)
    finally:
        if temporary is not None and temporary.exists():
            temporary.unlink()


@contextmanager
def launcher_lock(path):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('a+') as file:
        try:
            fcntl.flock(file, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise LaunchError('The controller is already running through the launcher. Use that terminal/window, or quit its controller first.') from error
        try:
            file.seek(0)
            file.truncate()
            file.write(str(os.getpid()) + '\n')
            file.flush()
            yield
        finally:
            fcntl.flock(file, fcntl.LOCK_UN)


def hardware_lock_path():
    return Path.home()/'Library/Application Support/Modular Machine/controller-launcher.lock'

@contextmanager
def hardware_ownership(args):
    if args.mock or hardware_lock_path().resolve()==(STATE/'controller-launcher.lock').resolve():
        yield
    else:
        with launcher_lock(hardware_lock_path()):yield


class Bridge:
    def __init__(self, stream):
        self.stream = stream
        self.buffer = bytearray()
        self.hello = {}

    def read(self):
        deadline = time.monotonic() + 2
        while b'\n' not in self.buffer:
            if len(self.buffer) > 1024 * 1024:
                raise LaunchError('Bridge response exceeds the size limit.')
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise LaunchError('G1-Emu bridge did not answer within two seconds.')
            self.stream.settimeout(remaining)
            chunk = self.stream.recv(8192)
            if not chunk:
                raise LaunchError('Bridge connection was refused or closed. Another controller may already be connected; quit it before launching again.')
            self.buffer.extend(chunk)
        line, _, rest = self.buffer.partition(b'\n')
        if len(line) > 1024 * 1024:
            raise LaunchError('Bridge response exceeds the size limit.')
        self.buffer = bytearray(rest)
        try:
            value = json.loads(line)
            if not isinstance(value, dict):
                raise ValueError('expected an object')
            return value
        except ValueError as error:
            raise LaunchError('The port is occupied by a service that does not speak the G1 controller protocol.') from error

    def status(self):
        self.stream.sendall(b'{"id":1,"method":"status"}\n')
        response = self.read()
        if response.get('id') != 1 or 'error' in response:
            raise LaunchError('G1-Emu status failed: ' + str(response.get('error', 'invalid response ID')))
        value = response.get('result')
        if not isinstance(value, dict) or type(value.get('running')) is not bool:
            raise LaunchError('G1-Emu returned an invalid status.')
        upload = value.get('upload')
        if not isinstance(upload, dict):
            raise LaunchError('The running G1-Emu bridge is outdated. Close G1-Emu normally, then launch again to use the updated local app.')
        if type(upload.get('serial')) is not int or upload['serial'] < 0 or type(upload.get('busy')) is not bool or type(upload.get('ok')) is not bool:
            raise LaunchError('G1-Emu returned invalid upload status; use the matching local bridge build.')
        if value['running'] and not isinstance(value.get('configuration'), dict):
            raise LaunchError('The running G1-Emu bridge is outdated for Settings/Mixer/Function. Close G1-Emu normally and launch the updated local app.')
        if value.get('encoderInput') != 'editor-parameters':
            raise LaunchError('Save your current patch, close G1-Emu normally, then rerun the standalone launcher. The updated encoder path requires the new native bridge; the running synth has been left untouched.')
        if value.get('engineThreadRequests') is not True and not (value.get('hostType') == 'plugin' and value.get('engineAccess') == 'runner-lock'):
            raise LaunchError('Save your patch and close G1-Emu normally, then rerun the standalone launcher to load the stability update. The running synth was left untouched.')
        if not isinstance(value.get('bankLoad'),dict):
            raise LaunchError('The native bridge needs the confirmed bank-load update. Save your patch, close G1-Emu normally and rerun the standalone launcher.')
        if value.get("padMidi") is not True:
            raise LaunchError("Save your patch, close G1-Emu normally and rerun the standalone launcher to enable the updated MIDI pads bridge. The running synth was left untouched.")
        if value.get("transportThread") != "dedicated":
            raise LaunchError("Save your patch, close G1-Emu normally and rerun the standalone launcher to load the dedicated control transport. The running synth was left untouched.")
        return value


def connect_bridge(port):
    try:
        stream = socket.create_connection(('127.0.0.1', port), timeout=1)
    except ConnectionRefusedError:
        return None
    except OSError as error:
        raise LaunchError(f'Cannot connect to 127.0.0.1:{port}: {error}') from error
    try:
        bridge = Bridge(stream)
        hello = bridge.read()
        if hello.get('service') != 'modular-machine-g1' or hello.get('protocol') != 1:
            raise LaunchError(f'Port {port} is in use by an incompatible service. Close that service or choose another --port.')
        bridge.hello = hello
        return bridge
    except BaseException:
        stream.close()
        raise


def find_plugins():
    found=[]
    for port in range(47420,47484):
        bridge=None
        try:
            bridge=connect_bridge(port)
            if bridge and bridge.hello.get('hostType')=='plugin':
                found.append({'port':port,'name':bridge.hello.get('pluginName','G1 plugin'),'instance':bridge.hello.get('pluginInstance'),'id':bridge.hello.get('instance')})
        except (LaunchError,OSError):
            pass
        finally:
            if bridge:bridge.stream.close()
    return found


def wait_ready(port, process=None, timeout=35):
    deadline = time.monotonic() + timeout
    bridge = None
    try:
        while time.monotonic() < deadline:
            if process is not None and process.poll() is not None:
                raise LaunchError('G1-Emu exited before its bridge was ready. Close any other standalone instance and inspect g1-state/launcher-emulator.log.')
            if bridge is None:
                bridge = connect_bridge(port)
            if bridge is not None:
                status = bridge.status()
                if status['running']:
                    return
            time.sleep(.1)
        raise LaunchError('G1-Emu is not ready. Check its ROM/audio settings and g1-state/launcher-emulator.log, then relaunch. Any opened emulator window remains available.')
    finally:
        if bridge is not None:
            bridge.stream.close()


def stop_controller(process):
    if process.poll() is None:
        process.send_signal(signal.SIGINT)
        try:
            process.wait(timeout=8)
        except subprocess.TimeoutExpired:
            process.terminate()
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                print('Controller did not finish cleanup; stopping its process. G1-Emu is left running.', flush=True)
                process.kill()
                process.wait()


def launch(args, preferences_path=PREFERENCES):
    saved = read_preferences(preferences_path)
    plugin_mode=getattr(args,'plugin',False)
    if plugin_mode and args.port is None:
        plugins=find_plugins()
        if not plugins:raise LaunchError('No available Modular Machine plugin found. Load the VST3/AU in your DAW and enable audio processing; quit any other controller connected to it.')
        if len(plugins)>1:
            choices='; '.join(f"{p['port']}: {p['name']} (instance {p['instance']})" for p in plugins)
            raise LaunchError('Several plugins are open. Select one with --plugin --port PORT: '+choices)
        port=plugins[0]['port']
        time.sleep(.05)
    else:port = args.port if args.port is not None else saved.get('port', 47410)
    if not 1024 <= port <= 65535:
        raise LaunchError('--port must be between 1024 and 65535')
    patch_dir = Path(args.patch_dir or saved.get('patch_dir', ROOT / 'patches')).expanduser().resolve()
    rom = Path(args.rom or saved.get('rom', ROOT / 'roms/NORD-MODULAR-RACK-VER-3.03.BIN')).expanduser().resolve()
    flash = Path(args.flash or saved.get('flash', STATE / 'controller-flash.bin')).expanduser().resolve()
    if not patch_dir.is_dir():
        raise LaunchError(f'Patch library folder is missing: {patch_dir}. Select one with --patch-dir FOLDER.')
    with launcher_lock(STATE / 'controller-launcher.lock'), hardware_ownership(args):
        if not args.no_build:
            print('Preparing controller…', flush=True)
            subprocess.run(['cargo', 'build', '--locked'], cwd=ROOT, check=True)
        controller = ROOT / 'target/debug/nord-modular-ni-interface'
        if not controller.is_file():
            raise LaunchError('Controller is not built. Run cargo build --locked, then launch again.')
        bridge = connect_bridge(port)
        if bridge is None:
            if plugin_mode:raise LaunchError(f'No plugin on port {port}. Load it in your DAW and enable audio processing.')
            if not rom.is_file():
                raise LaunchError(f'Compatible local ROM required: {rom}. Supply --rom PATH; firmware is never downloaded.')
            if not (ROOT / 'build/g1-emu/G1-Emu.app/Contents/MacOS/G1-Emu').is_file():
                raise LaunchError('G1-Emu is not built. Run python3 scripts/build_g1.py first.')
            print('Starting G1-Emu; waiting for the OS and controller bridge…', flush=True)
            command = [sys.executable, '-B', str(ROOT / 'scripts/run_g1.py'), '--rom', str(rom), '--flash', str(flash), '--port', str(port)]
            if args.demo:
                command.append('--demo')
            with (STATE / 'launcher-emulator.log').open('a') as log:
                emulator = subprocess.Popen(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            wait_ready(port, emulator)
        else:
            try:
                if plugin_mode and bridge.hello.get('hostType')!='plugin':raise LaunchError('That port belongs to the standalone. Choose the plugin port with --plugin --port PORT.')
                ready = bridge.status()['running']
            finally:
                bridge.stream.close()
            print(('Attached to DAW plugin.' if plugin_mode else 'Attached to running G1-Emu.') if ready else 'G1-Emu is starting; waiting for the OS…', flush=True)
            if not ready:
                # Let the bridge retire the probe connection before reconnecting.
                time.sleep(.1)
                wait_ready(port)
        write_preferences(preferences_path, {'port': port, 'patch_dir': str(patch_dir), 'rom': str(rom), 'flash': str(flash)})
        command = [str(controller), '--g1-address', f'127.0.0.1:{port}', '--patch-dir', str(patch_dir)]
        command.extend(['--controller', getattr(args, 'controller', 'komplete')])
        if args.mock:
            command.append('--mock')
        if args.visible:
            command.append('--visible')
        print(f'Ready — starting controller. Patch library: {patch_dir}', flush=True)
        print('Keep NI agents running. Type quit or press Ctrl+C to stop the controller; G1-Emu stays open.', flush=True)
        session_log=logging.getLogger('controller-session')
        session_log.setLevel(logging.INFO)
        handler=RotatingFileHandler(preferences_path.with_name('controller-session.log'),maxBytes=2*1024*1024,backupCount=2)
        handler.setFormatter(logging.Formatter('%(asctime)s %(message)s'))
        session_log.addHandler(handler)
        process = subprocess.Popen(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1,
                                   env={**os.environ, 'MK3_BROWSER_STATE': str(preferences_path.with_name('browser-state.json'))})
        try:
            for line in process.stdout:
                session_log.info(line.rstrip())
                if not line.startswith('health: '):print(line,end='',flush=True)
            result = process.wait()
        except KeyboardInterrupt:
            stop_controller(process)
            result = 0
        finally:
            stop_controller(process)
            for line in process.stdout or []:
                session_log.info(line.rstrip())
                if not line.startswith('health: '):print(line,end='',flush=True)
            if hasattr(process.stdout,'close'):process.stdout.close()
            handler.close();session_log.removeHandler(handler)
        print('Controller stopped. G1-Emu remains available; close its window normally to save its banks.', flush=True)
        return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--patch-dir', type=Path, help='Patch library folder; remembered after a successful connection')
    parser.add_argument('--rom', type=Path, help='Compatible local rack OS ROM')
    parser.add_argument('--flash', type=Path, help='Dedicated emulator bank file')
    parser.add_argument('--port', type=int, help='Local bridge port (default 47410; remembered)')
    parser.add_argument('--demo', action='store_true', help='Seed the project patch only when creating a new bank file')
    parser.add_argument('--controller', choices=['maschine','komplete'], default='komplete', help='Hardware panel: Maschine MK3 or Komplete Kontrol S49 Mk2')
    parser.add_argument('--mock', action='store_true', help='Desktop controller screens; still uses the real G1 emulator')
    parser.add_argument('--visible', action='store_true', help='Show hardware-screen previews')
    parser.add_argument('--no-build', action='store_true', help='Use the current debug controller binary')
    parser.add_argument('--plugin',action='store_true',help='Attach to a DAW plugin; never start the standalone')
    parser.add_argument('--list-plugins',action='store_true',help='List available DAW plugin control ports and exit')
    parser.add_argument('--preferences', type=Path, default=None, help='Controller preference file (default: g1-state/controller-preferences.json)')
    args = parser.parse_args()
    if args.list_plugins:
        plugins=find_plugins()
        for plugin in plugins:print(f"{plugin['port']}: {plugin['name']} (instance {plugin['instance']})")
        if not plugins:print('No available plugin instances. Load the plugin in your DAW and enable processing.')
        return 0
    if args.preferences is None:args.preferences=STATE/('plugin-controller-preferences.json' if args.plugin else 'controller-preferences.json')
    def interrupted(*_):
        raise KeyboardInterrupt()
    signal.signal(signal.SIGTERM, interrupted)
    try:
        return launch(args, args.preferences.expanduser().resolve())
    except KeyboardInterrupt:
        print('\nLaunch cancelled. Any opened G1-Emu window remains available.', flush=True)
        return 130
    except (LaunchError, OSError, subprocess.CalledProcessError) as error:
        print(f'Cannot start Nord Modular NI Interface: {error}', file=sys.stderr, flush=True)
        return 1


if __name__ == '__main__':
    sys.exit(main())
