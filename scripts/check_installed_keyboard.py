"""Check the installed per-user watcher with a RAM-only official VST test host."""
from pathlib import Path
import json,os,subprocess,time
from check_official_plugin import ROOT,digest,wait

def main():
    state_dir=Path.home()/'Library/Application Support/Modular Machine'
    def status():
        try:return json.loads((state_dir/'keyboard-status.json').read_text())
        except (OSError,ValueError):return {}
    assert status().get('state')=='waiting','An existing user plugin is active; leave it untouched'
    work=ROOT/'g1-state'/f'installed-keyboard-check-{time.time_ns()}';work.mkdir(parents=True)
    bank=Path.home()/'.local/share/Animatek/G1-Emu/flash.bin';before=digest(bank);host=None
    try:
        with (work/'host.log').open('w') as log:
            host=subprocess.Popen([str(ROOT/'build/g1-emu/g1mk3plugincheck'),str(ROOT/'build/g1-official/alpha30/G1-Emu.vst3')],stdin=subprocess.PIPE,stdout=log,stderr=log,text=True,env={**os.environ,'G1_ROM':str(ROOT/'roms/NORD-MODULAR-RACK-VER-3.03.BIN'),'G1_DATA_DIR':str(work),'G1_PLUGIN_PC_PORT':'0'})
        active=wait(lambda:(lambda s:s if s.get('state')=='controller-running' else None)(status()),25)
        wait(lambda:(state_dir/'keyboard-controller.log').exists() and 'G1-EMU / LIVE' in (state_dir/'keyboard-controller.log').read_text(),35)
        report=None
        for line in (state_dir/'keyboard-controller.log').read_text().splitlines():
            if line.startswith('health: '):report=json.loads(line[8:])
        print('PASS installed launchd service automatically attached its external adapter and background S49 controller to the official VST')
        if report:
            print('Observed hardware state:',report['connection']);print('Screen captures:',report.get('screens'))
            (work/'hardware-health.json').write_text(json.dumps(report,indent=2))
        (work/'active-status.json').write_text(json.dumps(active,indent=2))
        host.stdin.write('{"method":"quit"}\n');host.stdin.flush();host.wait(timeout=15)
        wait(lambda:status().get('state')=='waiting',20)
        print('PASS closing the official VST returned installed watcher to idle')
    finally:
        if host and host.poll() is None:host.stdin.write('{"method":"quit"}\n');host.stdin.flush();host.wait(timeout=15)
        assert digest(bank)==before,'Shared bank file changed'
        print('Shared bank file unchanged; evidence:',work)

if __name__=='__main__':main()
