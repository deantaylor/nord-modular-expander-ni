"""Hidden display fixture: idle breathing, genuine note-event payload and page cleanup."""
import copy,json,time,os
from pathlib import Path
from PIL import Image
import check_mock
from check_module_visuals import Fixture
ROOT=Path(__file__).resolve().parents[1]
def main():
 f=Fixture();base=f.status;activity={'connected':True,'source':'explicit test fixture','serial':0,'velocity':0,'timestampMs':0,'activeNotes':0,'heldVelocity':0,'heldPeakNotes':0,'releaseTimestampMs':0,'releaseVelocity':0,'releaseNotes':0};p=None
 folder=ROOT/'g1-state'/f'midi-bloom-{time.time_ns()}'
 def status():
  s=base();s['midiActivity']=copy.deepcopy(activity);return s
 f.status=status;check_mock.BINARY=ROOT/'target/release/nord-modular-ni-interface'
 try:
  p=check_mock.Session(['--mock','--g1','--g1-address',f'127.0.0.1:{f.port}']);p.wait_for(lambda:p.state().get('engine')=='G1-EMU / LIVE')
  def inspect():
   count=sum(x.startswith('mock DOM') for x in p.lines);p.command('inspect');p.wait_for(lambda:sum(x.startswith('mock DOM') for x in p.lines)>=count+2)
   return {side:json.loads(next(x.split(': ',1)[1] for x in reversed(p.lines) if x.startswith('mock DOM '+side+':'))) for side in ['left','right']}
  def capture(name):
   target=folder/name;p.command('capture-preview '+str(target));p.wait_for(lambda:any('preview captured: '+str(target) in x for x in p.lines))
   for side in ['left','right']:Image.frombytes('RGBA',(480,272),(target/(side+'.rgba')).read_bytes()).save(target/(side+'.png'))
  if os.environ.get('BLOOM_TEST_DARK')=='1':
   p.command('hold track');p.wait_for(lambda:p.state().get('trackHeld'));p.command('button top_8');p.wait_for(lambda:p.state().get('view')=='display');p.command('release track');p.command('turn 1 0.25');p.wait_for(lambda:p.state().get('uiTheme')=='dark');p.command('hold track');p.wait_for(lambda:p.state().get('view')=='parameters');p.command('release track');p.wait_for(lambda:not p.state().get('trackHeld'))
  dom=inspect();assert dom['left']['bloomVisible'] and dom['right']['bloomVisible'];first=dom['right']['bloomFrame'];capture('idle')
  p.wait_for(lambda:inspect()['right']['bloomFrame']>first+2)
  with f.lock:activity.update(serial=1,velocity=127,timestampMs=int(time.time()*1000),activeNotes=2,heldVelocity=127,heldPeakNotes=2)
  p.wait_for(lambda:p.state().get('midiActivity',{}).get('serial')==1)
  p.wait_for(lambda:inspect()['right']['bloomEnergy']>.2);capture('note')
  if os.environ.get('BLOOM_TEST_DARK')=='1':
   coloured=0
   for side in ['left','right']:
    im=Image.open(folder/'note'/(side+'.png')).convert('RGB').crop((0,42,480,184))
    coloured+=sum(max(pixel)-min(pixel)>=60 and max(pixel)>80 for pixel in im.getdata())
   assert coloured>=200,('RGB separation must be visible in captured pixels',coloured)
   print('PASS visible RGB pixels:',coloured)
  time.sleep(1.3);assert all(d['bloomEnergy']>.95 for d in inspect().values()),'Held notes must not decay'
  with f.lock:activity.update(activeNotes=0,heldVelocity=0,heldPeakNotes=0,releaseTimestampMs=int(time.time()*1000),releaseVelocity=127,releaseNotes=2)
  p.wait_for(lambda:p.state().get('midiActivity',{}).get('activeNotes')==0)
  p.wait_for(lambda:all(d['bloomEnergy']<.5 for d in inspect().values()))
  p.command('button module_edit');p.wait_for(lambda:p.state().get('moduleReady'));dom=inspect();assert not dom['right']['bloomVisible'];stopped=dom['right']['bloomFrame'];time.sleep(.2);assert inspect()['right']['bloomFrame']==stopped
  p.command('button browser');p.wait_for(lambda:p.state().get('browserOpen'));assert not inspect()['right']['bloomVisible']
  p.command('button browser');p.wait_for(lambda:not p.state().get('browserOpen'));assert inspect()['right']['bloomVisible']
  p.command('button mixer');p.wait_for(lambda:p.state().get('view')=='mixer');assert not inspect()['right']['bloomVisible']
  p.command('hold track');p.wait_for(lambda:p.state().get('view')=='parameters');p.command('button top_8');p.wait_for(lambda:p.state().get('view')=='display');p.command('release track')
  assert p.state()['blobsEnabled'] and p.state()['glitchIntensity']==.5
  assert all(d['bloomVisible'] for d in inspect().values()),'Display has live preview'
  p.command('turn 2 -0.01');p.command('turn 3 -1');p.wait_for(lambda:not p.state()['blobsEnabled'] and not p.state()['glitchesEnabled'])
  assert p.state()['displayValues'][1:3]==['Off','0%']
  p.command('hold track');p.wait_for(lambda:p.state().get('view')=='parameters');p.command('release track')
  dom=inspect();assert all(not d['bloomVisible'] for d in dom.values());frames={side:d['bloomFrame'] for side,d in dom.items()};time.sleep(.2);assert all(d['bloomFrame']==frames[side] for side,d in inspect().items())
  p.command('hold track');p.wait_for(lambda:p.state().get('trackHeld'));p.command('button top_8');p.wait_for(lambda:p.state().get('view')=='display');p.command('release track');p.command('turn 2 0.01');p.wait_for(lambda:p.state()['blobsEnabled']);assert not p.state()['glitchesEnabled']
  p.command('hold track');p.wait_for(lambda:p.state().get('view')=='parameters');p.command('release track');assert all(d['bloomVisible'] for d in inspect().values())
  p.command('hold track');p.wait_for(lambda:p.state().get('trackHeld'));p.command('button top_8');p.wait_for(lambda:p.state().get('view')=='display');p.command('release track')
  p.command('turn 3 0.005');p.wait_for(lambda:p.state()['displayValues'][2]=='1%')
  p.command('turn 3 1');p.wait_for(lambda:p.state()['glitchIntensity']==1)
  p.wait_for(lambda:any(d['effectiveTheme']!='dark' for d in inspect().values()),timeout=16)
  assert p.state()['uiTheme']=='dark','Temporary glitches must not change saved theme'
  p.wait_for(lambda:all(d['effectiveTheme']=='dark' for d in inspect().values()),timeout=5)
  p.wait_for(lambda:all(d['bloomAlertCount']>=1 and d['bloomAlertSequence']=='1,2,3,0' for d in inspect().values()),timeout=16)
  p.command('turn 3 -0.05');p.wait_for(lambda:p.state()['displayValues'][2]=='90%');p.wait_for(lambda:all(d['effectiveTheme']=='dark' for d in inspect().values()))
  with f.lock:activity.update(activeNotes=0,heldVelocity=0,heldPeakNotes=0,releaseTimestampMs=0)
  p.wait_for(lambda:p.state().get('midiActivity',{}).get('activeNotes')==0)
  p.command('turn 2 -0.01');p.wait_for(lambda:not p.state()['blobsEnabled'])
  p.command('button mixer');p.wait_for(lambda:p.state()['view']=='mixer')
  p.command('idle-for 300');p.wait_for(lambda:p.state().get('screensaver'))
  p.wait_for(lambda:all(float(d['saverUiOpacity'])==0 and d['bloomVisible'] for d in inspect().values()))
  capture('screensaver')
  p.command('idle-for 3900');p.wait_for(lambda:all(d['effectiveGlitch']==1 for d in inspect().values()))
  p.command('hold track');p.wait_for(lambda:not p.state().get('screensaver'));p.command('release track')
  p.wait_for(lambda:all(float(d['saverUiOpacity'])==1 for d in inspect().values()))
  assert p.state()['glitchIntensity']==.9 and not p.state()['blobsEnabled']
  p.command('idle-for 300');p.wait_for(lambda:p.state().get('screensaver'))
  with f.lock:activity.update(activityTimestampMs=int(time.time()*1000))
  p.wait_for(lambda:not p.state().get('screensaver'))
  print('PASS idle saver hides UI, forces blobs, reaches full intensity, surface and MIDI wake restore saved settings')
  print('PASS temporary theme jumps/restoration; live Display preview, small encoder increment, and clear/glitch/clear ALERT sequence on both screens')
  print('PASS idle frames, MIDI note/velocity response, both Home screens, timers stopped in module/browser/mixer; evidence:',folder)
 finally:
  if p:p.close()
  f.close()
if __name__=='__main__':main()
