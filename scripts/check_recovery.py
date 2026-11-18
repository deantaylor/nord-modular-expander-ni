"""Sustained real bridge traffic, renderer reload and emulator-restart/no-replay checks."""
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import time
from check_g1 import ROOT, Rpc, wait
from check_mock import Session


def main():
    folder=ROOT/'g1-state'/('recovery-'+str(time.time_ns()));folder.mkdir()
    library=folder/'Library';library.mkdir()
    for name in ['01 First.pch','02 Second.pch','03 Third.pch']:(library/name).write_bytes((ROOT/'patches/MK3-Bridge.pch').read_bytes())
    with socket.socket() as probe:probe.bind(('127.0.0.1',0));port=probe.getsockname()[1]
    log=(folder/'host.log').open('w');host=None;rpc=None;panel=None
    def start():
        return subprocess.Popen([str(ROOT/'build/g1-emu/g1mk3check'),str(ROOT/'roms/NORD-MODULAR-RACK-VER-3.03.BIN'),str(folder/'flash.bin'),str(ROOT/'patches/MK3-Bridge.pch'),str(port),'180'],stdout=log,stderr=subprocess.STDOUT,
                                env={**os.environ,'G1_MODEL':'regular','G1_GAIN_DB':'-12','G1_MIDI_CLIENT':'MK3-Recovery-Test'})
    def connect():
        try:return Rpc(port)
        except (OSError,ValueError,AssertionError):return None
    def ready():
        return wait(lambda:rpc.call('status'),lambda s:s.get('requestedVoices') is not None and s['upload']['ok'] and s['knobs'][0]['assigned'],25)
    try:
        host=start();rpc=wait(connect,lambda r:r is not None);state=ready()
        assert state['engineThreadRequests'] is True
        for n in range(300):
            target=65 if n%2==0 else 64
            rpc.call('set_knobs',targets=[dict(index=0,value=target,binding=state['knobs'][0]['binding'])])
            state=wait(lambda:rpc.call('status'),lambda s:s['knobs'][0]['value']==target,3)
        # Burst requests exercise future ordering and bounded asynchronous response draining.
        ids=[]
        for _ in range(32):
            rpc.id+=1;ids.append(rpc.id)
            rpc.file.write((json.dumps({'id':rpc.id,'method':'status'})+'\n').encode())
        rpc.file.flush()
        for expected in ids:
            reply=json.loads(rpc.file.readline());assert reply['id']==expected,reply
            assert reply['result']['running']
        print('PASS 300 encoder changes and 32 pipelined status replies on engine thread',flush=True)
        old_binding=state['knobs'][0]['binding']
        rpc.call('load_file',slot=0,path=str(library/'01 First.pch'))
        state=wait(lambda:rpc.call('status'),lambda s:s['upload']['ok'] and not s['upload']['busy'] and s['knobs'][0]['binding']!=old_binding,20)
        try:rpc.call('set_knobs',targets=[dict(index=0,value=99,binding=old_binding)])
        except RuntimeError as error:assert 'Stale' in str(error)
        else:raise AssertionError('Old patch identity was accepted')
        for _ in range(8):
            rpc.call('function',pressed=True);rpc.close();rpc=None
            rpc=wait(connect,lambda r:r is not None)
            assert not rpc.call('status')['configuration']['function']
        print('PASS patch identity invalidation and modifier release across 8 client reconnects',flush=True)
        rpc.close();rpc=None;time.sleep(.1)
        previous_trace=os.environ.get('MK3_TRACE_FRAMES');os.environ['MK3_TRACE_FRAMES']='1'
        panel=Session(['--mock','--g1-address',f'127.0.0.1:{port}','--patch-dir',str(library)])
        if previous_trace is None:os.environ.pop('MK3_TRACE_FRAMES',None)
        else:os.environ['MK3_TRACE_FRAMES']=previous_trace
        panel.wait_for(lambda:panel.state().get('engine')=='G1-EMU / LIVE')
        panel.command('button macro\nbutton macro\nbutton browser\nscroll 2')
        panel.wait_for(lambda:panel.state()['page']==2 and panel.state()['browserCursor']==2 and panel.state()['browserOpen'])
        panel.command('reload-ui');panel.wait_for(lambda:any('mock views reloading' in l for l in panel.lines))
        time.sleep(.7);panel.command('inspect');panel.wait_for(lambda:sum(l.startswith('mock DOM') for l in panel.lines)==2)
        panes=[json.loads(l.split(': ',1)[1]) for l in panel.lines if l.startswith('mock DOM')]
        assert all(p['knobsHidden'] for p in panes)
        left=next(p for p in panes if p['browserOpen']);assert abs(left['scrollTarget']-2)<.01,left
        print('PASS renderer reload replays current Browser state without changing engine',flush=True)
        # Stop only the private helper, send inputs while disconnected, restart same port.
        host.send_signal(signal.SIGTERM);host.wait(timeout=10)
        panel.wait_for(lambda:panel.state()['engine']=='G1-EMU / NOT READY')
        assert panel.state()['page']==2 and panel.state()['browserCursor']==2
        panel.command('turn 1 0.7\nturn 2 -0.8')
        host=start()
        panel.wait_for(lambda:panel.state()['engine']=='G1-EMU / LIVE' and not panel.state()['loading'],30)
        assert panel.state()['page']==2 and panel.state()['browserCursor']==2
        panel.command('button browser\nbutton macro')
        panel.wait_for(lambda:not panel.state()['browserOpen'] and panel.state()['page']==0)
        panel.wait_for(lambda:panel.state()['displayValues'][0]=='E4' and panel.state()['displayValues'][1]=='100',25)
        assert not any('JavaScript error' in l for l in panel.lines)
        print('PASS emulator restart, stale-value clearing, preserved navigation and no stale encoder replay',flush=True)
        panel.command('quit');assert panel.process.wait(timeout=5)==0
    finally:
        if panel:panel.close()
        if rpc:rpc.close()
        if host and host.poll() is None:host.send_signal(signal.SIGTERM);host.wait(timeout=10)
        log.close()
    print('PASS stability/recovery; evidence:',folder)

if __name__=='__main__':main()
