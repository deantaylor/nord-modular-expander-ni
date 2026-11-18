"""Exercise the unmodified release through an external DirectLink adapter; never store banks."""
from pathlib import Path
import argparse,fcntl,hashlib,json,os,socket,subprocess,time
from fault_directlink import FaultLink
ROOT=Path(__file__).resolve().parents[1]

def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest() if p.exists() else None

def protect_plugin_data(work,bank):
    # Official alpha.30 does not support the legacy G1_DATA_DIR override.
    # Block access to the user's flash and writes to shared settings.
    shared=bank.parent
    protection=work/'official-test.sb'
    protection.write_text('(version 1)\n(allow default)\n'
        +'(deny file-read-data (literal '+json.dumps(str(bank))+'))\n'
        +'(deny file-write* (require-all (subpath '+json.dumps(str(shared))+') '
        +'(require-not (subpath '+json.dumps(str(shared/'locks'))+'))))\n')
    if not Path('/usr/bin/sandbox-exec').exists():raise RuntimeError('Official plugin test requires filesystem isolation')
    return ['/usr/bin/sandbox-exec','-f',str(protection)]

def discover():
    ports=[]
    for port in range(47310,47318):
        try:
            with socket.create_connection(('127.0.0.1',port),.12) as s:
                s.settimeout(.2);line=s.recv(1024)
                if line.startswith(b'G1-Emu 1 '):ports.append(port)
        except OSError:pass
    return ports

class Peer:
    def __init__(self,port):
        self.socket=socket.create_connection(('127.0.0.1',port),1);self.socket.setsockopt(socket.IPPROTO_TCP,socket.TCP_NODELAY,1);self.socket.settimeout(2);self.file=self.socket.makefile('rwb');self.hello=json.loads(self.file.readline());self.seq=0
    def call(self,method,**values):
        self.seq+=1;self.file.write((json.dumps(dict(id=self.seq,method=method,**values))+'\n').encode());self.file.flush();answer=json.loads(self.file.readline());assert answer.get('id')==self.seq,answer
        if 'error' in answer:raise RuntimeError(answer['error'])
        result=answer['result']
        if isinstance(result,dict) and 'rejected' in result:raise RuntimeError(result['rejected'])
        return result
    def close(self):self.file.close();self.socket.close()

def wait(fn,seconds=25):
    end=time.monotonic()+seconds
    while time.monotonic()<end:
        value=fn()
        if value:return value
        time.sleep(.05)
    raise AssertionError('Timed out waiting for confirmed official-plugin state')

