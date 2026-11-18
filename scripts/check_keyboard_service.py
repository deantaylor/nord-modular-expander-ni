"""Official-VST appearance/removal, automatic hidden controller and supervisor cleanup."""
from pathlib import Path
import argparse,json,os,subprocess,time
from check_official_plugin import ROOT,digest,discover,protect_plugin_data,wait
from keyboard_service import CONTROLLERS

def alive(pid):
    try:os.kill(pid,0);return True
    except ProcessLookupError:return False

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--controller',choices=CONTROLLERS,default='komplete')
    args=parser.parse_args()
    work=ROOT/'g1-state'/f'keyboard-service-check-{time.time_ns()}';work.mkdir(parents=True)
    bank=Path.home()/'.local/share/Animatek/G1-Emu/flash.bin';before=digest(bank)
    sandbox=protect_plugin_data(work,bank)
    host=service=None
    try:
        known=discover()
        with (work/'host.log').open('w') as log:
            host=subprocess.Popen([*sandbox,str(ROOT/'build/g1-emu/g1mk3plugincheck'),str(ROOT/'build/g1-official/alpha30/G1-Emu.vst3')],stdin=subprocess.PIPE,stdout=log,stderr=log,text=True,env={**os.environ,'G1_ROM':str(ROOT/'roms/NORD-MODULAR-RACK-VER-3.03.BIN'),'G1_DATA_DIR':str(work),'G1_PLUGIN_PC_PORT':'0'})
        port=wait(lambda:next((p for p in discover() if p not in known),None))
        (work/'keyboard-preferences.json').write_text(json.dumps({'direct_port':port,'patch_dir':str(ROOT/'patches')}))
        with (work/'service-output.log').open('w') as log:service=subprocess.Popen(['/usr/bin/python3','-B',str(ROOT/'scripts/keyboard_service.py'),'--mock','--controller',args.controller],stdout=log,stderr=log,env={**os.environ,'MODULAR_MACHINE_STATE_DIR':str(work),'MK3_HIDDEN_PREVIEW':'1'})
        def state():
            try:return json.loads((work/'keyboard-status.json').read_text())
            except (OSError,ValueError):return {}
        active=wait(lambda:(lambda s:s if s.get('state')=='controller-running' else None)(state()),15)
        assert active['controller']==args.controller
        assert alive(active['adapterPid']) and alive(active['controllerPid'])
        wait(lambda:(work/'keyboard-controller.log').exists() and 'G1-EMU / LIVE' in (work/'keyboard-controller.log').read_text(),30)
        print(f'PASS official plugin discovery starts one external adapter and hidden {CONTROLLERS[args.controller]} controller')
        # A second watcher exits without replacing the first or creating controller instances.
        duplicate=subprocess.run(['/usr/bin/python3','-B',str(ROOT/'scripts/keyboard_service.py'),'--mock'],env={**os.environ,'MODULAR_MACHINE_STATE_DIR':str(work)},capture_output=True,timeout=5)
        assert duplicate.returncode==0 and state()['controllerPid']==active['controllerPid']
        host.stdin.write('{"method":"quit"}\n');host.stdin.flush();host.wait(timeout=15)
        wait(lambda:state().get('state')=='waiting',15)
        assert not alive(active['adapterPid']) and not alive(active['controllerPid'])
        assert service.poll() is None
        print('PASS duplicate watcher suppressed; plugin removal stops only owned children and returns watcher to idle')
    finally:
        if service and service.poll() is None:service.terminate();service.wait(timeout=15)
        if host and host.poll() is None:host.stdin.write('{"method":"quit"}\n');host.stdin.flush();host.wait(timeout=15)
        assert digest(bank)==before,'Shared bank file changed'
        print('Evidence:',work)

if __name__=='__main__':main()
