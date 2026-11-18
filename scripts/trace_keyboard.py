"""Read current keyboard pipeline logs without controlling the synth or keyboard."""
import argparse
from collections import Counter
from datetime import datetime,timezone
from pathlib import Path
import re,time

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dir',type=Path,default=Path.home()/'Library/Application Support/Modular Machine')
    args=parser.parse_args();events=[];counts=Counter()
    for name in ('keyboard-controller.log','adapter-session.log'):
        path=args.dir/name
        if not path.exists():continue
        for raw in path.read_text(errors='replace').splitlines():
            line=re.sub(r'\x1b\[[0-9;]*m','',raw)
            if 'PIPELINE ' not in line:continue
            stage=re.search(r'\bstage=(\w+)',line)
            if not stage:continue
            wall=re.search(r'\bts_ms=(\d+)',line)
            if wall:stamp=int(wall[1])/1000
            else:
                iso=re.match(r'(\d{4}-\d\d-\d\dT[\d:.]+Z)',line)
                if not iso:continue
                stamp=datetime.fromisoformat(iso[1].replace('Z','+00:00')).timestamp()
            body=line[line.index('PIPELINE ')+9:];events.append((stamp,stage[1],body));counts[stage[1]]+=1
    events.sort()
    print('Pipeline stage counts:',dict(counts))
    if not events:print('No trace events in current logs.');return
    print('Newest event age: %.1fs' % (time.time()-events[-1][0]))
    targets={};captures={};write_delays=[];transfer_delays=[];js=[]
    for stamp,stage,body in events:
        sequence=re.search(r'\binput_seq=(\d+)',body)
        frame=re.search(r'\bframe_hash=([\da-f]+)',body)
        if stage=='target' and sequence:targets[sequence[1]]=stamp
        if stage=='protocol_send' and sequence and sequence[1] in targets:
            write_delays.append((stamp-targets[sequence[1]])*1000)
        if stage=='capture' and frame:captures[frame[1]]=stamp
        if stage=='screen_transfer_done' and frame and frame[1] in captures:
            transfer_delays.append((stamp-captures[frame[1]])*1000)
        if stage=='js_complete':
            elapsed=re.search(r'elapsed_ms=([\d.]+)',body)
            if elapsed:js.append(float(elapsed[1]))
    for label,values in [('target to protocol send',write_delays),('state JS execution',js),('capture to host transfer completion',transfer_delays)]:
        if values:print('%s: n=%d median=%.1fms max=%.1fms' % (label,len(values),sorted(values)[len(values)//2],max(values)))
    print('Latest events (UTC):')
    for stamp,_,body in events[-12:]:print(datetime.fromtimestamp(stamp,timezone.utc).isoformat(timespec='milliseconds'),body)
    print('Coalesced/redundant events may be skipped. Host transfer completion is not LCD scanout.')

if __name__=='__main__':main()
