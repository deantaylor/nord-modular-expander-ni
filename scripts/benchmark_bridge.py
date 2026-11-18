"""Isolated state/hidden-WebKit benchmark; never connects to the live synth."""
import json,sys
from pathlib import Path
import check_mock
from check_module_visuals import Fixture,parameter
ROOT=Path(__file__).resolve().parents[1]
def main():
 f=Fixture();p=None
 f.modules=[('EventSeq',[parameter(i,15 if i==0 else 0,127 if i==0 else 1) for i in range(36)])]
 try:
  check_mock.BINARY=ROOT/'target/release/nord-modular-ni-interface'
  p=check_mock.Session(['--mock','--g1','--g1-address',f'127.0.0.1:{f.port}'])
  p.wait_for(lambda:p.state().get('leds',{}).get('pattern')==30)
  p.command('button pattern');p.wait_for(lambda:p.state().get('sequence',{}).get('ready'))
  for _ in range(3):
   for command,prefix in [('bench-state','bench LEDs:'),('bench-sequence','bench sequence:')]:
    count=sum(l.startswith(prefix) for l in p.lines)
    p.command(command);p.wait_for(lambda:sum(l.startswith(prefix) for l in p.lines)>count)
    result=next(l for l in reversed(p.lines) if l.startswith(prefix));print(result,flush=True)
    if command=='bench-sequence' and '--expect-retained' in sys.argv:
     assert json.loads(result.split(': ',1)[1])['replaced']==0,result
 finally:
  if p:p.close()
  f.close()
if __name__=='__main__':main()
