"""Real assigned controls on pages 1/3, empty page2, labels and slot independence."""
import json
import os
import signal
import socket
import subprocess
import time
from check_g1 import ROOT,Rpc,wait
from check_mock import Session


def main():
    folder=ROOT/'g1-state'/('control-pages-'+str(time.time_ns()));folder.mkdir()
    patch=folder/'Pages.pch'
    text=(ROOT/'patches/MK3-Bridge.pch').read_text().replace('1 2 0 1\n[/KnobMapDump]','1 2 0 1\n1 1 1 16\n1 2 1 17\n[/KnobMapDump]')
    patch.write_text(text)
    with socket.socket() as probe:probe.bind(('127.0.0.1',0));port=probe.getsockname()[1]
    log=(folder/'host.log').open('w');host=subprocess.Popen([str(ROOT/'build/g1-emu/g1mk3check'),str(ROOT/'roms/NORD-MODULAR-RACK-VER-3.03.BIN'),str(folder/'flash.bin'),str(patch),str(port),'60'],stdout=log,stderr=subprocess.STDOUT,
                                                      env={**os.environ,'G1_MODEL':'regular','G1_GAIN_DB':'-12','G1_MIDI_CLIENT':'MK3-Control-Pages-Test'})
    rpc=None;panel=None
    try:
        def connect():
            try:return Rpc(port)
            except (OSError,ValueError):return None
        rpc=wait(connect,lambda r:r is not None)
        wait(lambda:rpc.call('status'),lambda s:len(s['knobs'])==18 and s['knobs'][16]['assigned'] and s['knobs'][17]['assigned'],20)
        rpc.close();rpc=None;time.sleep(.1)
        panel=Session(['--mock','--g1-address',f'127.0.0.1:{port}','--patch-dir',str(folder)])
        panel.wait_for(lambda:panel.state().get('engine')=='G1-EMU / LIVE')
        panel.command('capture-preview '+str(folder/'normal'));panel.wait_for(lambda:any('preview captured:' in l for l in panel.lines))
        panel.command('button macro');panel.wait_for(lambda:panel.state()['page']==1)
        assert all(v=='—' for v in panel.state()['displayValues'])
        panel.command('button macro');panel.wait_for(lambda:panel.state()['page']==2)
        assert all(v=='—' for v in panel.state()['displayValues'][2:])
        assert all(v!='—' for v in panel.state()['displayValues'][:2])
        before=panel.state()['values'][0];panel.command('turn 1 0.02');panel.wait_for(lambda:panel.state()['values'][0]!=before)
        panel.command('inspect');panel.wait_for(lambda:sum(l.startswith('mock DOM') for l in panel.lines)==2)
        panes={side:json.loads(next(l.split(': ',1)[1] for l in reversed(panel.lines) if l.startswith('mock DOM '+side))) for side in ['left','right']}
        assert all(len(p['knobs'])==4 for p in panes.values())
        assert [k['encoder'] for k in panes['left']['knobs']+panes['right']['knobs']]==[str(n) for n in range(1,9)]
        assert all(k['bottom']<=257 for p in panes.values() for k in p['knobs'])
        panel.command('capture-preview '+str(folder/'last-page'));panel.wait_for(lambda:any('preview captured: '+str(folder/'last-page') in l for l in panel.lines))
        panel.command('button group_b');panel.wait_for(lambda:panel.state()['slot']=='B');assert panel.state()['page']==2
        panel.command('button group_a');panel.wait_for(lambda:panel.state()['slot']=='A');assert panel.state()['page']==2
        panel.command('button browser');panel.wait_for(lambda:panel.state()['browserOpen'])
        panel.command('button browser');panel.wait_for(lambda:not panel.state()['browserOpen']);assert panel.state()['page']==2
        panel.command('quit');assert panel.process.wait(timeout=5)==0
        print('PASS actual page1/page3 assignments, empty page2, final six inactive controls, label geometry and independent slot/browser/page state')
    finally:
        if panel:panel.close()
        if rpc:rpc.close()
        if host.poll() is None:host.send_signal(signal.SIGTERM);host.wait(timeout=10)
        log.close()
    print('Evidence:',folder)

if __name__=='__main__':main()
