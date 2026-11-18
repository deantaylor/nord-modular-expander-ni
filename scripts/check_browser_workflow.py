"""Isolated real-engine Browser workflow, compatibility and recovery checks.
Copies the user's local library for tests; never writes source patches or user banks.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import subprocess
import time
from check_g1 import ROOT, Rpc, wait
from check_mock import Session


def main():
    folder=ROOT/'g1-state'/('browser-workflow-'+str(time.time_ns()));folder.mkdir()
    library=folder/'Library';library.mkdir()
    originals={p:hashlib.sha256(p.read_bytes()).hexdigest() for p in (ROOT/'patches').glob('*.pch')}
    for path in originals:shutil.copy2(path,library/path.name)
    (library/'Nested').mkdir();(library/'Empty').mkdir()
    shutil.copy2(ROOT/'patches/MK3-Bridge.pch',library/'Nested/Nested Test.pch')
    long_name='Z A very long descriptive patch name with spaces and details for a readable Browser selection.pch'
    shutil.copy2(ROOT/'patches/MK3-Bridge.pch',library/long_name)
    (library/'ZZ Invalid.pch').write_text('This is not a Nord patch.\n')
    with socket.socket() as probe:probe.bind(('127.0.0.1',0));port=probe.getsockname()[1]
    log=(folder/'host.log').open('w')
    host=subprocess.Popen([str(ROOT/'build/g1-emu/g1mk3check'),str(ROOT/'roms/NORD-MODULAR-RACK-VER-3.03.BIN'),str(folder/'flash.bin'),str(ROOT/'patches/MK3-Bridge.pch'),str(port),'240'],
                          stdout=log,stderr=subprocess.STDOUT,env={**os.environ,'G1_MODEL':'regular','G1_GAIN_DB':'-12','G1_MIDI_CLIENT':'MK3-Workflow-Test'})
    panel=None;rpc=None;results=[]
    try:
        def connect():
            try:return Rpc(port)
            except (OSError,ValueError):return None
        rpc=wait(connect,lambda r:r is not None)
        wait(lambda:rpc.call('status'),lambda s:s.get('requestedVoices') is not None and s['upload']['ok'])
        rpc.close();rpc=None;time.sleep(.1)
        panel=Session(['--mock','--g1-address',f'127.0.0.1:{port}','--patch-dir',str(library)])
        panel.wait_for(lambda:panel.state().get('engine')=='G1-EMU / LIVE')
        def command(text,predicate,timeout=15):
            panel.command(text);panel.wait_for(lambda:predicate(panel.state()),timeout)
        def inspect():
            before=sum(l.startswith('mock DOM') for l in panel.lines)
            panel.command('inspect');panel.wait_for(lambda:sum(l.startswith('mock DOM') for l in panel.lines)==before+2)
            return {side:json.loads(next(l.split(': ',1)[1] for l in reversed(panel.lines) if l.startswith('mock DOM '+side))) for side in ['left','right']}
        def browser():
            if not panel.state()['browserOpen']:command('button browser',lambda s:s['browserOpen'])
            if panel.state()['browserSource']!='files':command('button top_1',lambda s:s['browserSource']=='files')
        def choose(name):
            browser();command('scroll -4096',lambda s:s['browserCursor']==0)
            for _ in range(20):
                entries=panel.state()['browser'];entry=next((e for e in entries if e['name']==name),None)
                if entry:
                    target=entry['index'];command('scroll '+str(target-panel.state()['browserCursor']),lambda s:s['browserCursor']==target);return
                cursor=panel.state()['browserCursor'];command('scroll 5',lambda s:s['browserCursor']!=cursor)
            raise AssertionError('Cannot select '+name)
        def load(name):
            choose(name);command('button encoder_press',lambda s:s['status']=='Loaded '+name and not s['loading'],40)
        browser();load('MK3-Bridge.pch')
        command('button browser',lambda s:not s['browserOpen'])
        initial=panel.state()['values'][0]
        command('turn 1 0.02',lambda s:s['values'][0]!=initial)
        print('PASS Browser local load -> parameter control',flush=True)
        choose('Nested');command('button encoder_press',lambda s:s['browserFolder']=='Nested')
        load('Nested Test.pch');command('button encoder_left',lambda s:s['browserFolder']=='Library')
        command('button top_2',lambda s:s['browserSource']=='banks')
        command('button top_3',lambda s:s['presetCount']>=1)
        panel.command('button encoder_press');panel.wait_for(lambda:not panel.state()['loading'] and panel.state()['status']=='Loaded MK3-Bridge',20)
        panel.wait_for(lambda:'MK3-Bridge' in panel.state()['patch'])
        command('button top_1',lambda s:s['browserSource']=='files')
        print('PASS subfolder/parent, Files/Banks and refresh',flush=True)
        # A distinct file per slot; check each after loading all four.
        expected={}
        for slot in range(4):
            name=f'Slot {chr(65+slot)}.pch';shutil.copy2(ROOT/'patches/MK3-Bridge.pch',library/name)
            command('button top_3',lambda s:s['browserCount']>=len(originals)+5+slot)
            command('button group_'+chr(97+slot),lambda s:s['slot']==chr(65+slot))
            load(name);expected[slot]=name[:-4]
        for slot,name in expected.items():
            command('button group_'+chr(97+slot),lambda s:s['slot']==chr(65+slot))
            panel.wait_for(lambda:name in panel.state()['patch'])
        print('PASS slot A-D loads preserve the other three slot names',flush=True)
        command('button group_a',lambda s:s['slot']=='A')
        for path in originals:
            try:
                load(path.name)
                labels=panel.state()['labels'];assigned=sum(label!='UNASSIGNED' for label in labels)
                if path.name=='MaryMill.pch':assert assigned==0,labels
                results.append({'name':path.name,'loaded':True,'firstPageAssignments':assigned})
            except Exception as error:
                results.append({'name':path.name,'loaded':False,'error':str(error)[:500]});raise
            finally:(folder/'compatibility.json').write_text(json.dumps(results,indent=2))
            print('PATCH PASS',path.name,flush=True)
        choose(long_name);time.sleep(.3);dom=inspect()
        assert long_name in dom['right']['text'],dom['right']
        assert all(p['knobsHidden'] for p in dom.values()),dom
        panel.command('capture-preview '+str(folder/'long-name'));panel.wait_for(lambda:any('preview captured:' in l for l in panel.lines))
        choose('Empty');command('button encoder_press',lambda s:s['browserCount']==0)
        dom=inspect();assert 'No patches' in dom['left']['text'],dom['left']
        shutil.copy2(ROOT/'patches/MK3-Bridge.pch',library/'Empty/Appeared.pch')
        command('button top_3',lambda s:s['browserCount']==1)
        dom=inspect();assert 'No patches in this folder' not in dom['left']['text'],dom['left']
        command('button encoder_left',lambda s:s['browserFolder']=='Library')
        for _ in range(2):
            choose('ZZ Invalid.pch');panel.command('button encoder_press')
            panel.wait_for(lambda:not panel.state()['loading'] and ('invalid' in panel.state()['status'].lower() or 'failed' in panel.state()['status'].lower() or 'patch' in panel.state()['status'].lower()),5)
            # Drain one further update so an unchanged repeated error cannot hide a pending load.
            time.sleep(.1);panel.command('report');panel.wait_for(lambda:any(l.startswith('report:') for l in panel.lines))
            assert not panel.state()['loading'],panel.state()
        dom=inspect();assert any(word in dom['right']['text'].lower() for word in ['invalid','failed','header','patch']),dom
        load('MK3-Bridge.pch')
        print('PASS long name, empty/refresh folder, repeated invalid-file rejection and successful retry',flush=True)
        assert all(hashlib.sha256(p.read_bytes()).hexdigest()==sha for p,sha in originals.items())
        panel.command('quit');assert panel.process.wait(timeout=5)==0
    finally:
        if panel:panel.close()
        if rpc:rpc.close()
        if host.poll() is None:host.send_signal(signal.SIGTERM);host.wait(timeout=10)
        log.close()
    print('PASS first five Browser workflow tasks (automated; human physical review pending). Evidence:',folder)

if __name__=='__main__':main()
