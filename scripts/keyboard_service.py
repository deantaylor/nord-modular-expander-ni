"""Run the selected NI surface for official G1-Emu DirectLink instances."""
import argparse
from contextlib import ExitStack
import fcntl,json,logging,os,signal,socket,subprocess,time
from logging.handlers import RotatingFileHandler
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
STATE=Path(os.environ.get('MODULAR_MACHINE_STATE_DIR',Path.home()/'Library/Application Support/Modular Machine'))
STOP=False
CONTROLLERS={'komplete':'Komplete Kontrol S49 Mk2','maschine':'Maschine MK3'}

def discover(preferred=None):
    ports=[preferred] if preferred is not None else range(47310,47318)
    for port in ports:
        try:
            with socket.create_connection(('127.0.0.1',port),.08) as stream:
                stream.settimeout(.15);line=bytearray()
                while b'\n' not in line and len(line)<1024:
                    data=stream.recv(1024)
                    if not data:break
                    line.extend(data)
                greeting=line.partition(b'\n')[0].decode('utf-8','strict')
                if greeting.startswith('G1-Emu 1 G1-Emu plugin '):return port
        except (OSError,UnicodeError):pass
    return None

def stop_child(process):
    if process is None or process.poll() is not None:return
    process.send_signal(signal.SIGINT)
    try:process.wait(timeout=6)
    except subprocess.TimeoutExpired:
        process.terminate()
        try:process.wait(timeout=3)
        except subprocess.TimeoutExpired:process.kill();process.wait()

def acquire(path):
    path.parent.mkdir(parents=True,exist_ok=True);file=path.open('a+')
    try:fcntl.flock(file,fcntl.LOCK_EX|fcntl.LOCK_NB)
    except BlockingIOError:file.close();return None
    file.seek(0);file.truncate();file.write(str(os.getpid())+'\n');file.flush();return file

def bridge_ready(port):
    try:
        with socket.create_connection(('127.0.0.1',port),.1) as stream:
            stream.settimeout(.2);data=bytearray()
            while b'\n' not in data and len(data)<4096:
                part=stream.recv(4096)
                if not part:return False
                data.extend(part)
            hello=json.loads(data.partition(b'\n')[0]);return hello.get('hostType')=='official-plugin' and hello.get('service')=='modular-machine-g1'
    except (OSError,ValueError):return False

def patch_directory():
    for p in [STATE/'keyboard-preferences.json',STATE/'plugin-controller-preferences.json',ROOT/'g1-state/plugin-controller-preferences.json']:
        try:
            folder=Path(json.loads(p.read_text())['patch_dir']).expanduser()
            if folder.is_dir():return folder
        except (OSError,KeyError,ValueError):pass
    return ROOT/'patches'

def preferred_port():
    try:
        p=json.loads((STATE/'keyboard-preferences.json').read_text()).get('direct_port')
        return p if type(p) is int and 47310<=p<47318 else None
    except (OSError,ValueError):return None

def paths():
    adapter=ROOT/'bin/g1keyboardadapter';controller=ROOT/'bin/modular-machine-keyboard'
    if not adapter.exists():adapter=ROOT/'build/g1-official/adapter-build/g1keyboardadapter_artefacts/Release/g1keyboardadapter'
    if not controller.exists():controller=ROOT/'target/release/modular-machine-mk3'
    return adapter,controller

def child_log(name):
    for number in (3,2,1):
        old=STATE/(name if number==1 else f'{name}.{number-1}')
        if old.exists():old.replace(STATE/f'{name}.{number}')
    return (STATE/name).open('w')

