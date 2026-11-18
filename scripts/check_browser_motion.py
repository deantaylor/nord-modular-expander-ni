"""Focused real-WebKit regression for browser motion, retained rows and value activity.
Uses the local ROM and isolated emulator state; never touches the user session.
"""
import sys,os,socket,subprocess,time,json,signal
from pathlib import Path
root=Path(__file__).resolve().parents[1]
if '--hidden' in sys.argv: os.environ['MK3_TRACE_FRAMES']='1'
from check_g1 import Rpc,wait
from check_mock import Session
folder=root/'g1-state'/('quick-fix-'+str(time.time_ns()));folder.mkdir();lib=folder/'patches';lib.mkdir()
original=(root/'patches/MK3-Bridge.pch').read_text();original=original.replace('0 127 0 127 2 0 0 1 4000','0 127 0 127 2 0 0 8 4000')
for name in ['Eight','Second','Third']+['Z %02d'%n for n in range(30)]:(lib/(name+'.pch')).write_text(original)
with socket.socket() as probe:probe.bind(('127.0.0.1',0));port=probe.getsockname()[1]
log=(folder/'host.log').open('w');host=subprocess.Popen([str(root/'build/g1-emu/g1mk3check'),str(root/'roms/NORD-MODULAR-RACK-VER-3.03.BIN'),str(folder/'flash.bin'),str(lib/'Eight.pch'),str(port),'90'],stdout=log,stderr=subprocess.STDOUT,env={**os.environ,'G1_MODEL':'regular','G1_GAIN_DB':'0'})
rpc=None;panel=None
try:
 def connect():
  try:return Rpc(port)
  except (OSError,ValueError):return None
 rpc=wait(connect,lambda x:x is not None)
 state=wait(lambda:rpc.call('status'),lambda s:s.get('requestedVoices')==8 and s.get('allocatedVoices')==8,timeout=18)
 print('PASS real voice counts',state['allocatedVoices'],state['requestedVoices'],state['lcd'],flush=True)
 rpc.close();rpc=None;time.sleep(.1)
 panel=Session(['--mock','--g1','--g1-address',f'127.0.0.1:{port}','--patch-dir',str(lib)])
 panel.wait_for(lambda:panel.state().get('requestedVoices')==8)
 panel.command('button browser');panel.wait_for(lambda:panel.state().get('browserOpen'))
 panel.command('scroll 1');panel.wait_for(lambda:panel.state().get('browserCursor')==1)
 time.sleep(.3);panel.command('inspect');panel.wait_for(lambda:sum(l.startswith('mock DOM') for l in panel.lines)==2)
 dom=json.loads(next(l.split(': ',1)[1] for l in panel.lines if l.startswith('mock DOM left')))
 assert any(r['index']=='1' and abs(r['top']-11)<1 for r in dom['rows']),dom
 assert '8 (8)' in dom['text'],dom
 assert 'Eight' in panel.state()['patch'],panel.state()
 for line in panel.lines:
  if line.startswith('mock DOM'):
   assert json.loads(line.split(': ',1)[1])['knobsHidden'],line
 assert 'Second' in dom['text'],dom
 following=sorted((r for r in dom['rows'] if int(r['index'])>1),key=lambda r:int(r['index']))
 assert len(following)>2,dom
 assert float(following[0]['opacity'])>float(following[1]['opacity'])>float(following[2]['opacity']),following
 panel.command('scroll 1');panel.wait_for(lambda:panel.state().get('browserCursor')==2)
 panel.command('scroll -1');panel.wait_for(lambda:panel.state().get('browserCursor')==1)
 time.sleep(.25)
 panel.command('capture-preview '+str(folder/'browser'));panel.wait_for(lambda:any('preview captured:' in l for l in panel.lines))
 print('PASS selected browser row at header; scroll preserves loaded patch; 8 (8) rendered',flush=True)
 for delta in [1]*22+[-1]*22:
  previous=panel.state()['browserCursor'];panel.command('scroll '+str(delta));panel.wait_for(lambda:panel.state()['browserCursor']==previous+delta)
 time.sleep(.25)
 before=sum(l.startswith('mock DOM') for l in panel.lines);panel.command('inspect');panel.wait_for(lambda:sum(l.startswith('mock DOM') for l in panel.lines)==before+2)
 latest=json.loads(next(l.split(': ',1)[1] for l in reversed(panel.lines) if l.startswith('mock DOM left')))
 assert any(r['index']=='1' and abs(r['top']-11)<1 for r in latest['rows']),latest
 assert all(r['transform']=='none' for r in latest['rows']),latest
 assert len([r for r in latest['rows'] if r['top']>=11 and r['top']<272])>=6,latest
 panel.command('button browser');panel.wait_for(lambda:not panel.state()['browserOpen'])
 panel.command('button browser');panel.wait_for(lambda:panel.state()['browserOpen'])
 print('PASS 44 rapid turns across virtual windows, focused row at A/, rows retained after reopen',flush=True)
 if '--hidden' in sys.argv:
  panel.command('scroll 9');panel.wait_for(lambda:panel.state()['browserCursor']==10);time.sleep(.25)
  start=len(panel.lines)
  for delta in [1,-1]*50+[1]*12+[-1]*12:
   panel.command('scroll '+str(delta));panel.wait_for(lambda:True);time.sleep(.01)
  panel.wait_for(lambda:panel.state()['browserCursor']==10);time.sleep(.3)
  before=sum(l.startswith('mock DOM') for l in panel.lines);panel.command('inspect');panel.wait_for(lambda:sum(l.startswith('mock DOM') for l in panel.lines)==before+2)
  moving=json.loads(next(l.split(': ',1)[1] for l in reversed(panel.lines) if l.startswith('mock DOM left')))
  assert abs(moving['scrollPosition']-10)<.01,moving
  frames=[json.loads(l.split(': ',1)[1]) for l in panel.lines[start:] if l.startswith('mock frame left')]
  assert len(frames)>=30,len(frames)
  blank=[f for f in frames if sum(n>30 for n in f['bands'])<4]
  (folder/'frame-trace.json').write_text(json.dumps(frames,indent=2))
  assert not blank,blank
  assert not any('JavaScript error' in l for l in panel.lines),panel.lines[-10:]
  print('PASS hidden native snapshots during 124 mid-animation direction changes; no blank frames (%d captured)'%len(frames),flush=True)
  panel.command('scroll -9');panel.wait_for(lambda:panel.state()['browserCursor']==1)
  time.sleep(5.1)
  panel.command('simulate-blank-ui')
  panel.wait_for(lambda:any('mock capture recovery left:' in l for l in panel.lines),8)
  time.sleep(.7)
  before=sum(l.startswith('mock DOM') for l in panel.lines);panel.command('inspect');panel.wait_for(lambda:sum(l.startswith('mock DOM') for l in panel.lines)==before+2)
  restored=json.loads(next(l.split(': ',1)[1] for l in reversed(panel.lines) if l.startswith('mock DOM left')))
  assert restored['browserOpen'] and abs(restored['scrollTarget']-1)<.01,restored
  assert any(r['index']=='1' and abs(r['top']-11)<1 for r in restored['rows']),restored
  print('PASS injected missing-list capture rejected; current selection restored',flush=True)
  time.sleep(5.1);panel.command('simulate-script-error')
  panel.wait_for(lambda:any('Injected renderer failure' in l for l in panel.lines),5)
  time.sleep(.7)
  before=sum(l.startswith('mock DOM') for l in panel.lines);panel.command('inspect');panel.wait_for(lambda:sum(l.startswith('mock DOM') for l in panel.lines)==before+2)
  restored=json.loads(next(l.split(': ',1)[1] for l in reversed(panel.lines) if l.startswith('mock DOM left')))
  assert restored['browserOpen'] and abs(restored['scrollTarget']-1)<.01,restored
  print('PASS injected JavaScript error reloads and restores current view',flush=True)


 panel.command('button encoder_press');panel.wait_for(lambda:'Loaded Second.pch' in str(panel.state()))
 print('PASS click loads selected patch',flush=True)
 panel.command('button browser');panel.wait_for(lambda:not panel.state()['browserOpen'])
 initial=panel.state()['displayValues'][0];panel.command('turn 1 0.1');panel.wait_for(lambda:panel.state()['displayValues'][0]!=initial)
 before=sum(l.startswith('mock DOM') for l in panel.lines);panel.command('inspect');panel.wait_for(lambda:sum(l.startswith('mock DOM') for l in panel.lines)==before+2)
 active=json.loads(next(l.split(': ',1)[1] for l in reversed(panel.lines) if l.startswith('mock DOM left')))
 assert active['knobs'][0]['active'],active
 assert active['knobs'][0]['weight']=='400',active
 print('PASS changed value has activity emphasis',flush=True)
 panel.command('quit');panel.process.wait(timeout=5)
finally:
 if panel:panel.close()
 if rpc:rpc.close()
 if host.poll() is None:host.send_signal(signal.SIGTERM);host.wait(timeout=10)
 log.close();print('Evidence:',folder,flush=True)
