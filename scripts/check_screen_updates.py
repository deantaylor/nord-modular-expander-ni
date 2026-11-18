"""Bounded physical MK3 capture/loop timing. Quit the existing controller first.
Only changes controller pages; does not load patches or edit the running synth.
"""
import argparse
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--address", default="127.0.0.1:47410")
    args = parser.parse_args()
    folder = ROOT/'g1-state'/('screen-timing-'+str(time.time_ns()))
    folder.mkdir(parents=True)
    path = folder/'controller.log'
    with path.open('w') as log:
        process = subprocess.Popen([str(ROOT/'target/debug/modular-machine-mk3'),'--g1-address', args.address], cwd=ROOT,
                                   stdin=subprocess.PIPE, stdout=log, stderr=subprocess.STDOUT,
                                   env={**os.environ,'MK3_SCREEN_TIMING':'1','RUST_LOG':'info,encdr::usb::device_thread=debug'})
        def wait(predicate, timeout=20):
            deadline=time.monotonic()+timeout
            while time.monotonic()<deadline:
                text=path.read_text(errors='replace')
                if predicate(text): return text
                if process.poll() is not None: raise RuntimeError(text[-4000:])
                time.sleep(.005)
            raise TimeoutError(path.read_text(errors='replace')[-4000:])
        try:
            wait(lambda text:'Screen timing:' in text and '"hasControl":true' in text)
            delays=[]
            sequence=['macro']*12+['settings','settings','mixer','mixer','browser']+['scroll 1']*8+['scroll -1']*8+['browser']
            for index,button in enumerate(sequence):
                sent=time.monotonic()
                process.stdin.write(((button if button.startswith('scroll ') else 'button '+button)+'\nreport\n').encode());process.stdin.flush()
                wait(lambda text:len(re.findall(r'^report: ',text,re.M))>=index+1,5)
                delays.append((time.monotonic()-sent)*1000)
                time.sleep(.10)
            text=wait(lambda text:len(re.findall('Screen timing:',text))>=5)
            samples=re.findall(r'frames: (\d+), last_ms: ([\d.]+), max_ms: ([\d.]+)',text)
            assert samples and all(int(frame)>=4 for frame,_,_ in samples[-2:]),samples
            assert all(float(last)<500 for _,last,_ in samples[-2:]),samples[-2:]
            assert max(delays)<500,delays
            assert 'Bulk out completed for screen' in text,text[-1000:]
            summary={'controller_loop_ms':delays,'capture_samples':samples,'service_completions':text.count('Bulk out completed for screen')}
            (folder/'timing.json').write_text(json.dumps(summary,indent=2)+'\n')
            print('PASS: physical MK3 screen captures and NI transfers; command-loop median %.1f ms, max %.1f ms; final capture samples %s' % (sorted(delays)[len(delays)//2],max(delays),samples[-2:]))
            print('Evidence:',folder)
        finally:
            if process.poll() is None:
                process.stdin.write(b'quit\n');process.stdin.flush()
                try:process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.send_signal(signal.SIGTERM)
                    try:process.wait(timeout=5)
                    except subprocess.TimeoutExpired:process.kill();process.wait()
            if process.returncode!=0:raise RuntimeError('Controller exit '+str(process.returncode))

if __name__=='__main__':main()
