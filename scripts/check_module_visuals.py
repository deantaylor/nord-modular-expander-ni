"""Hidden-WebKit module diagram QA using explicit fixtures, no synth or hardware access."""
import copy,json,math,socket,threading,time
from pathlib import Path
from service_paths import BRIDGE_SERVICE
from PIL import Image
import check_mock
ROOT=Path(__file__).resolve().parents[1]

def parameter(i,value=64,maximum=127,name=None,display=None):
    return dict(index=i,binding=f'visual-fixture:param:{i}',name=name or f'Parameter {i}',value=value,min=0,max=maximum,display=str(value) if display is None else display)

class Fixture:
    def __init__(self):
        self.lock=threading.RLock();self.selected=None;self.hold=False;self.held=[];self.extra_controls=[]
        adsr=[parameter(0,0,2,'Attack shape','Log'),parameter(1,40,name='Attack',display='12 ms'),parameter(2,70,name='Decay',display='450 ms'),parameter(3,90,name='Sustain'),parameter(4,55,name='Release',display='220 ms'),parameter(5,0,1,'Invert','Off')]
        filt=[parameter(i,0) for i in range(10)]
        for i,v,m,n,d in [(0,0,3,'Mode','LP'),(1,1,1,'Gain','On'),(3,72,127,'Cutoff','660 Hz'),(4,64,127,'Key track','50%'),(6,90,127,'Resonance','90'),(7,1,1,'Slope','24 dB'),(9,0,1,'Bypass','Off')]:filt[i]=parameter(i,v,m,n,d)
        mod=[parameter(0,40,127,'Attack','12 ms'),parameter(1,70,127,'Decay','450 ms'),parameter(2,90,127,'Sustain'),parameter(3,55,127,'Release','220 ms')]+[parameter(i,0) for i in range(4,8)]+[parameter(8,0,1,'Invert','Off')]
        classic=[parameter(i,0) for i in range(7)];classic[0]=parameter(0,80,127,'Cutoff','1.05 kHz');classic[2]=parameter(2,95,127,'Resonance');classic[5]=parameter(5,2,2,'Slope','24 dB');classic[6]=parameter(6,0,1,'Bypass','Off')
        multi=[parameter(0,64,127,'Cutoff','416 Hz'),parameter(1,90,127,'Resonance'),parameter(2,1,1,'Gain','On')]
        self.modules=[('ADSR',adsr),('FilterE',filt),('Mod-Env',mod),('FilterF',classic),('FilterC',multi),('OscA',[parameter(0),parameter(1),parameter(2),parameter(3,64,127,'Pulse Width','50%'),parameter(4,0,3,'Waveform','Sine')]),('Unsupported',[parameter(0)])]
        self.listener=socket.socket();self.listener.bind(('127.0.0.1',0));self.port=self.listener.getsockname()[1];self.listener.listen(1);self.listener.settimeout(.2);self.stop=threading.Event();self.client=None
        self.thread=threading.Thread(target=self.run,daemon=True);self.thread.start()
    def status(self):
        edit=None
        if self.selected is not None:
            kind,params=self.modules[self.selected];edit=dict(section=1,index=self.selected+1,name=kind,typeName=kind,parameters=copy.deepcopy(params))
        return dict(running=True,activeSlot=0,patchBinding='visual-fixture',lcd=['Visual fixture',''],allocatedVoices=6,requestedVoices=8,edit=edit,knobs=[dict(index=i,assigned=False,slot=0,binding=f'assigned:{i}',module='',parameter='',value=0,max=127,display='Unassigned') for i in range(18)],configuration={'controls':copy.deepcopy(self.extra_controls)},padMidi=False)
    def run(self):
        try:
            while not self.stop.is_set():
                try:self.client,_=self.listener.accept();break
                except socket.timeout:continue
            if self.client is None:return
            file=self.client.makefile('rwb');file.write((json.dumps(dict(service=BRIDGE_SERVICE, protocol=1, instance='visual-fixture')) + '\n').encode());file.flush()
            while not self.stop.is_set():
                line=file.readline()
                if not line:break
                r=json.loads(line);method=r['method'];result={'accepted':True}
                with self.lock:
                    if method=='status':result=self.status()
                    elif method=='modules':result=dict(patchBinding='visual-fixture',entries=[dict(section=1,index=i+1,name=k,typeName=k,parameterCount=len(p)) for i,(k,p) in enumerate(self.modules)])
                    elif method=='module_select':self.selected=r['index']-1
                    elif method=='module_exit':self.selected=None
                    elif method=='set_control':
                        for c in self.extra_controls:
                            if c['key']==r['key']:
                                c['value']=r['value'];c['display']=str(r['value']);result['previews']=[dict(binding=c['key'],value=c['value'],display=c['display'])]
                    elif method=='set_parameters':
                        result['previews']=[]
                        for target in r['targets']:
                            p=self.modules[target['module']-1][1][target['index']]
                            result['previews'].append(dict(binding=p['binding'],value=target['value'],display=str(target['value'])))
                            if self.hold:self.held.append((p,target['value']))
                            else:p['value']=target['value'];p['display']=str(target['value'])
                file.write((json.dumps(dict(id=r['id'],result=result))+'\n').encode());file.flush()
        except (OSError,ValueError):pass
    def close(self):
        self.stop.set()
        if self.client:
            try:self.client.shutdown(socket.SHUT_RDWR)
            except OSError:pass
            self.client.close()
        self.listener.close();self.thread.join(timeout=2)

