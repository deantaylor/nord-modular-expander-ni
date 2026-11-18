"""Real ROM/DSP + CoreAudio + local bridge check, with an isolated bank file.
Requires the locally patched g1mk3check target and controller debug build. No ROM downloads.
"""
import argparse
import json
import math
import os
from pathlib import Path
import signal
import socket
import subprocess
import time
import wave
from check_mock import Session

ROOT=Path(__file__).resolve().parents[1]
class Rpc:
    def __init__(self,port):
        self.socket=socket.create_connection(('127.0.0.1',port),timeout=2)
        self.socket.settimeout(2);self.file=self.socket.makefile('rwb');self.id=0
        hello=json.loads(self.file.readline(1024*1024))
        assert hello['service']=='modular-machine-g1' and hello['protocol']==1,hello
    def call(self,method,**fields):
        self.id+=1;self.file.write((json.dumps(dict(id=self.id,method=method,**fields))+'\n').encode());self.file.flush()
        reply=json.loads(self.file.readline(1024*1024));assert reply['id']==self.id
        if 'error' in reply:raise RuntimeError(reply['error'])
        return reply['result']
    def close(self):self.file.close();self.socket.close()

def wait(function,predicate,timeout=15):
    deadline=time.monotonic()+timeout;latest=None
    while time.monotonic()<deadline:
        try:
            latest=function()
            if predicate(latest):return latest
        except (OSError,ValueError) as error:latest=str(error)
        time.sleep(.1)
    raise RuntimeError('Timed out: '+str(latest))

