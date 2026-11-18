"""Actual process exit/relaunch browser preferences and missing-folder fallback."""
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
    folder=ROOT/'g1-state'/('browser-state-'+str(time.time_ns()));folder.mkdir()
    library=folder/'Library';(library/'Nested').mkdir(parents=True)
    for name in ['A.pch','B.pch']:(library/'Nested'/name).write_bytes((ROOT/'patches/MK3-Bridge.pch').read_bytes())
    (library/'Root.pch').write_bytes((ROOT/'patches/MK3-Bridge.pch').read_bytes())
    preferences=folder/'browser-state.json';os.environ['MK3_BROWSER_STATE']=str(preferences)
    with socket.socket() as probe:probe.bind(('127.0.0.1',0));port=probe.getsockname()[1]
    log=(folder/'host.log').open('w')
    host=subprocess.Popen([str(ROOT/'build/g1-emu/g1mk3check'),str(ROOT/'roms/NORD-MODULAR-RACK-VER-3.03.BIN'),str(folder/'flash.bin'),str(ROOT/'patches/MK3-Bridge.pch'),str(port),'90'],stdout=log,stderr=subprocess.STDOUT,
                          env={**os.environ,'G1_MODEL':'regular','G1_GAIN_DB':'-12','G1_MIDI_CLIENT':'MK3-Browser-State-Test'})
    panel=None;rpc=None
    def connect():
        try:return Rpc(port)
        except (OSError,ValueError):return None
    def start():
        result=Session(['--mock','--g1-address',f'127.0.0.1:{port}','--patch-dir',str(library)])
        result.wait_for(lambda:result.state().get('engine')=='G1-EMU / LIVE');return result
    def stop():
        panel.command('quit');assert panel.process.wait(timeout=5)==0;panel.close();time.sleep(.1)
    try:
        rpc=wait(connect,lambda r:r is not None);wait(lambda:rpc.call('status'),lambda s:s.get('requestedVoices') is not None)
        rpc.close();rpc=None;time.sleep(.1)
        panel=start();panel.command('button browser\nbutton encoder_press')
        panel.wait_for(lambda:panel.state()['browserFolder']=='Nested')
        panel.command('scroll 1');panel.wait_for(lambda:panel.state()['browserCursor']==1);stop();panel=None
        saved=json.loads(preferences.read_text());assert saved['folder']=='Nested' and saved['selected']=='B.pch'
        panel=start();assert panel.state()['browserFolder']=='Nested' and panel.state()['browserCursor']==1
        panel.command('button browser\nbutton top_2');panel.wait_for(lambda:panel.state()['browserSource']=='banks' and panel.state()['presetCount']>0)
        stop();panel=None
        for path in (library/'Nested').iterdir():path.unlink()
        (library/'Nested').rmdir()
        panel=start();assert panel.state()['browserFolder']=='Library' and panel.state()['browserSource']=='banks'
        panel.command('button browser\nbutton top_1');panel.wait_for(lambda:panel.state()['browserSource']=='files' and panel.state()['browserCount']==1)
        assert panel.state()['browserCursor']==0;stop();panel=None
        preferences.write_text('{invalid preferences')
        panel=start();stop();panel=None
        assert preferences.read_text()=='{invalid preferences','invalid preferences must be preserved'
        print('PASS folder/entry/source across real process relaunches, deleted folder fallback and corrupt preference preservation')
    finally:
        if panel:panel.close()
        if rpc:rpc.close()
        if host.poll() is None:host.send_signal(signal.SIGTERM);host.wait(timeout=10)
        log.close()
    print('Evidence:',folder)

if __name__=='__main__':main()
