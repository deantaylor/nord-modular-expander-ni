"""Measure native RPC readback separately from controller-command-to-confirmed-state latency.
Neither measurement claims physical input-to-panel scanout latency.
"""
import json
import math
import os
import signal
import socket
import statistics
import subprocess
import time
from check_g1 import ROOT,Rpc,wait
from check_mock import Session


def main():
    folder=ROOT/'g1-state'/('control-latency-'+str(time.time_ns()));folder.mkdir()
    with socket.socket() as probe:probe.bind(('127.0.0.1',0));port=probe.getsockname()[1]
    log=(folder/'host.log').open('w');host=subprocess.Popen([str(ROOT/'build/g1-emu/g1mk3check'),str(ROOT/'roms/NORD-MODULAR-RACK-VER-3.03.BIN'),str(folder/'flash.bin'),str(ROOT/'patches/MK3-Bridge.pch'),str(port),'40'],stdout=log,stderr=subprocess.STDOUT,env={**os.environ,'G1_MODEL':'regular','G1_GAIN_DB':'-12','G1_MIDI_CLIENT':'MK3-Latency-Test'})
    rpc=None;panel=None
    try:
        def connect():
            try:return Rpc(port)
            except OSError:return None
        rpc=wait(connect,lambda r:r is not None)
        status=wait(lambda:rpc.call('status'),lambda s:s.get('transportThread')=='dedicated' and s.get('upload',{}).get('ok') and s['knobs'][0]['assigned'],20)
        binding=status['knobs'][0]['binding'];direct=[]
        for target in [65,64]*8:
            start=time.monotonic();rpc.call('set_knobs',targets=[dict(index=0,value=target,binding=binding)])
            while rpc.call('status')['knobs'][0]['value']!=target:
                assert time.monotonic()-start<1
                time.sleep(.001)
            direct.append((time.monotonic()-start)*1000)
        rpc.close();rpc=None;time.sleep(.1)
        panel=Session(['--mock','--g1-address',f'127.0.0.1:{port}','--patch-dir',str(ROOT/'patches')])
        panel.wait_for(lambda:panel.state().get('engine')=='G1-EMU / LIVE')
        value=round(panel.state()['values'][0]*127);remainder=0.;controller=[]
        for delta in [0.04,-0.04]*8:
            movement=remainder+delta*.5*127;steps=math.trunc(movement);remainder=movement-steps;value+=steps
            start=time.monotonic();panel.command(f'turn 1 {delta}')
            panel.wait_for(lambda:round(panel.state()['values'][0]*127)==value)
            controller.append((time.monotonic()-start)*1000)
        assert max(controller)<250,controller
        panel.command('quit');assert panel.process.wait(timeout=10)==0
        result={'nativeRpcReadbackMs':direct,'controllerCommandToConfirmedStateMs':controller,'physicalScanoutMeasured':False}
        (folder/'timing.json').write_text(json.dumps(result,indent=2))
        print('Native set/readback median %.1fms max %.1fms; controller command/confirmed state median %.1fms max %.1fms' % (statistics.median(direct),max(direct),statistics.median(controller),max(controller)))
        print('Evidence:',folder)
    finally:
        if panel:panel.close()
        if rpc:rpc.close()
        if host.poll() is None:host.send_signal(signal.SIGTERM);host.wait(timeout=10)
        log.close()


if __name__=='__main__':main()
