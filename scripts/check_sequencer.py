"""Isolated hidden-WebKit sequencer navigation/editing fixture; no live patches."""
import json,time
from pathlib import Path
from PIL import Image
import check_mock
from check_module_visuals import Fixture,parameter
ROOT=Path(__file__).resolve().parents[1]
def main():
 f=Fixture();base=f.status;playhead={'value':None}
 def status():
  s=base()
  if s['edit'] is not None:s['edit']['playhead']=playhead['value']
  return s
 f.status=status;p=None;folder=ROOT/'g1-state'/f'sequencer-ui-{time.time_ns()}'
 def note(kind,value):return (kind,[parameter(i,value if i<16 else 15 if i==16 else 0,127 if i<=16 else 16 if i==17 else 1) for i in range(21)])
 f.modules=[('OscA',[parameter(0)]),note('NoteSeqA',64),note('NoteSeqB',60),('CtrlSeq',[parameter(i,64 if i<16 else 15 if i==16 else 0,127 if i<=16 else 1) for i in range(19)]),('EventSeq',[parameter(i,15 if i==0 else 0,127 if i==0 else 1) for i in range(36)])]
 try:
  check_mock.BINARY=ROOT/'target/release/nord-modular-ni-interface';p=check_mock.Session(['--mock','--g1','--g1-address',f'127.0.0.1:{f.port}']);p.wait_for(lambda:p.state().get('engine')=='G1-EMU / LIVE' and p.state().get('leds',{}).get('pattern')==30)
  def inspect():
   count=sum(l.startswith('mock DOM') for l in p.lines);p.command('inspect');p.wait_for(lambda:sum(l.startswith('mock DOM') for l in p.lines)>=count+2)
   return {side:json.loads(next(l.split(': ',1)[1] for l in reversed(p.lines) if l.startswith('mock DOM '+side+':'))) for side in ['left','right']}
  def capture(name):
   target=folder/name;p.command('capture-preview '+str(target));p.wait_for(lambda:any('preview captured: '+str(target) in l for l in p.lines))
   for side in ['left','right']:Image.frombytes('RGBA',(480,272),(target/(side+'.rgba')).read_bytes()).save(target/(side+'.png'))
  p.command('button pattern');p.wait_for(lambda:p.state().get('sequence',{}).get('ready'))
  assert p.state()['moduleType']=='NoteSeqA';assert len(p.state()['sequence']['list'])==4
  dom=inspect();assert dom['left']['pageTitle']=='Pattern' and len(dom['right']['sequenceCells'])==16;assert dom['right']['editAction']['text']=='Edit Sequence' and not dom['right']['editAction']['active'];assert dom['right']['editAction']['x']==8 and dom['right']['editAction']['y']==8;assert not dom['left']['supportingText'] and not dom['right']['supportingText']
  p.command('button encoder_right');assert not p.state()['sequence']['editing'] and not p.state()['sequence']['stepFocus'];assert p.state()['sequence']['step']==0
  p.command('button top_5');p.wait_for(lambda:p.state()['sequence']['editing']);p.command('wheel 1');p.wait_for(lambda:p.state()['moduleParameters'][0]['confirmedValue']==65)
  p.command('button encoder_right');p.command('wheel -500');p.wait_for(lambda:p.state()['moduleParameters'][1]['confirmedValue']==0);capture('interval')
  for kind in ['NoteSeqB','CtrlSeq','EventSeq']:
   p.command('button top_5');p.wait_for(lambda:not p.state()['sequence']['editing']);assert not p.state()['sequence']['stepFocus'];assert not any(c['selected'] for c in inspect()['right']['sequenceCells']);p.command('wheel 1');p.wait_for(lambda:p.state()['moduleType']==kind and p.state()['sequence']['ready']);p.command('button top_5');p.wait_for(lambda:p.state()['sequence']['editing'])
   if kind=='EventSeq':
    p.command('button encoder_down');p.command('button encoder_left');p.command('button browser_select');p.wait_for(lambda:p.state()['moduleParameters'][35]['confirmedValue']==1)
    p.command('wheel 1');p.wait_for(lambda:p.state()['sequence']['step']==0);assert p.state()['sequence']['rows'][1][15]['value']==1 and p.state()['sequence']['rows'][1][0]['value']==0
    p.command('button encoder_left');p.wait_for(lambda:p.state()['sequence']['step']==15)
   else:
    p.command('wheel 1');p.wait_for(lambda:p.state()['moduleParameters'][0]['confirmedValue']==(61 if kind=='NoteSeqB' else 65))
   with f.lock:playhead['value']=5
   p.wait_for(lambda:p.state()['sequence'].get('playhead')==5)
   p.wait_for(lambda:sum(dot['playhead'] for dot in inspect()['right']['sequenceDots'])==(2 if kind=='EventSeq' else 1))
   dots=inspect()['right']['sequenceDots'];assert len(dots)==(32 if kind=='EventSeq' else 16);assert all(dot['colour']=='rgb(239, 68, 68)' for dot in dots if dot['playhead'])
   dom=inspect();assert len(dom['right']['sequenceCells'])==(32 if kind=='EventSeq' else 16);assert sum(c['selected'] for c in dom['right']['sequenceCells'])==1;assert max(c['bottom'] for c in dom['right']['sequenceCells'])<=264
   capture(kind)
  p.command('button knob_page_next');p.wait_for(lambda:p.state()['view']=='parameters');assert not p.state()['sequence']['editing'];assert not inspect()['right']['sequenceVisible']
  p.command('button pattern');p.wait_for(lambda:p.state()['view']=='sequencer');p.command('button browser');p.wait_for(lambda:p.state()['browserOpen']);assert not inspect()['right']['sequenceVisible']
  print('PASS filtered sequencers; note/value rotation and bounds; independent wheel directions; two event rows/press toggle; mode/page cleanup; evidence:',folder)
 finally:
  if p:p.close()
  f.close()
if __name__=='__main__':main()