def main():
    folder=ROOT/'g1-state'/f'module-visuals-{time.time_ns()}';folder.mkdir(parents=True)
    check_mock.BINARY=ROOT/'target/release/nord-modular-ni-interface';fixture=Fixture();panel=None
    try:
        panel=check_mock.Session(['--mock','--g1','--g1-address',f'127.0.0.1:{fixture.port}'])
        panel.wait_for(lambda:panel.state().get('engine')=='G1-EMU / LIVE');panel.command('button module_edit');panel.wait_for(lambda:panel.state().get('moduleReady'))
        def inspect():
            count=sum(x.startswith('mock DOM') for x in panel.lines);panel.command('inspect');panel.wait_for(lambda:sum(x.startswith('mock DOM') for x in panel.lines)>=count+2)
            return {side:json.loads(next(x.split(': ',1)[1] for x in reversed(panel.lines) if x.startswith('mock DOM '+side+':'))) for side in ['left','right']}
        def capture(name):
            target=folder/name;panel.command('capture-preview '+str(target));panel.wait_for(lambda:any('preview captured: '+str(target) in x for x in panel.lines))
            for side in ['left','right']:Image.frombytes('RGBA',(480,272),(target/(side+'.rgba')).read_bytes()).save(target/(side+'.png'))
        for index,kind in enumerate(['ADSR','FilterE','Mod-Env','FilterF','FilterC','OscA']):
            if index:panel.command('button top_2');panel.wait_for(lambda:panel.state().get('moduleCursor')==index and panel.state().get('moduleReady'))
            dom=inspect();right=dom['right'];assert right.get('moduleVisual') and not dom['left'].get('moduleVisual'),dom
            assert right['visualBox']['bottom']<min(k['top'] for k in right['knobs']),right
            assert all(k['bottom']<=272 for k in right['knobs']),right
            assert right['visualBox']['left']==8 and right['visualBox']['right']==472 and right['visualBox']['height']==132,right
            if kind=='FilterE':assert dom['left']['knobs'][0]['display']=='LP' and dom['right']['knobs'][3]['display']=='24 dB',dom
            if kind=='FilterF':assert dom['right']['knobs'][1]['display']=='24 dB',dom
            capture(kind);print('PASS right-hand diagram/layout',kind,flush=True)
            assert all(not k['numberVisible'] and not k['moduleVisible'] for side in dom.values() for k in side['knobs']),dom
            if kind=='OscA':
                before=right['visualPaths'];panel.command('turn 5 0.1');panel.wait_for(lambda:panel.state()['moduleParameters'][4]['value']==1);assert inspect()['right']['visualPaths']!=before
            if kind=='ADSR':
                before=right['visualPaths']
                with fixture.lock:fixture.hold=True
                panel.command('turn 2 0.3');panel.wait_for(lambda:panel.state()['pendingValues'][1] and panel.state()['displayValues'][1]!='12 ms')
                pending=panel.state();assert pending['moduleParameters'][1]['confirmedValue']==40
                active=inspect();assert 'PENDING' in active['right']['moduleVisual'];assert active['right']['visualPaths']!=before
                with fixture.lock:
                    for p,value in fixture.held:p['value']=value;p['display']=str(value)
                    fixture.held.clear();fixture.hold=False
                panel.wait_for(lambda:not panel.state()['pendingValues'][1]);active=inspect();assert active['right']['visualPaths']!=before;assert active['left']['knobs'][1]['active'];time.sleep(.35);assert not inspect()['left']['knobs'][1]['active'],'Activity must clear after its one-shot timer'
            if kind=='FilterE':
                panel.command('button knob_page_next');panel.wait_for(lambda:panel.state().get('page')==1);before=inspect()['right']['visualPaths']
                with fixture.lock:fixture.modules[1][1][3]['value']=24;fixture.modules[1][1][3]['display']='41 Hz'
                panel.wait_for(lambda:panel.state()['moduleParameters'][3]['value']==24);assert inspect()['right']['visualPaths']!=before,'Off-page cutoff must change diagram'
                panel.command('turn 2 1');panel.wait_for(lambda:panel.state()['moduleParameters'][9]['value']==1);assert 'BYPASS' in inspect()['right']['moduleVisual']
            if kind=='Mod-Env':
                panel.command('button knob_page_next');panel.wait_for(lambda:panel.state().get('page')==1);before=inspect()['right']['visualPaths'];panel.command('turn 1 1');panel.wait_for(lambda:panel.state()['moduleParameters'][8]['value']==1);assert inspect()['right']['visualPaths']!=before
        panel.command('button top_2');panel.wait_for(lambda:panel.state().get('moduleCursor')==6 and panel.state().get('moduleReady'));assert not inspect()['right'].get('moduleVisual'),'Unsupported module must clear old diagram'
        panel.command('button module_edit');panel.wait_for(lambda:not panel.state().get('moduleEdit'));assert not inspect()['right'].get('moduleVisual')
        panel.command('quit');panel.process.wait(timeout=8);print('PASS edits, off-page cutoff/invert, bypass, unsupported/exit cleanup. Evidence:',folder)
    finally:
        if panel:panel.close()
        fixture.close()

if __name__=='__main__':main()
