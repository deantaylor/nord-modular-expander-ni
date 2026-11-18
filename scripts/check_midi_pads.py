"""Real-ROM MIDI pad receipt, gate audio, mode/release and rapid-encoder regressions.
Uses only project-owned patches, private ports/flash and hidden controller previews.
"""
import json
import math
import os
from pathlib import Path
import signal
import socket
import subprocess
import time
import wave
from check_g1 import ROOT, Rpc, wait
from check_mock import Session


def main():
    folder = ROOT/'g1-state'/('midi-pads-'+str(time.time_ns()))
    folder.mkdir()
    with socket.socket() as probe:
        probe.bind(('127.0.0.1',0)); port=probe.getsockname()[1]
    log=(folder/'host.log').open('w')
    host=subprocess.Popen([str(ROOT/'build/g1-emu/g1mk3check'),str(ROOT/'roms/NORD-MODULAR-RACK-VER-3.03.BIN'),str(folder/'flash.bin'),str(ROOT/'patches/MK3-MIDI.pch'),str(port),'100'],stdout=log,stderr=subprocess.STDOUT,
                          env={**os.environ,'G1_MODEL':'regular','G1_GAIN_DB':'-12','G1_MIDI_CLIENT':'MK3-Pads-Test','G1_MIDI_LOG':'1','G1_RECORD':'100'})
    rpc=None; panel=None
    try:
        def connect():
            try:return Rpc(port)
            except OSError:return None
        rpc=wait(connect,lambda r:r is not None)
        status=wait(lambda:rpc.call('status'),lambda s:s.get('padMidi') and s.get('requestedVoices')==8 and s['knobs'][0]['assigned'],25)
        assert status['padMidiHeld']==[]
        initial=status['midiInBytes']
        try:rpc.call('pad_midi',pad=0,slot=0,note=128,velocity=100)
        except RuntimeError as error:assert 'Invalid integer' in str(error)
        else:raise AssertionError('accepted invalid MIDI note')
        time.sleep(.7)
        rpc.call('pad_midi',pad=0,slot=0,note=48,velocity=64)
        wait(lambda:rpc.call('status'),lambda s:s['padMidiHeld']==[0] and s['midiInBytes']>=initial+3)
        time.sleep(.8)
        before=rpc.call('status')['midiInBytes']
        for _ in range(30):rpc.call('pad_midi',pad=0,slot=0,note=48,velocity=127)
        assert rpc.call('status')['midiInBytes']==before,'duplicate strike retriggered'
        rpc.call('pad_midi',pad=0,slot=0,note=48,velocity=0)
        wait(lambda:rpc.call('status'),lambda s:s['padMidiHeld']==[] and s['midiInBytes']>=before+3)
        time.sleep(.7)
        rpc.call('pad_midi',pad=1,slot=0,note=60,velocity=127)
        time.sleep(.8)
        rpc.call('pad_panic')
        wait(lambda:rpc.call('status'),lambda s:s['padMidiHeld']==[])
        time.sleep(.7)
        # A held note keeps its original channel even when configuration changes.
        rpc.call('pad_midi',pad=2,slot=0,note=52,velocity=90)
        old_channel=next(c['value'] for c in rpc.call('status')['configuration']['controls'] if c['key']=='midi0')
        new_channel=(old_channel+1)%16
        rpc.call('set_control',key='midi0',value=new_channel)
        wait(lambda:rpc.call('status'),lambda s:any(c['key']=='midi0' and c['value']==new_channel for c in s['configuration']['controls']))
        rpc.call('pad_midi',pad=2,slot=0,note=52,velocity=0)
        rpc.call('set_control',key='midi0',value=old_channel)
        wait(lambda:rpc.call('status'),lambda s:any(c['key']=='midi0' and c['value']==old_channel for c in s['configuration']['controls']))
        rpc.call('pad_midi',pad=3,slot=0,note=55,velocity=90)
        before=rpc.call('status')['midiInBytes']
        rpc.close();rpc=None;time.sleep(.2)
        rpc=wait(connect,lambda r:r is not None)
        wait(lambda:rpc.call('status'),lambda s:s['padMidiHeld']==[] and s['midiInBytes']>=before+3)
        print('PASS real MIDI receipt, velocity bytes, duplicate suppression, note-off/channel retention, panic and client-loss cleanup',flush=True)
        rpc.close();rpc=None;time.sleep(.2)
        panel=Session(['--mock','--g1-address',f'127.0.0.1:{port}','--patch-dir',str(ROOT/'patches')])
        panel.wait_for(lambda:panel.state().get('engine')=='G1-EMU / LIVE')
        assert panel.state()['padMode']=='drums' and panel.state()['padBase']==36
        panel.command('pad 1 0.25');panel.wait_for(lambda:panel.state().get('heldPads')==[0])
        assert panel.state()['leds']['pad_1']==80
        panel.command('button keyboard');panel.wait_for(lambda:panel.state()['padMode']=='keyboard' and not panel.state()['heldPads'])
        assert panel.state()['padBase']==48 and panel.state()['leds']['keyboard']==80
        panel.command('pad 13 1');panel.wait_for(lambda:panel.state()['heldPads']==[12])
        for _ in range(1000):panel.command('pad 13 0.8')
        panel.command('pad 13 0');panel.wait_for(lambda:not panel.state()['heldPads'])
        panel.command('pad 2 1');panel.wait_for(lambda:panel.state()['heldPads']==[1])
        panel.command('button stop');panel.wait_for(lambda:not panel.state()['heldPads'])
        panel.command('pad 3 1');panel.wait_for(lambda:panel.state()['heldPads']==[2])
        panel.command('disconnect');panel.wait_for(lambda:not panel.state()['heldPads'])
        panel.command('reconnect');panel.wait_for(lambda:panel.state()['connection']['hasControl'])
        # Exercise fast reversals alongside a held note, with final value still confirmed.
        panel.command('pad 4 1');panel.wait_for(lambda:panel.state()['heldPads']==[3])
        for _ in range(20):
            for _ in range(20):panel.command('turn 1 0.008')
            for _ in range(20):panel.command('turn 1 -0.008')
            time.sleep(.015)
        panel.command('pad 4 0');panel.wait_for(lambda:not panel.state()['heldPads'])
        time.sleep(.5)
        assert panel.process.poll() is None
        assert not any('G1 bridge connection lost' in l for l in panel.lines)
        panel.command('quit');assert panel.process.wait(timeout=10)==0
        print('PASS physical mode command mappings, confirmed pad LEDs, 1000 pressure updates without retrigger, STOP/disconnect release and 800 rapid encoder reversals',flush=True)
    finally:
        if panel:panel.close()
        if rpc:rpc.close()
        if host.poll() is None:host.send_signal(signal.SIGTERM);host.wait(timeout=10)
        log.close()
    text=(folder/'host.log').read_text(errors='replace')
    assert '90 30 40' in text and '80 30 00' in text, 'expected MIDI note/velocity bytes not seen'
    assert '90 24 40' in text and '90 3c 7f' in text, 'drum/keyboard mode notes not seen'
    assert f'{0x80|old_channel:02x} 34 00' in text, 'release used a changed MIDI channel'
    with wave.open(str(folder/'output.wav'),'rb') as wav:
        assert wav.getsampwidth()==3 and wav.getnchannels()==4
        rate=wav.getframerate()/4
        raw=wav.readframes(wav.getnframes())
    samples=[int.from_bytes(raw[i:i+3],'little',signed=True)/8388608 for i in range(0,len(raw),48)]
    size=int(rate/5);windows=[]
    for start in range(0,len(samples)-size,size):
        values=samples[start:start+size];mean=sum(values)/size
        centered=[v-mean for v in values];rms=math.sqrt(sum(v*v for v in centered)/size)
        crosses=sum(centered[i-1]<=0<centered[i] for i in range(1,size))
        windows.append({'time':start/rate,'rms':rms,'frequency':crosses*rate/size})
    assert max(w['rms'] for w in windows)>1e-5, 'no recorded MIDI-gated audio'
    peak=max(w['rms'] for w in windows)
    assert sum(w['rms']<peak*.01 for w in windows)>=3, 'note releases did not return to silence'
    active=sorted(w['frequency'] for w in windows if w['rms']>peak*.3 and w['frequency']>20)
    assert active and active[-1]/active[0]>1.7, 'chromatic note pitch change not recorded'
    (folder/'audio-summary.json').write_text(json.dumps(windows,indent=2))
    print('PASS recorded MIDI gate/silence and changing note pitch; evidence:',folder,flush=True)


if __name__=='__main__':main()