def analyze(path):
    with wave.open(str(path),'rb') as wav:
        assert wav.getsampwidth()==3 and wav.getnchannels()==4
        rate=wav.getframerate()/4
        raw=wav.readframes(wav.getnframes())
    samples=[int.from_bytes(raw[i:i+3],'little',signed=True)/8388608 for i in range(0,len(raw),48)]
    size=int(rate/4);windows=[]
    for start in range(0,len(samples)-size,size):
        values=samples[start:start+size]
        # G1_RECORD contains raw DSP words; remove the hardware DAC DC offset.
        mean=sum(values)/size;values=[v-mean for v in values];rms=math.sqrt(sum(v*v for v in values)/size)
        crosses=[i for i in range(1,size) if values[i-1]<=0<values[i]]
        if len(crosses)>4 and rms>1e-8:
            frequency=(len(crosses)-1)*rate/(crosses[-1]-crosses[0])
            windows.append((frequency,rms))
    low=[(f,r) for f,r in windows if 310<f<350]
    high=[(f,r) for f,r in windows if 620<f<700]
    assert low and high,windows
    levels=sorted(r for _,r in high)
    assert levels[-1]/levels[0]>1.3,levels
    print('Audio waveform PASS: ~%.1f Hz and ~%.1f Hz; changed level ratio %.2f' %(sum(f for f,_ in low)/len(low),sum(f for f,_ in high)/len(high),levels[-1]/levels[0]))

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--host-check',type=Path,default=ROOT/'build/g1-emu/g1mk3check');parser.add_argument('--rom',type=Path,default=ROOT/'roms/NORD-MODULAR-RACK-VER-3.03.BIN');args=parser.parse_args()
    folder=ROOT/'g1-state'/('verification-'+str(time.time_ns()));folder.mkdir(parents=True)
    with socket.socket() as probe:probe.bind(('127.0.0.1',0));port=probe.getsockname()[1]
    log=(folder/'host.log').open('w');host=subprocess.Popen([str(args.host_check.resolve()),str(args.rom.resolve()),str(folder/'flash.bin'),str(ROOT/'patches/MK3-Bridge.pch'),str(port),'60'],stdout=log,stderr=subprocess.STDOUT,env={**os.environ,'G1_MODEL':'regular','G1_GAIN_DB':'0','G1_RECORD':'120'})
    library=folder/'patches';library.mkdir()
    original=(ROOT/'patches/MK3-Bridge.pch').read_text()
    (library/'01 Original.pch').write_text(original)
    (library/'02 Bright.pch').write_text(original.replace('1 7 10 64 64', '1 7 10 69 64').replace('2 19 3 100 0 0', '2 19 3 80 0 0'))
    for index in range(3,9):(library/('%02d Extra.pch'%index)).write_text(original)
    rpc=None;panel=None
    try:
        def connect():
            try:return Rpc(port)
            except (OSError,ValueError):return None
        rpc=wait(connect,lambda value:value is not None)
        state=wait(lambda:rpc.call('status'),lambda value:len(value['knobs'])==18 and all(k['assigned'] for k in value['knobs'][:2]),timeout=25)
        state=wait(lambda:rpc.call('status'),lambda value:'MK3-Bridge' in value['lcd'][0])
        initial=[k['value'] for k in state['knobs'][:2]]
        print('Actual G1 assignments:',[(k['module'],k['parameter'],k['value'],k['display']) for k in state['knobs'][:2]])
        rpc.call('presets',refresh=True)
        presets=wait(lambda:rpc.call('presets',refresh=False),lambda value:bool(value['entries']))
        assert any(p['name']=='MK3-Bridge' for p in presets['entries']),presets
        try:rpc.call('set_knobs',targets=[dict(index=0,value=0,binding='stale-binding')])
        except RuntimeError as error:assert 'Stale' in str(error)
        else:raise AssertionError('Stale assignment was accepted')
        time.sleep(1)
        for index,target in [(0,initial[0]+12),(1,initial[1]-30)]:
            rpc.call('set_knobs',targets=[dict(index=index,value=target,binding=state['knobs'][index]['binding'])])
            state=wait(lambda:rpc.call('status'),lambda value:value['knobs'][index]['value']==target)
            print('Confirmed G1 parameter',index+1,':',state['knobs'][index]['display']);time.sleep(1)
        rpc.call('set_knobs',targets=[dict(index=i,value=value,binding=state['knobs'][i]['binding']) for i,value in enumerate(initial)])
        wait(lambda:rpc.call('status'),lambda value:[k['value'] for k in value['knobs'][:2]]==initial)
        configuration=rpc.call('status')['configuration']
        assert len(configuration['controls'])>=19,configuration
        master=next(c for c in configuration['controls'] if c['key']=='master')['value']
        rpc.call('set_control',key='master',value=max(0,master-30))
        wait(lambda:rpc.call('status'),lambda value:next(c for c in value['configuration']['controls'] if c['key']=='master')['value']==max(0,master-30))
        rpc.call('set_control',key='master',value=master)
        # The ROM ignores ordinary patch-knob movements while its Function/Shift input is held.
        rpc.call('function',pressed=True);time.sleep(.1)
        state=rpc.call('status');assert state['configuration']['function']
        rpc.call('set_knobs',targets=[dict(index=0,value=initial[0]+2,binding=state['knobs'][0]['binding'])])
        time.sleep(.2);assert rpc.call('status')['knobs'][0]['value']==initial[0]
        rpc.call('function',pressed=False);time.sleep(.1)
        rpc.call('set_knobs',targets=[dict(index=0,value=initial[0]+3,binding=state['knobs'][0]['binding'])])
        wait(lambda:rpc.call('status'),lambda value:value['knobs'][0]['value']==initial[0]+3)
        rpc.call('set_knobs',targets=[dict(index=0,value=initial[0],binding=state['knobs'][0]['binding'])])
        wait(lambda:rpc.call('status'),lambda value:value['knobs'][0]['value']==initial[0])
        rpc.call('function',pressed=True)
        rpc.close();time.sleep(.15);rpc=Rpc(port)
        assert not rpc.call('status')['configuration']['function'],'Function stuck after client disconnect'
        print('Real emulator controls PASS: master ADC + ROM Function/Shift gating and release')
        rpc.close();rpc=None
        panel=Session(['--mock','--g1-address',f'127.0.0.1:{port}','--patch-dir',str(library)])
        panel.wait_for(lambda:panel.state().get('engine')=='G1-EMU / LIVE')
        panel.wait_for(lambda:panel.state().get('presetCount',0)>0)
        before=panel.state()['values'][:2]
        panel.command('turn 1 0.1');panel.wait_for(lambda:panel.state()['values'][0]>before[0])
        panel.command('turn 2 -0.1');panel.wait_for(lambda:panel.state()['values'][1]<before[1])
        panel.command('inspect');panel.wait_for(lambda:sum(line.startswith('mock DOM') for line in panel.lines)==2)
        right=json.loads(next(line.split(': ',1)[1] for line in panel.lines if line.startswith('mock DOM right')))
        left=json.loads(next(line.split(': ',1)[1] for line in panel.lines if line.startswith('mock DOM left')))
        assert panel.state()['displayValues'][0] in left['text'],left
        assert len(left['knobs'])==4 and len(right['knobs'])==4,(left,right)
        assert all(any(face['family']=='Inter' and face['status']=='loaded' for face in pane['fontFaces']) for pane in [left,right]),(left,right)
        assert left['background']=='rgb(20, 20, 20)',left
        assert all(k['weight']=='150' for k in left['knobs']+right['knobs']),(left,right)
        assert [k['encoder'] for k in left['knobs']]==['1','2','3','4'],left
        assert [k['encoder'] for k in right['knobs']]==['5','6','7','8'],right
        assert all(184<=k['top'] and k['bottom']<=257 for k in left['knobs']+right['knobs']),(left,right)
        panel.command('capture-preview '+str(folder/'normal'))
        panel.wait_for(lambda:any('preview captured: '+str(folder/'normal') in line for line in panel.lines))
        panel.command('button browser')
        panel.wait_for(lambda:panel.state().get('browserOpen') and panel.state().get('browserCount')==8)
        panel.command('scroll 1')
        panel.wait_for(lambda:panel.state()['browserCursor']==1)
        inspected=sum(line.startswith('mock DOM') for line in panel.lines)
        panel.command('inspect');panel.wait_for(lambda:sum(line.startswith('mock DOM') for line in panel.lines)==inspected+2)
        moving=json.loads(next(line.split(': ',1)[1] for line in reversed(panel.lines) if line.startswith('mock DOM left')))
        assert moving['animationUntil']>moving['now'],moving
        moving_row=next(row for row in moving['rows'] if row['index']=='1')
        time.sleep(.25)
        inspected=sum(line.startswith('mock DOM') for line in panel.lines)
        panel.command('inspect');panel.wait_for(lambda:sum(line.startswith('mock DOM') for line in panel.lines)==inspected+2)
        settled=json.loads(next(line.split(': ',1)[1] for line in reversed(panel.lines) if line.startswith('mock DOM left')))
        settled_row=next(row for row in settled['rows'] if row['index']=='1')
        assert abs(settled_row['top']-11)<.5,settled_row
        assert moving_row['top']>settled_row['top'],(moving_row,settled_row)
        panel.command('button encoder_press')
        panel.wait_for(lambda:panel.state()['status']=='Loaded 02 Bright.pch' and not panel.state()['loading'])
        assert abs(panel.state()['values'][0]-69/127)<1e-9 and abs(panel.state()['values'][1]-80/127)<1e-9,panel.state()
        inspected=sum(line.startswith('mock DOM') for line in panel.lines)
        panel.command('inspect');panel.wait_for(lambda:sum(line.startswith('mock DOM') for line in panel.lines)==inspected+2)
        left=json.loads(next(line.split(': ',1)[1] for line in reversed(panel.lines) if line.startswith('mock DOM left')))
        assert left['browserOpen'] and any(row['name']=='02 Bright.pch' for row in left['rows']),left
        assert all(row['top']>=11 for row in left['rows'] if row['index']==str(panel.state()['browserCursor'])),left
        time.sleep(.6)
        panel.command('capture-preview '+str(folder/'browser'))
        panel.wait_for(lambda:any('preview captured: '+str(folder/'browser') in line for line in panel.lines))
        panel.command('button top_2')
        panel.wait_for(lambda:panel.state().get('browserSource')=='banks')
        panel.command('button top_4')
        panel.wait_for(lambda:all(abs(panel.state()['values'][i]-before[i])<1e-9 for i in range(2)))
        panel.command('button browser');panel.wait_for(lambda:not panel.state()['browserOpen'])
        for group,slot in [('b','B'),('c','C'),('d','D'),('a','A')]:
            panel.command('button group_'+group);panel.wait_for(lambda:panel.state()['slot']==slot)
            assert panel.state()['leds']['group_'+group]==80,panel.state()
        for page in [1,2,0]:
            panel.command('button macro');panel.wait_for(lambda:panel.state()['page']==page)
        panel.command('button shift');panel.wait_for(lambda:panel.state()['configuration']['function'])
        panel.command('handover');panel.wait_for(lambda:not panel.state()['configuration']['function'])
        panel.command('takeover');panel.wait_for(lambda:panel.state()['connection']['hasControl'])
        panel.command('button settings');panel.wait_for(lambda:panel.state()['view']=='settings')
        def control(key):return next(c for c in panel.state()['configuration']['controls'] if c['key']==key)
        bpm=control('bpm')['value']
        panel.command('turn 2 0.02');panel.wait_for(lambda:control('bpm')['value']>bpm)
        assert panel.state()['displayValues'][1]==control('bpm')['display'],panel.state()
        inspected=sum(line.startswith('mock DOM') for line in panel.lines)
        panel.command('inspect');panel.wait_for(lambda:sum(line.startswith('mock DOM') for line in panel.lines)==inspected+2)
        settings_left=json.loads(next(line.split(': ',1)[1] for line in reversed(panel.lines) if line.startswith('mock DOM left')))
        settings_right=json.loads(next(line.split(': ',1)[1] for line in reversed(panel.lines) if line.startswith('mock DOM right')))
        assert 'settings' in settings_left['text'].lower() and 'clock tempo' in settings_left['text'].lower() and settings_left['knobs'][1]['display']==control('bpm')['display'],settings_left
        assert 'slot a midi' in settings_right['text'].lower() and len(settings_right['knobs'])==4,settings_right
        panel.command('capture-preview '+str(folder/'settings'))
        panel.wait_for(lambda:any('preview captured: '+str(folder/'settings') in line for line in panel.lines))
        panel.command('button mixer');panel.wait_for(lambda:panel.state()['view']=='mixer')
        master=control('master')['value']
        panel.command('turn 1 -0.05');panel.wait_for(lambda:control('master')['value']<master)
        inspected=sum(line.startswith('mock DOM') for line in panel.lines)
        panel.command('inspect');panel.wait_for(lambda:sum(line.startswith('mock DOM') for line in panel.lines)==inspected+2)
        mixer_left=json.loads(next(line.split(': ',1)[1] for line in reversed(panel.lines) if line.startswith('mock DOM left')))
        assert 'master volume' in mixer_left['text'].lower() and control('master')['display'] in mixer_left['text'],mixer_left
        panel.command('capture-preview '+str(folder/'mixer'))
        panel.wait_for(lambda:any('preview captured: '+str(folder/'mixer') in line for line in panel.lines))
        panel.command('button mixer');panel.wait_for(lambda:panel.state()['view']=='parameters')
        panel.command('button browser');panel.wait_for(lambda:panel.state()['browserOpen'])
        panel.command('button top_1');panel.wait_for(lambda:panel.state()['browserSource']=='files')
        panel.command('button arrow_right');panel.wait_for(lambda:panel.state()['status']=='Loaded 03 Extra.pch')
        panel.command('button arrow_left');panel.wait_for(lambda:panel.state()['status']=='Loaded 02 Bright.pch')
        print('User mappings PASS: A-D slots + Macro pages + Function/handover release + real Settings tempo + Mixer master volume + LED state')
        print('Rust controller PASS: live parameters + Browser button file selection/upload + rendered list + bank patch reload')
        panel.command('quit');assert panel.process.wait(timeout=5)==0
        panel.close();panel=None
        # The controller exiting must not shut down the emulator.
        assert host.poll() is None
    finally:
        if rpc:rpc.close()
        if panel:panel.close()
        if host.poll() is None:
            host.send_signal(signal.SIGTERM)
            try:host.wait(timeout=10)
            except subprocess.TimeoutExpired:host.kill();host.wait();raise
        log.close()
    text=(folder/'host.log').read_text(errors='replace')
    assert host.returncode==0,text[-5000:]
    assert 'audio=' in text and 'no sound:' not in text and 'outputs' in text,text[-1000:]
    analyze(folder/'output.wav')
    print('PASS: real G1 ROM, isolated stored patch, CoreAudio host, two controls, patch reload; evidence:',folder)

if __name__=='__main__':main()
