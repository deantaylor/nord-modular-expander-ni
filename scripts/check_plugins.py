"""Load the actual VST3 in an isolated JUCE host; verify two bridges, sound and project restore."""
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import time
from check_g1 import ROOT,Rpc,wait


class Peer:
    """Lifecycle transitions may close a connection; retry reads, never replay writes."""
    def __init__(self,port):self.port=port;self.rpc=Rpc(port)
    def call(self,method,**fields):
        if self.rpc is None:self.rpc=Rpc(self.port)
        try:return self.rpc.call(method,**fields)
        except (OSError,ValueError,RuntimeError) as error:
            self.close()
            if method=='status':raise OSError(str(error)) from error
            raise
    def close(self):
        if self.rpc:
            try:self.rpc.close()
            except OSError:pass
        self.rpc=None


def main():
    import argparse
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fx',action='store_true')
    args=parser.parse_args()
    name='Modular Machine G1'+(' FX' if args.fx else '')
    bundle=ROOT/'build/g1-emu/plugins/VST3'/(name+'.vst3')
    assert bundle.is_dir(),bundle
    subprocess.run(['codesign','--verify','--deep','--strict',str(bundle)],check=True)
    folder=ROOT/'g1-state'/('plugin-check-'+('fx-' if args.fx else '')+str(time.time_ns()));folder.mkdir()
    state_dir=folder/'data';state_dir.mkdir()
    path=folder/'host.log';log=path.open('w')
    host=subprocess.Popen([str(ROOT/'build/g1-emu/g1mk3plugincheck'),str(bundle),'2'],stdin=subprocess.PIPE,stdout=log,stderr=subprocess.STDOUT,
                          env={**os.environ,'G1_ROM':str(ROOT/'roms/NORD-MODULAR-RACK-VER-3.03.BIN'),'G1_DATA_DIR':str(state_dir),'G1_PLUGIN_MARGIN_MS':'10'})
    peers=[];launcher=None
    def command(method,index=0,**fields):
        host.stdin.write((json.dumps(dict(method=method,index=index,**fields))+'\n').encode());host.stdin.flush()
    def text():
        content=path.read_text(errors='replace')
        if host.poll() is not None:raise RuntimeError(content[-5000:])
        return content
    def attach(port):
        try:return Peer(port)
        except OSError:return None
    try:
        wait(text,lambda t:'HOST READY' in t,60)
        ports=list(dict.fromkeys(int(p) for p in re.findall(r'MK3 bridge v1 on 127.0.0.1:(\d+)',text())))
        assert len(ports)==2,ports
        for port in ports:
            rpc=wait(lambda:attach(port),lambda p:p is not None)
            peers.append(rpc)
            wait(lambda:rpc.call('status'),lambda s:s['running'] and s.get('hostType')=='plugin',30)
        first,second=peers
        assert first.call('status')['pluginInstance']!=second.call('status')['pluginInstance']
        for rpc in peers:
            before=rpc.call('status')['upload']['serial']
            rpc.call('load_file',slot=0,path=str(ROOT/'patches/MK3-MIDI.pch'))
            wait(lambda:rpc.call('status'),lambda s:s['upload']['serial']>before and not s['upload']['busy'] and s['upload']['ok'] and s['knobs'][0]['assigned'],25)
        a=first.call('status');b=second.call('status')
        old_binding=a['knobs'][0]['binding']
        first.call('set_knobs',targets=[dict(index=0,value=70,binding=old_binding)])
        wait(lambda:first.call('status'),lambda s:s['knobs'][0]['value']==70)
        assert second.call('status')['knobs'][0]['value']==b['knobs'][0]['value'],'cross-instance knob edit'
        first.call('set_control',key='gain',value=12)
        wait(lambda:first.call('status'),lambda s:any(c['key']=='gain' and c['value']==12 for c in s['configuration']['controls']))
        time.sleep(.2);mark=len(text());command('parameters')
        params=wait(lambda:text()[mark:],lambda t:'HOST PARAMETERS' in t)
        values=re.search(r'first=([\d.]+) gain=([\d.]+)',params)
        assert values and float(values[1])>.52 and abs(float(values[2])-.2)<.01,params
        print('PASS actual '+name+' VST3, two independent instance ports, knob readback and host parameters',flush=True)
        mark=len(text());before=first.call('status')['midiInBytes']
        first.call('pad_midi',pad=0,slot=0,note=48,velocity=100)
        wait(lambda:first.call('status'),lambda s:s['padMidiHeld']==[0] and s['midiInBytes']>=before+3)
        wait(lambda:text()[mark:],lambda t:any(float(v)>1e-5 for v in re.findall(r'HOST AUDIO index=0 peak=([\d.]+)',t)),8)
        command('pause')
        wait(lambda:first.call('status'),lambda s:not s['running'],10)
        command('resume')
        wait(lambda:first.call('status'),lambda s:s['running'] and s['padMidiHeld']==[],20)
        print('PASS MIDI pad audio and note cleanup across host pause/resume',flush=True)
        time.sleep(.8)
        saved=folder/'project-state.bin';command('save',path=str(saved))
        wait(lambda:saved.stat().st_size if saved.exists() else 0,lambda n:n>100,15)
        state_before=first.call('status')
        binding=state_before['knobs'][0]['binding']
        first.call('set_knobs',targets=[dict(index=0,value=42,binding=binding)])
        wait(lambda:first.call('status'),lambda s:s['knobs'][0]['value']==42)
        command('restore',path=str(saved))
        restored=wait(lambda:first.call('status'),lambda s:s['running'] and s.get('engineGeneration',0)>state_before['engineGeneration'] and s['knobs'][0]['assigned'] and s['knobs'][0]['value']==70,30)
        assert restored['knobs'][0]['binding']!=binding
        try:first.call('set_knobs',targets=[dict(index=0,value=5,binding=binding)])
        except RuntimeError as e:assert 'Stale' in str(e)
        else:raise AssertionError('restored engine accepted stale target')
        assert second.call('status')['knobs'][0]['value']==b['knobs'][0]['value']
        print('PASS DAW state save/restore, patch/knob persistence and stale-target rejection',flush=True)
        if args.fx:
            before=first.call('status')['upload']['serial']
            first.call('load_file',slot=0,path=str(ROOT/'patches/MK3-AudioIn.pch'))
            wait(lambda:first.call('status'),lambda s:s['upload']['serial']>before and s['upload']['ok'] and not s['upload']['busy'],20)
            mark=len(text());command('input',value=True)
            wait(lambda:text()[mark:],lambda t:any(float(v)>1e-5 for v in re.findall(r'HOST AUDIO index=0 peak=([\d.]+)',t)),8)
            command('input',value=False);time.sleep(.7);mark=len(text())
            tail=wait(lambda:text()[mark:],lambda t:len(re.findall(r'HOST AUDIO index=0 peak=([\d.]+)',t))>=2,5)
            assert all(float(v)<1e-5 for v in re.findall(r'HOST AUDIO index=0 peak=([\d.]+)',tail)),tail[-1000:]
            # Restore the gated patch for the controller-pad smoke below.
            before=first.call('status')['upload']['serial']
            first.call('load_file',slot=0,path=str(ROOT/'patches/MK3-MIDI.pch'))
            wait(lambda:first.call('status'),lambda s:s['upload']['serial']>before and s['upload']['ok'] and not s['upload']['busy'],20)
            print('PASS actual host audio input through G1 FX; silence after input stops',flush=True)
        # Verify the ordinary controller launcher selects this plugin without starting a synth app.
        first.close();peers[0]=None;time.sleep(.1)
        runtime=folder/'controller';runtime.mkdir()
        controller_log=(folder/'controller.log').open('w')
        launcher=subprocess.Popen(['python3',str(ROOT/'scripts/run_machine.py'),'--plugin','--port',str(ports[0]),'--mock','--no-build','--patch-dir',str(ROOT/'patches')],stdin=subprocess.PIPE,stdout=controller_log,stderr=subprocess.STDOUT,
                                  env={**os.environ,'MODULAR_MACHINE_STATE_DIR':str(runtime),'MK3_HIDDEN_PREVIEW':'1'})
        wait(lambda:(folder/'controller.log').read_text(),lambda t:'G1-EMU / LIVE' in t,20)
        launcher.stdin.write(b'button keyboard\npad 1 0.8\n');launcher.stdin.flush()
        wait(lambda:(folder/'controller.log').read_text(),lambda t:'"heldPads":[0]' in t,10)
        launcher.stdin.write(b'button stop\nquit\n');launcher.stdin.flush()
        assert launcher.wait(timeout=12)==0,(folder/'controller.log').read_text()[-2000:]
        controller_log.close();assert host.poll() is None
        assert 'Starting G1-Emu;' not in (folder/'controller.log').read_text()
        (folder/'result.json').write_text(json.dumps({'plugin':name,'ports':ports,'stateBytes':saved.stat().st_size,'stateSha256':hashlib.sha256(saved.read_bytes()).hexdigest()},indent=2))
        print('PASS plugin-only controller attachment, keyboard pads/STOP and clean controller exit; host stays alive',flush=True)
    finally:
        for rpc in peers:
            if rpc:
                try:rpc.close()
                except OSError:pass
        if launcher and launcher.poll() is None:launcher.terminate();launcher.wait(timeout=10)
        if host.poll() is None:
            try:command('quit');host.wait(timeout=12)
            except (OSError,subprocess.TimeoutExpired):host.send_signal(signal.SIGTERM);host.wait(timeout=10)
        log.close()
    assert host.returncode==0,path.read_text(errors='replace')[-3000:]
    print('Evidence:',folder,flush=True)


if __name__=='__main__':main()