def main():
    args=argparse.ArgumentParser(description=__doc__);args.add_argument('--playhead-only',action='store_true');args.add_argument('--sequencer-only',action='store_true');args.add_argument('--restore-only',action='store_true');args.add_argument('--readback-only',action='store_true');args.add_argument('--adapter',type=Path);args.add_argument('--lost-reply',action='store_true');args.add_argument('--mixed',action='store_true');args.add_argument('--module-edit',action='store_true');args.add_argument('--slot-selection-only',action='store_true');args.add_argument('--mixer-only',action='store_true');args.add_argument('--inactive-read-stall',action='store_true');args.add_argument('--patch-file',type=Path,default=ROOT/'patches/MK3-Bridge.pch');args=args.parse_args()
    # The watcher must never attach its keyboard to an isolated test plugin.
    # Refuse an active production owner rather than stopping it for a test.
    lock_path=Path.home()/'Library/Application Support/Modular Machine/controller-launcher.lock'
    lock_path.parent.mkdir(parents=True,exist_ok=True)
    test_lock=lock_path.open('a+')
    try:fcntl.flock(test_lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    except BlockingIOError:raise RuntimeError('Live keyboard bridge owns the controller; isolated test not started')
    work=ROOT/'g1-state'/f'official-check-{time.time_ns()}';work.mkdir(parents=True)
    bank=Path.home()/'.local/share/Animatek/G1-Emu/flash.bin';bank_before_digest=digest(bank)
    original=ROOT/'build/g1-official/alpha30/G1-Emu.vst3';host_binary=ROOT/'build/g1-emu/g1mk3plugincheck';adapter_binary=ROOT/'build/g1-official/adapter-build/g1keyboardadapter_artefacts/Release/g1keyboardadapter'
    if args.adapter:adapter_binary=args.adapter.resolve()
    sandbox=protect_plugin_data(work,bank)
    known=discover();host=adapter=peer=relay=panel=None
    try:
        with (work/'host.log').open('w') as log:
            host=subprocess.Popen([*sandbox,str(host_binary),str(original)],stdin=subprocess.PIPE,stdout=log,stderr=log,text=True,env={**os.environ,'G1_ROM':str(ROOT/'roms/NORD-MODULAR-RACK-VER-3.03.BIN'),'G1_DATA_DIR':str(work),'G1_PLUGIN_PC_PORT':'0'})
        port=wait(lambda:next((p for p in discover() if p not in known),None))
        with socket.socket() as probe:probe.bind(('127.0.0.1',0));json_port=probe.getsockname()[1]
        if args.lost_reply or args.inactive_read_stall:relay=FaultLink(port)
        with (work/'adapter.log').open('w') as log:adapter=subprocess.Popen([str(adapter_binary),str(relay.port if relay else port),str(json_port)],stdout=log,stderr=log,env={**os.environ,'G1_KEYBOARD_TRACE':'1'})
        def connect():
            try:return Peer(json_port)
            except OSError:return None
        peer=wait(connect)
        boot=peer.call('status')
        if not boot['running']:assert boot['knobs']==[], 'Unready metadata must match the controller contract'
        def ready():
            state=peer.call('status');(work/'startup-status.json').write_text(json.dumps(state,indent=2));return state['running']
        wait(ready,40)
        if args.sequencer_only or args.playhead_only:
            fixture=args.patch_file.read_text()
            module_marker='3 4 0 12\n';parameter_marker='3 4 3 127 0 0\n'
            assert module_marker in fixture and parameter_marker in fixture,'Sequencer fixture requires MK3-Bridge seed'
            fixture=fixture.replace(module_marker,module_marker+'4 15 0 18\n5 90 0 24\n6 91 0 30\n7 17 0 36\n',1)
            params=[(4,15,[64]*16+[15,0,0,0,1]),(5,90,[60]*16+[15,0,0,1,1]),(6,91,[64]*16+[15,1,1]),(7,17,[15,1,0,0]+[0]*32)]
            if args.playhead_only:params[1][2][19]=0
            fixture=fixture.replace(parameter_marker,parameter_marker+''.join(f'{index} {kind} {len(values)} '+ ' '.join(map(str,values))+'\n' for index,kind,values in params),1)
            if args.playhead_only:
                # Run all sequencers in the common area, which runs without keybed notes.
                seq_modules='4 15 0 18\n5 90 0 24\n6 91 0 30\n7 17 0 36\n'
                seq_params=''.join(f'{index} {kind} {len(values)} '+' '.join(map(str,values))+'\n' for index,kind,values in params)
                fixture=fixture.replace(seq_modules,'',1).replace(seq_params,'',1)
                fixture=fixture.replace('[ModuleDump]\n0\n','[ModuleDump]\n0\n'+seq_modules+'8 68 0 42\n',1)
                fixture=fixture.replace('[ParameterDump]\n0\n','[ParameterDump]\n0\n'+seq_params+'8 68 2 64 1\n',1)
                fixture=fixture.replace('[CableDump]\n0\n','[CableDump]\n0\n'+''.join(f'1 {module} 0 0 8 1 1\n' for module in [4,5,6,7]),1)
            args.patch_file=work/'sequencer-fixture.pch';args.patch_file.write_text(fixture)
        peer.call('load_file',slot=0,path=str(args.patch_file.resolve()))
        s=wait(lambda:(lambda s:s if s['running'] and s['upload']['ok'] and not s['upload']['busy'] else None)(peer.call('status')),45)
        if args.playhead_only:
            catalogue=peer.call('modules');seen_by_type={}
            for kind in ['NoteSeqA','NoteSeqB','CtrlSeq','EventSeq']:
                entry=next(m for m in catalogue['entries'] if m['typeName']==kind)
                peer.call('module_select',patchBinding=catalogue['patchBinding'],section=entry['section'],index=entry['index'])
                seen=set();end=time.monotonic()+8
                while time.monotonic()<end:
                    current=peer.call('status')['edit'].get('playhead')
                    if current is not None:assert 0<=current<16;seen.add(current)
                    if len(seen)>=3:break
                    time.sleep(.035)
                current=peer.call('status')['edit'];seen_by_type[kind]={'steps':sorted(seen),'meterBase':current.get('meterBase'),'meterCount':current.get('meterCount')}
                print('LIVE PLAYHEAD',kind,seen_by_type[kind],flush=True)
            (work/'playhead-values.json').write_text(json.dumps(seen_by_type,indent=2))
            assert all(len(values['steps'])>=3 for values in seen_by_type.values()),seen_by_type
            return
        if args.sequencer_only:
            catalogue=peer.call('modules');saved=[];before=peer.call('status')
            for kind,index in [('NoteSeqA',15),('NoteSeqB',15),('CtrlSeq',0),('EventSeq',35)]:
                entry=next(m for m in catalogue['entries'] if m['typeName']==kind)
                peer.call('module_select',patchBinding=catalogue['patchBinding'],section=entry['section'],index=entry['index'])
                status=peer.call('status');p=next(p for p in status['edit']['parameters'] if p['index']==index)
                target=p['value']+1 if p['value']<p['max'] else p['value']-1
                peer.call('set_parameters',targets=[dict(section=entry['section'],module=entry['index'],index=index,value=target,binding=p['binding'])])
                wait(lambda:(lambda s:any(p['index']==index and p['value']==target for p in s['edit']['parameters']))(peer.call('status')))
                saved.append((entry,index,p['value']));print('PASS actual sequencer step edit/readback',kind,index,flush=True)
            peer.call('restore_slot',slot=0,patchBinding=catalogue['patchBinding'])
            restored=wait(lambda:(lambda s:s if not s['restore']['busy'] else None)(peer.call('status')),20)
            assert restored['restore']['message']=='Original slot values restored' and restored['restore']['written']==4,restored['restore']
            assert restored['patchBinding']==before['patchBinding'] and restored['upload']['serial']==before['upload']['serial'],'Sequencer edits reloaded patch'
            for entry,index,value in saved:
                peer.call('module_select',patchBinding=catalogue['patchBinding'],section=entry['section'],index=entry['index'])
                assert next(p['value'] for p in peer.call('status')['edit']['parameters'] if p['index']==index)==value
            print('PASS CLEAR restores all four sequencers without reload',flush=True)
            return
        if args.restore_only:
            def select(slot):
                peer.call('select_slot',slot=slot)
                return wait(lambda:(lambda s:s if s['activeSlot']==slot and s['running'] else None)(peer.call('status')))
            def exercise(slot):
                before=peer.call('status');assert before['restore']['baselineAvailable'] and not before['restore']['available']
                catalogue=peer.call('modules');osc=next(m for m in catalogue['entries'] if m['typeName']=='OscA')
                peer.call('module_select',patchBinding=catalogue['patchBinding'],section=osc['section'],index=osc['index'])
                original=peer.call('status');knob=next(k for k in original['knobs'] if k['assigned']);param=next(p for p in original['edit']['parameters'] if p['index']==3)
                target=knob['value']+1 if knob['value']<knob['max'] else knob['value']-1
                peer.call('set_knobs',targets=[dict(index=knob['index'],binding=knob['binding'],value=target)])
                peer.call('set_parameters',targets=[dict(section=osc['section'],module=osc['index'],index=param['index'],binding=param['binding'],value=param['min'] if param['value']!=param['min'] else param['max'])])
                wait(lambda:(lambda s:s['knobs'][knob['index']]['value']==target and next(p['value'] for p in s['edit']['parameters'] if p['index']==param['index'])!=param['value'])(peer.call('status')))
                assert peer.call('status')['restore']['available'],'Changed slot did not enable CLEAR'
                peer.call('restore_slot',slot=slot,patchBinding=original['patchBinding'])
                restored=wait(lambda:(lambda s:s if not s['restore']['busy'] and s['restore']['serial']>original['restore']['serial'] else None)(peer.call('status')),20)
                assert restored['restore']['message']=='Original slot values restored',restored['restore']
                assert restored['restore']['written']==2 and restored['restore']['total']==2,restored['restore']
                assert not restored['restore']['available'],'Restored slot kept CLEAR enabled'
                peer.call('restore_slot',slot=slot,patchBinding=original['patchBinding'])
                noop=peer.call('status');assert not noop['restore']['busy'] and noop['restore']['written']==0 and noop['restore']['total']==0,noop['restore']
                assert [(k['value'],k['binding']) for k in restored['knobs']]==[(k['value'],k['binding']) for k in original['knobs']]
                assert [(p['index'],p['value']) for p in restored['edit']['parameters']]==[(p['index'],p['value']) for p in original['edit']['parameters']]
                assert restored['patchBinding']==original['patchBinding'] and restored['upload']['serial']==original['upload']['serial'] and restored['lcd']==original['lcd'],'Restore reloaded the patch'
                return restored
            first=exercise(0);print('PASS exactly two changed parameter writes, clean-slot zero-write CLEAR, confirmed readback, no patch reload',flush=True)
            peer.call('load_file',slot=1,path=str(args.patch_file.resolve()))
            wait(lambda:(lambda s:s['upload']['ok'] and not s['upload']['busy'])(peer.call('status')),25)
            other=select(1);k=next(k for k in other['knobs'] if k['assigned']);changed=k['value']+2 if k['value']+2<=k['max'] else k['value']-2
            peer.call('set_knobs',targets=[dict(index=k['index'],binding=k['binding'],value=changed)])
            wait(lambda:peer.call('status')['knobs'][k['index']]['value']==changed)
            select(0);peer.call('restore_slot',slot=0,patchBinding=first['patchBinding'])
            wait(lambda: not peer.call('status')['restore']['busy'],20)
            assert select(1)['knobs'][k['index']]['value']==changed,'Restoring A altered B'
            peer.call('restore_slot',slot=1,patchBinding=other['patchBinding']);wait(lambda:not peer.call('status')['restore']['busy'],20)
            assert peer.call('status')['knobs'][k['index']]['value']==k['value'],(k,peer.call('status'))
            select(0);changed_patch=work/'restore-fresh-baseline.pch';changed_patch.write_text(args.patch_file.read_text().replace('1 7 10 64','1 7 10 71',1))
            peer.call('load_file',slot=0,path=str(changed_patch))
            fresh=wait(lambda:(lambda s:s if s['running'] and s['upload']['ok'] and not s['upload']['busy'] and s['patchBinding']!=first['patchBinding'] else None)(peer.call('status')),25)
            assert fresh['knobs'][0]['value']==71
            try:peer.call('restore_slot',slot=0,patchBinding=first['patchBinding'])
            except RuntimeError:pass
            else:raise AssertionError('Stale restore binding accepted')
            assert exercise(0)['knobs'][0]['value']==71
            print('PASS per-slot isolation, independent baselines, stale-binding rejection and new-load baseline',flush=True)
            return
        if args.mixer_only:
            for slot in [1,2,3]:
                peer.call('load_file',slot=slot,path=str(args.patch_file.resolve()))
                wait(lambda:(lambda s:s['upload']['ok'] and not s['upload']['busy'])(peer.call('status')),45)
            peer.call('mixer_mode',enabled=True)
            def levels():return [c for c in peer.call('status')['configuration']['controls'] if c['key'].startswith('slotLevel:')]
            controls=wait(lambda:(lambda c:c if len(c)==4 and all(x['max']==127 for x in c) else None)(levels()),30)
            active=peer.call('status')['activeSlot'];keys=[c['key'] for c in controls]
            expected=[c['value'] for c in controls]
            if args.inactive_read_stall:
                relay.drop_reads=True;peer.call('set_control',key=keys[1],value=112);wait(lambda:relay.dropped>0,3)
                relay.drop_reads=False;knob=next(k for k in peer.call('status')['knobs'] if k['assigned']);target=max(0,knob['value']-2) if knob['value']>=2 else 2
                start=time.monotonic();peer.call('set_knobs',targets=[dict(index=knob['index'],binding=knob['binding'],value=target)])
                wait(lambda:peer.call('status')['knobs'][knob['index']]['value']==target,.4)
                print('PASS active knob readback bypassed lost inactive-slot response in',round((time.monotonic()-start)*1000),'ms',flush=True)
                wait(lambda:levels()[1]['value']==112,3);expected=[c['value'] for c in levels()]
            for slot,value in [(1,93),(3,77),(0,101),(2,86)]:
                reply=peer.call('set_control',key=keys[slot],value=value);assert reply['previews'][0]['binding']==keys[slot]
                expected[slot]=value;wait(lambda:[c['value'] for c in levels()]==expected,5)
                assert peer.call('status')['activeSlot']==active,'Mixer edit changed slot focus'
                print('PASS isolated slot',slot,'output level',value,'other slots unchanged',flush=True)
            peer.call('set_control',key=keys[0],value=111);time.sleep(.04)
            for target in [1,0]:
                peer.call('select_slot',slot=target);wait(lambda:(lambda s:s['activeSlot']==target and s['running'])(peer.call('status')),2)
            wait(lambda:levels()[0]['value']==111,5);print('PASS slot switching while Mixer edits/readback are active',flush=True)
            try:peer.call('set_control',key=keys[0],value=128)
            except RuntimeError:pass
            else:raise AssertionError('Out of range mixer target accepted')
            def replace_when_idle():
                try:peer.call('load_file',slot=1,path=str(args.patch_file.resolve()));return True
                except RuntimeError as e:
                    if 'Wait for the current patch operation' in str(e):return False
                    raise
            wait(replace_when_idle,10);wait(lambda:(lambda s:s['upload']['ok'] and not s['upload']['busy'])(peer.call('status')),45)
            wait(lambda:levels()[1]['key']!=keys[1],10)
            try:peer.call('set_control',key=keys[1],value=60)
            except RuntimeError:pass
            else:raise AssertionError('Stale mixer binding accepted after patch replacement')
            peer.call('mixer_mode',enabled=False);assert not levels()
            knob=next(k for k in peer.call('status')['knobs'] if k['assigned']);value=max(0,knob['value']-1) if knob['value'] else 1
            peer.call('set_knobs',targets=[dict(index=knob['index'],binding=knob['binding'],value=value)]);wait(lambda:peer.call('status')['knobs'][knob['index']]['value']==value,5)
            print('PASS mixer stale/range guards, exit cleanup and normal knob edits afterward')
            if args.inactive_read_stall:
                peer.call('mixer_mode',enabled=True);control=levels()[1];before=len(relay.requests);relay.drop_reads=True
                peer.call('set_control',key=control['key'],value=73);wait(lambda:levels()[1]['max']==0,1.5)
                count=len(relay.requests)-before;time.sleep(.65);assert len(relay.requests)-before==count,'Failed inactive read retried indefinitely';assert count<=2,count
                relay.drop_reads=False;peer.call('mixer_mode',enabled=False);print('PASS inactive read failure stops after',count,'requests, other slots stay usable')
            return
        if args.slot_selection_only:
            timings=[]
            for target in [1,0,2,0,3,0]:
                start=time.monotonic();peer.call('select_slot',slot=target)
                try:
                    state=wait(lambda:(lambda s:s if s['activeSlot']==target else None)(peer.call('status')),2)
                except AssertionError:
                    state=peer.call('status');(work/'slot-failure.json').write_text(json.dumps(dict(target=target,status=state),indent=2));raise
                wait(lambda:peer.call('status')['running'],10)
                elapsed=time.monotonic()-start;timings.append(dict(slot=target,seconds=elapsed));print('Confirmed slot',target,'in',round(elapsed*1000),'ms',flush=True)
            (work/'slot-timings.json').write_text(json.dumps(timings,indent=2));print('PASS official slot selection and return to A within controller deadline');return
        assigned=[k for k in s['knobs'] if k['assigned']];assert assigned or args.mixed,s
        (work/'before-knob.json').write_text(json.dumps(s,indent=2));k=assigned[0] if assigned else None;target=(max(0,k['value']-12) if k['value']>=12 else min(k['max'],k['value']+12)) if k else 0
        if args.module_edit:
            catalogue=peer.call('modules');entries=catalogue['entries'];assert entries
            (work/'module-catalogue.json').write_text(json.dumps(catalogue,indent=2))
            osc=next(m for m in entries if m['typeName']=='OscA');other=next(m for m in entries if (m['section'],m['index'])!=(osc['section'],osc['index']))
            key=catalogue['patchBinding'];peer.call('module_select',patchBinding=key,section=osc['section'],index=osc['index'])
            state=peer.call('status');edit=state['edit'];assert len(edit['parameters'])==10,edit
            knob_map=[(x['index'],x['assigned'],x['binding']) for x in state['knobs']]
            p=next(x for x in edit['parameters'] if x['index']==3)
            target=p['min'] if p['value']!=p['min'] else p['max']
            def request(value,binding=p['binding']):return dict(section=osc['section'],module=osc['index'],index=p['index'],binding=binding,value=value)
            try:peer.call('set_parameters',targets=[request(p['max']+1)])
            except RuntimeError:pass
            else:raise AssertionError('Out-of-range parameter accepted')
            peer.call('set_parameters',targets=[request(target)])
            state=wait(lambda:(lambda s:s if any(x['index']==p['index'] and x['value']==target for x in s['edit']['parameters']) else None)(peer.call('status')))
            assert [(x['index'],x['assigned'],x['binding']) for x in state['knobs']]==knob_map,'Module edit rewrote saved knob assignments'
            peer.call('module_select',patchBinding=key,section=other['section'],index=other['index']);assert len(peer.call('status')['edit']['parameters'])==other['parameterCount']
            try:peer.call('set_parameters',targets=[request(target)])
            except RuntimeError:pass
            else:raise AssertionError('Old module target accepted after navigation')
            peer.call('module_exit');assert peer.call('status')['edit'] is None
            print('PASS official module catalogue, ten actual OscA parameters, unassigned-parameter write/readback, range/stale-module guards, module navigation, unchanged assignments',flush=True)
            peer.close();peer=None
            import check_mock
            check_mock.BINARY=ROOT/'target/release/nord-modular-ni-interface'
            panel=check_mock.Session(['--mock','--g1','--g1-address',f'127.0.0.1:{json_port}','--patch-dir',str(ROOT/'patches')])
            panel.wait_for(lambda:panel.state().get('engine')=='G1-EMU / LIVE',20)
            panel.command('button module_edit');panel.wait_for(lambda:panel.state().get('moduleReady') and panel.state().get('moduleEdit'),15)
            assert panel.state()['pageCount']==2,panel.state()
            before_display=panel.state()['displayValues'][3];panel.command('turn 4 0.2')
            panel.wait_for(lambda:panel.state()['displayValues'][3]!=before_display,10)
            panel.command('button knob_page_next');panel.wait_for(lambda:panel.state().get('page')==1)
            count=sum(x.startswith('mock DOM') for x in panel.lines);panel.command('inspect');panel.wait_for(lambda:sum(x.startswith('mock DOM') for x in panel.lines)>=count+2)
            right=json.loads(next(x.split(': ',1)[1] for x in reversed(panel.lines) if x.startswith('mock DOM right')))
            assert 'Module Edit' in right['text'] and 'PAGE 2 / 2' in right['text'],right['text']
            panel.command('button top_2');panel.wait_for(lambda:panel.state().get('moduleCursor')==1 and panel.state().get('moduleReady'))
            panel.command('button module_edit');panel.wait_for(lambda:not panel.state().get('moduleEdit') and panel.state().get('pageCount')==3)
            panel.command('quit');panel.process.wait(timeout=8);panel.close();panel=None
            print('PASS real controller→official VST module edit, encoder readback, page2/2 WebKit render, module navigation and assigned-view return',flush=True)
            return
        if args.mixed:
            timings=[]
            for cycle in range(12):
                state=peer.call('status');assigned=[x for x in state['knobs'] if x['assigned']]
                if assigned:
                    knob=assigned[0]
                    for step in range(5):
                        peer.call('set_knobs',targets=[dict(index=knob['index'],binding=knob['binding'],value=round(knob['max']*(.3 if step%2 else .7)))])
                        time.sleep(.012)
                serial=state['upload']['serial'];started=time.monotonic()
                path=args.patch_file.resolve() if cycle%2 else ROOT/'patches/MK3-MIDI.pch'
                peer.call('load_file',slot=0,path=str(path))
                def completed():
                    state=peer.call('status');upload=state['upload']
                    if upload['serial']>serial and not upload['busy']:
                        assert upload['ok'], f'Mixed load {cycle} failed: {upload}'
                        return state if state['running'] else None
                    return None
                state=wait(completed,20);timings.append((time.monotonic()-started)*1000)
                print(f'Mixed load {cycle}: {timings[-1]:.1f}ms',flush=True)
            (work/'mixed-loads.json').write_text(json.dumps(timings))
            print(f'PASS mixed knob edits and12 patch loads; maximum confirmed load{max(timings):.1f}ms')
            return
        if args.lost_reply:
            relay.drop_reads=True;start=time.monotonic();sent=0
            while time.monotonic()-start<4:
                target=round(k['max']*(.3 if sent%2 else .7))
                peer.call('set_knobs',targets=[dict(index=k['index'],binding=k['binding'],value=target)])
                sent+=1;time.sleep(.02)
            relay.drop_reads=False
            writes=[(t-start,v) for t,v in relay.writes if t>=start]
            gaps=[b[0]-a[0] for a,b in zip(writes,writes[1:])]
            max_gap=max(gaps+[4-writes[-1][0] if writes else 4])
            summary=dict(sent=sent,wireWrites=writes,droppedReplies=relay.dropped,maxWireGapMs=max_gap*1000)
            (work/'lost-reply.json').write_text(json.dumps(summary,indent=2))
            assert relay.dropped>0, 'Fault injection did not withhold any replies'
            assert len(writes)>=40 and max_gap<.3, summary
            peer.call('set_knobs',targets=[dict(index=k['index'],binding=k['binding'],value=target)])
            wait(lambda:peer.call('status')['knobs'][k['index']]['value']==target,10)
            print(f'PASS lost replies: {len(writes)} actual wire writes during4s; max gap{max_gap*1000:.1f}ms, dropped{relay.dropped} replies; confirmed recovery')
            return
        if args.readback_only:
            latencies=[]
            for step in range(16):
                target=round(k['max']*(.25 if step%2 else .75));sent=time.monotonic()
                peer.call('set_knobs',targets=[dict(index=k['index'],binding=k['binding'],value=target)])
                while True:
                    actual=peer.call('status')['knobs'][k['index']]['value']
                    if actual==target:break
                    assert time.monotonic()-sent<2, 'Parameter failed confirmation'
                    time.sleep(.001)
                latencies.append((time.monotonic()-sent)*1000)
            print(f'Command-to-confirmation median {sorted(latencies)[len(latencies)//2]:.1f}ms, max {max(latencies):.1f}ms',flush=True)
            (work/'command-latency.json').write_text(json.dumps(latencies))
            samples=[];confirmed=[];start=time.monotonic()
            # Keep sending through two reversals: no idle gap can release a
            # debounce. Only actual engine-confirmed values count as progress.
            for direction in (1,-1,1):
                count_before=len(confirmed)
                for step in range(48):
                    value=round(k['max']*(.2+.6*(step/47 if direction>0 else 1-step/47)))
                    peer.call('set_knobs',targets=[dict(index=k['index'],binding=k['binding'],value=value)])
                    state=peer.call('status');actual=state['knobs'][k['index']]['value'];now=time.monotonic()-start
                    samples.append(dict(seconds=now,target=value,confirmed=actual))
                    if not confirmed or actual!=confirmed[-1]['value']:confirmed.append(dict(seconds=now,value=actual))
                    time.sleep(.012)
                assert len(confirmed)-count_before>=4, f'Confirmed values stalled during continuous movement: {confirmed}'
            wait(lambda:peer.call('status')['knobs'][k['index']]['value']==value,10)
            gaps=[b['seconds']-a['seconds'] for a,b in zip(confirmed,confirmed[1:])]
            (work/'continuous-readback.json').write_text(json.dumps(dict(samples=samples,confirmed=confirmed,maxGapSeconds=max(gaps)),indent=2))
            print(f'PASS continuous/reversing encoder: {len(samples)} writes, {len(confirmed)} confirmed changes while moving, maximum confirmation gap {max(gaps)*1000:.1f}ms')
            return
        peer.call('function',pressed=True)
        assert peer.call('status')['configuration']['function'] is True
        peer.call('set_knobs',targets=[dict(index=k['index'],binding=k['binding'],value=target)])
        time.sleep(.15)
        assert peer.call('status')['knobs'][k['index']]['value']==k['value'], 'Function-held movement changed a parameter'
        peer.call('select_slot',slot=1)
        shifted=wait(lambda:(lambda s:s if s['configuration']['selectedSlots'][1] else None)(peer.call('status')),10)
        assert shifted['activeSlot']==0, 'Shift slot toggle changed focus'
        peer.call('select_slot',slot=1)
        wait(lambda:not peer.call('status')['configuration']['selectedSlots'][1],10)
        peer.call('function',pressed=False)
        assert peer.call('status')['configuration']['function'] is False
        print('PASS held Function gates parameter changes and toggles slot enable without changing focus')
        peer.call('set_knobs',targets=[dict(index=k['index'],binding=k['binding'],value=target)])
        def changed():
            value=peer.call('status');(work/'after-knob.json').write_text(json.dumps(value,indent=2));return value if value['knobs'][k['index']]['value']==target else None
        s=wait(changed,20)
        assert s['knobs'][k['index']]['binding']==k['binding'],'A value refresh changed assignment identity'
        assert 'allocatedVoices' in s and 'requestedVoices' in s,s
        print('PASS unmodified official VST: DirectLink, patch fetch/upload, real knob assignments, confirmed parameter change and voice counts')
        old=k['binding'];peer.call('load_file',slot=0,path=str(ROOT/'patches/MK3-MIDI.pch'))
        s=wait(lambda:(lambda s:s if s['upload']['serial']>=2 and s['upload']['ok'] and not s['upload']['busy'] else None)(peer.call('status')),45)
        try:peer.call('set_knobs',targets=[dict(index=k['index'],binding=old,value=target)])
        except RuntimeError:pass
        else:raise AssertionError('Stale target accepted')
        print('PASS stale assignment rejected after patch change; keybed MIDI is not opened by adapter')
    finally:
        if panel:panel.close()
        if peer:peer.close()
        if adapter and adapter.poll() is None:adapter.terminate();adapter.wait(timeout=10)
        if host and host.poll() is None:
            host.stdin.write('{"method":"quit"}\n');host.stdin.flush()
            try:host.wait(timeout=15)
            except subprocess.TimeoutExpired:host.terminate();host.wait(timeout=10)
        if relay:relay.close()
        test_lock.close()
        assert digest(bank)==bank_before_digest,'Shared bank file changed during RAM-only verification'
        print('Shared bank file unchanged; evidence:',work)

if __name__=='__main__':main()