def run(mock=False, profile='komplete'):
    STATE.mkdir(parents=True,exist_ok=True)
    log=logging.getLogger('keyboard-service');log.setLevel(logging.INFO)
    handler=RotatingFileHandler(STATE/'keyboard-service.log',maxBytes=1024*1024,backupCount=2);log.addHandler(handler)
    lock=acquire(STATE/'keyboard-service.lock')
    if lock is None:return 0
    adapter_binary,controller_binary=paths()
    if not adapter_binary.is_file() or not controller_binary.is_file():raise RuntimeError('Build/install the official keyboard adapter first')
    def state(value,**fields):
        temporary=STATE/'keyboard-status.json.tmp';temporary.write_text(json.dumps(dict(state=value,**fields))+'\n');temporary.replace(STATE/'keyboard-status.json')
    state('waiting')
    selected=None;adapter=controller=None;ownership=None;log.info('Waiting for official G1-Emu plugin; key MIDI remains independent')
    def release():
        nonlocal adapter,controller,ownership,selected
        stop_child(controller);stop_child(adapter);controller=adapter=None;selected=None
        if ownership:ownership.close();ownership=None
        state('waiting')
    try:
        while not STOP:
            if adapter is not None:
                preferred=preferred_port()
                if adapter.poll() is not None or (preferred is not None and preferred!=selected):
                    log.info('Plugin/selection ended; cleaning up only owned controller processes');release()
                elif controller is not None and controller.poll() is not None:
                    log.warning('Controller exited; reconnecting');release()
                time.sleep(.25);continue
            port=discover(preferred_port())
            if port is None:time.sleep(.6);continue
            # Cooperate with manual source and packaged launchers; never kill another owner.
            ownership=ExitStack()
            locks={STATE/'controller-launcher.lock'}
            if any((f:=acquire(p)) is None or not ownership.enter_context(f) for p in sorted(locks)):
                ownership.close();ownership=None;time.sleep(1);continue
            with socket.socket() as probe:probe.bind(('127.0.0.1',0));json_port=probe.getsockname()[1]
            env={**os.environ,'MODULAR_MACHINE_BACKGROUND':'1','MK3_BROWSER_STATE':str(STATE/'keyboard-browser-state.json')}
            try:preferences=json.loads((STATE/'keyboard-preferences.json').read_text())
            except (OSError,ValueError):preferences={}
            if preferences.get('trace_pipeline') is True:
                env['MK3_TRACE_PIPELINE']='1';env['MK3_TRACE_UNTIL_MS']=str(int((time.time()+600)*1000))
                log.info('Pipeline trace enabled for ten minutes; stage logs in controller/adapter logs')
            if profile=='komplete' and preferences.get('s49_full_frames') is True:
                env['MK3_S49_FULL_FRAMES']='1'
                log.info('S49 service uses self-contained full frames; unchanged frames skipped')
            if (STATE/'parameter-names.json').is_file():env['MK3_PARAMETER_NAMES']=str(STATE/'parameter-names.json')
            adapter_log=child_log('adapter-session.log')
            adapter=subprocess.Popen([str(adapter_binary),str(port),str(json_port)],stdin=subprocess.DEVNULL,stdout=adapter_log,stderr=subprocess.STDOUT,env=env);adapter_log.close();selected=port
            deadline=time.monotonic()+5
            while adapter.poll() is None and not bridge_ready(json_port) and time.monotonic()<deadline and not STOP:time.sleep(.05)
            if adapter.poll() is not None or STOP or not bridge_ready(json_port):release();time.sleep(1);continue
            try:trace_input=json.loads((STATE/'keyboard-preferences.json').read_text()).get('trace_input',False) is True
            except (OSError,ValueError):trace_input=False
            if trace_input:env['RUST_LOG']='info,encdr::usb::device_thread=trace'
            if profile=='komplete' and preferences.get('capture_buttons_only') is True:
                env['MK3_S49_CAPTURE_BUTTONS_ONLY']='1';env['RUST_LOG']='info,encdr::usb::device_thread=trace'
                if preferences.get('probe_leds') is True:env['MK3_S49_LED_PROBE_FILE']=str(STATE/'s49-led-probe.json')
            controller_log=child_log('keyboard-controller.log')
            command=[str(controller_binary),'--controller',profile,'--g1-address',f'127.0.0.1:{json_port}','--patch-dir',str(patch_directory())]
            if mock:command.append('--mock')
            controller=subprocess.Popen(command,stdin=subprocess.DEVNULL,stdout=controller_log,stderr=subprocess.STDOUT,env=env);controller_log.close()
            state('controller-running',controller=profile,directPort=port,adapterPid=adapter.pid,controllerPid=controller.pid)
            log.info('Attached %s bridge to official protocol port %s',CONTROLLERS[profile],port)
    finally:release();lock.close();log.info('Keyboard service stopped')
    return 0

def main():
    global STOP
    def stopped(*_):
        global STOP
        STOP=True
    signal.signal(signal.SIGTERM,stopped);signal.signal(signal.SIGINT,stopped)
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--mock',action='store_true',help='Isolated hidden UI verification; no hardware claimed')
    parser.add_argument('--controller',choices=CONTROLLERS,default='komplete')
    args=parser.parse_args()
    return run(args.mock,args.controller)

if __name__=='__main__':raise SystemExit(main())
