"""Focused hidden-WebKit mixer layout/control fixture, without synth or hardware."""
import json,time
from pathlib import Path
from PIL import Image
import check_mock
from check_module_visuals import Fixture
ROOT=Path(__file__).resolve().parents[1]
def main():
    f=Fixture();panel=None;folder=ROOT/'g1-state'/f'mixer-ui-{time.time_ns()}';folder.mkdir(parents=True)
    f.extra_controls=[dict(key=f'slotLevel:{i}:fixture:1:1:3',label=label,value=100 if i<2 else 0,min=0,max=127 if i<2 else 0,display='100' if i<2 else '—') for i,label in enumerate(['Slot A · AnalogBass','Slot B · Drums','Slot C · No Output module','Slot D · Multiple Outputs'])]
    check_mock.BINARY=ROOT/'target/release/nord-modular-ni-interface'
    try:
        panel=check_mock.Session(['--mock','--g1','--g1-address',f'127.0.0.1:{f.port}']);panel.wait_for(lambda:panel.state().get('engine')=='G1-EMU / LIVE')
        def inspect():
            count=sum(x.startswith('mock DOM') for x in panel.lines);panel.command('inspect');panel.wait_for(lambda:sum(x.startswith('mock DOM') for x in panel.lines)>=count+2)
            return {side:json.loads(next(x.split(': ',1)[1] for x in reversed(panel.lines) if x.startswith('mock DOM '+side+':'))) for side in ['left','right']}
        def capture(name):
            target=folder/name;panel.command('capture-preview '+str(target));panel.wait_for(lambda:any('preview captured: '+str(target) in x for x in panel.lines))
            for side in ['left','right']:Image.frombytes('RGBA',(480,272),(target/(side+'.rgba')).read_bytes()).save(target/(side+'.png'))
        panel.wait_for(lambda:'Visual fixture' in inspect()['left']['text'])
        headers=inspect();assert 'Visual fixture' in headers['left']['text'] and '6 (8)' in headers['left']['text'],headers;assert '1 / 3' in headers['right']['text'] and 'PAGE' not in headers['right']['text'];assert all('Nord Modular' not in d['text'] for d in headers.values())
        assert panel.state()['uiTheme']=='light'
        assert all(d['background']=='rgb(255, 255, 255)' for d in headers.values());capture('light-home')
        panel.command('hold track');panel.wait_for(lambda:panel.state().get('trackHeld'))
        assert panel.state()['leds']['top_7']==30
        panel.command('button top_7');panel.wait_for(lambda:panel.state().get('view')=='settings')
        assert not panel.state().get('moduleEdit') and not panel.state().get('browserOpen')
        panel.command('release track');panel.command('hold track');panel.wait_for(lambda:panel.state().get('view')=='parameters')
        panel.command('button top_8');panel.wait_for(lambda:panel.state().get('view')=='display')
        panel.command('release track');panel.command('turn 1 0.25');panel.wait_for(lambda:panel.state()['uiTheme']=='dark')
        assert all(d['background']=='rgb(20, 20, 20)' for d in inspect().values())
        panel.command('hold track');panel.wait_for(lambda:panel.state().get('view')=='parameters');capture('dark-home')
        panel.command('button top_8');panel.wait_for(lambda:panel.state().get('view')=='display')
        panel.command('release track');panel.command('turn 1 0.25');panel.wait_for(lambda:panel.state()['uiTheme']=='colour')
        panel.command('hold track');panel.wait_for(lambda:panel.state().get('view')=='parameters')
        panel.command('release track');panel.wait_for(lambda:not panel.state().get('trackHeld'));capture('colour-home')
        original_status=f.status
        for slot,colour in enumerate(['rgb(255, 128, 130)','rgb(235, 255, 219)','rgb(255, 202, 135)','rgb(255, 243, 155)']):
            def themed_status(slot=slot):
                state=original_status();state['activeSlot']=slot;return state
            f.status=themed_status
            panel.wait_for(lambda:panel.state().get('slot')=='ABCD'[slot])
            panel.wait_for(lambda:all(d['background']==colour for d in inspect().values()))
        f.status=original_status;panel.wait_for(lambda:panel.state().get('slot')=='A')
        panel.command('hold track');panel.wait_for(lambda:panel.state().get('trackHeld'))
        assert panel.state()['leds']['top_8']==30
        panel.command('button top_8');panel.wait_for(lambda:panel.state().get('view')=='display')
        panel.command('release track');panel.wait_for(lambda:not panel.state().get('trackHeld'))
        assert panel.state()['labels'][1]=='Colour intensity'
        panel.command('turn 4 0.4');panel.wait_for(lambda:panel.state().get('displayValues',[None]*4)[3]=='40%')
        dim=inspect();assert all(d['background']=='rgb(255, 77, 80)' for d in dim.values()),dim
        assert 'Display' in dim['left']['text'] and 'Colour intensity' in dim['left']['text']
        target=folder/'display';panel.command('capture-preview '+str(target));panel.wait_for(lambda:any('preview captured: '+str(target) in x for x in panel.lines))
        for side in ['left','right']:Image.frombytes('RGBA',(480,272),(target/(side+'.rgba')).read_bytes()).save(target/(side+'.png'))
        assert all(k['bottom']<=272 for d in dim.values() for k in d['knobs'])
        panel.command('turn 4 0.6');panel.wait_for(lambda:panel.state()['displayValues'][3]=='100%')
        saturated=inspect();assert all(d['background']=='rgb(255, 0, 4)' for d in saturated.values())
        panel.command('hold track');panel.wait_for(lambda:panel.state().get('view')=='parameters')
        assert panel.state()['uiColourIntensity']==1
        home=inspect();assert 'Colour intensity' not in home['left']['text'] and 'Visual fixture' in home['left']['text']
        panel.command('button top_8');panel.wait_for(lambda:panel.state().get('view')=='display')
        panel.command('release track');panel.command('turn 4 -1');panel.wait_for(lambda:panel.state()['displayValues'][3]=='0%')
        panel.command('hold track');panel.wait_for(lambda:panel.state().get('view')=='parameters')
        panel.command('release track');panel.wait_for(lambda:not panel.state().get('trackHeld'))
        panel.command('button module_edit');panel.wait_for(lambda:panel.state().get('moduleEdit') and panel.state().get('moduleReady'))
        panel.command('button browser');panel.wait_for(lambda:panel.state().get('browserOpen') and not panel.state().get('moduleEdit') and not panel.state().get('moduleReady'))
        browser=inspect();assert browser['right']['selectedNameCount']==0 and 'Visual fixture' not in browser['right']['text'],browser['right']['text']
        panel.command('button browser');panel.wait_for(lambda:not panel.state().get('browserOpen'));assert not panel.state()['moduleEdit']
        panel.command('button mixer');panel.wait_for(lambda:panel.state().get('view')=='mixer')
        base=f.status
        def with_modifier():
            s=base();s['configuration']['function']=True;s['problem']='Test notice';return s
        f.status=with_modifier;panel.wait_for(lambda:panel.state().get('configuration',{}).get('function'))
        dom=inspect();assert dom['right']['text'].strip()=='',dom['right']['text'];assert 'Master' not in dom['right']['text'] and 'master' not in dom['right']['text'];assert 'DAW' not in dom['right']['text'];assert 'No Output module' not in dom['left']['text'] and 'Multiple Outputs' not in dom['left']['text'];assert 'ANALOGBASS' in dom['left']['text'].upper() and 'DRUMS' in dom['left']['text'].upper(),dom['left']['text']
        assert panel.state()['labels'][1].startswith('Slot B') and panel.state()['labels'][2]=='UNASSIGNED' and panel.state()['labels'][3]=='UNASSIGNED'
        panel.command('turn 2 -0.1');panel.wait_for(lambda:panel.state()['displayValues'][1]=='94');assert panel.state()['displayValues'][0]=='100'
        target=folder/'page';panel.command('capture-preview '+str(target));panel.wait_for(lambda:any('preview captured: '+str(target) in x for x in panel.lines))
        for side in ['left','right']:Image.frombytes('RGBA',(480,272),(target/(side+'.rgba')).read_bytes()).save(target/(side+'.png'))
        f.status=base;panel.wait_for(lambda:not panel.state().get('configuration',{}).get('function'));panel.command('button mixer');panel.wait_for(lambda:panel.state().get('view')=='parameters');panel.command('quit');panel.process.wait(timeout=8)
        print('PASS slot palettes, TRACK+8 colour intensity/dual-screen saturation/Home; exclusive Module→Browser→Home, no repeated browser name; headers and blank Mixer screen2, bound encoder/readback, unavailable outputs, toggle; evidence:',folder)
    finally:
        if panel:panel.close()
        f.close()
if __name__=='__main__':main()
