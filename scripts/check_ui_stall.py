"""Prove control/MIDI socket progress during a deliberate stalled emulator UI thread."""
import json
import os
import signal
import socket
import subprocess
import time
from check_g1 import ROOT, Rpc, wait


def main():
    folder=ROOT/'g1-state'/('ui-stall-'+str(time.time_ns()));folder.mkdir()
    with socket.socket() as probe:probe.bind(('127.0.0.1',0));port=probe.getsockname()[1]
    path=folder/'host.log';log=path.open('w')
    host=subprocess.Popen([str(ROOT/'build/g1-emu/g1mk3check'),str(ROOT/'roms/NORD-MODULAR-RACK-VER-3.03.BIN'),str(folder/'flash.bin'),str(ROOT/'patches/MK3-MIDI.pch'),str(port),'30'],stdout=log,stderr=subprocess.STDOUT,
                          env={**os.environ,'G1_MODEL':'regular','G1_GAIN_DB':'-12','G1_MIDI_CLIENT':'MK3-UI-Stall-Test','G1_CHECK_UI_STALL_MS':'600'})
    rpc=None
    try:
        def connect():
            try:return Rpc(port)
            except OSError:return None
        rpc=wait(connect,lambda r:r is not None)
        status=wait(lambda:rpc.call('status'),lambda s:s.get('transportThread')=='dedicated' and s.get('upload',{}).get('ok') and s['knobs'][0]['assigned'],20)
        binding=status['knobs'][0]['binding']
        wait(lambda:path.read_text(),lambda text:'UI stall begin' in text and 'UI stall end' not in text,8)
        initial=rpc.call('status')['midiInBytes'];samples=[];during=0
        for n in range(12):
            start=time.monotonic()
            rpc.call('set_knobs',targets=[dict(index=0,binding=binding,value=64+n%2)])
            rpc.call('pad_midi',pad=0,slot=0,note=48+n%2,velocity=80)
            rpc.call('pad_midi',pad=0,slot=0,note=48+n%2,velocity=0)
            state=rpc.call('status')
            samples.append((time.monotonic()-start)*1000)
            if 'UI stall end' not in path.read_text():during+=1
        assert during>=5, (during,samples)
        assert max(samples)<150, samples
        state=wait(lambda:rpc.call('status'),lambda s:s['midiInBytes']>=initial+72 and not s['padMidiHeld'])
        assert not state['problem'],state['problem']
        wait(lambda:path.read_text(),lambda text:'UI stall end' in text,3)
        result={'uiStallMs':600,'batchesCompletedDuringStall':during,'fourRpcBatchMs':samples,'midiBytesReceived':state['midiInBytes']-initial}
        (folder/'result.json').write_text(json.dumps(result,indent=2))
        print('PASS dedicated transport: %d knob/MIDI/status batches during 600ms UI stall, batch max %.1fms, 72 MIDI bytes consumed; no held notes' % (during,max(samples)))
        print('Evidence:',folder)
    finally:
        if rpc:rpc.close()
        if host.poll() is None:host.send_signal(signal.SIGTERM);host.wait(timeout=10)
        log.close()


if __name__=='__main__':main()
