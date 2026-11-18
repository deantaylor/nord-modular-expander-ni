const assert=require('assert'),bloom=require('../screens/midi-bloom.js');
const still=bloom.frame(0,0),breath=bloom.frame(2,0),hit=bloom.frame(0,1,1);
assert(still.length>50&&still.length<=4235);assert.notDeepEqual(still,breath);
assert(hit.length>still.length,'Incoming notes enlarge the halftone field');
for(const t of [0,1,20,10000])for(const e of [0,.5,1])for(const d of bloom.frame(t,e)){
 assert(d.depth>=0&&d.depth<=1);
 assert(Number.isFinite(d.x)&&Number.isFinite(d.y)&&d.r>0&&d.r<=3.2);
 assert(d.x>=-4&&d.x<=964&&d.y>=-4&&d.y<=276,'Animation stays inside its canvas');
}
console.log('PASS idle breathing, MIDI growth, bounded dot count/geometry');

assert(new Set(bloom.frame(0,0).map(d=>Math.floor(d.depth*6))).size>=4,'Depth shading varies across the field');

const held={connected:true,activeNotes:2,heldVelocity:100,heldPeakNotes:2};
assert.deepEqual(bloom.activity(held,0),bloom.activity(held,3600000),'Held notes do not decay before note-off');
const released={connected:true,activeNotes:0,releaseTimestampMs:1000,releaseVelocity:100,releaseNotes:2};
assert(bloom.activity(released,2000).energy<bloom.activity(released,1000).energy);
assert.equal(bloom.activity({...held,connected:false},1000).energy,0);
let glitches=0;for(let i=0;i<2000;i++)if(bloom.glitch(i*.05,1).active)glitches++;
assert(glitches>0&&glitches<200,'Glitches occur occasionally rather than continuously');
console.log('PASS held-note sustain, note-off release, occasional synchronized glitches');

let torn=false;for(let i=0;i<2000&&!torn;i++){const t=i*.05;if(bloom.glitch(t,1).active)torn=bloom.frame(t,1,1).some(d=>d.glitch);}assert(torn,'Occasional glitches actually affect dot rows');

for(let i=0;i<100;i++)assert(bloom.frame(i*.5,1,1,false).every(d=>!d.glitch),'Glitches Off disables all dot tears');

const counts=[.25,.5,1].map(i=>{let count=0;for(let t=0;t<1000;t+=.05)if(bloom.glitch(t,1,i).active)count++;return count;});assert(counts[0]<counts[1]&&counts[1]<counts[2],counts);
for(let t=0;t<100;t+=.05){assert.equal(bloom.alert(t,.99),null);assert.equal(bloom.glitch(t,1,0).active,false);}
let astate={phase:0,last:null},phases=[];for(let frame=0;frame<5;frame++){astate=bloom.nextAlert(astate.phase,astate.last,123,1);phases.push(astate.phase);}assert.deepEqual(phases,[1,2,3,0,0]);assert.equal(bloom.nextAlert(2,123,123,.5).phase,0);
console.log('PASS glitch intensity scales frequency; ALERT clear/glitch/clear/hidden once per event only at100%');

let jump=null;for(let t=0;t<50;t+=.01){assert.equal(bloom.themeGlitch(t,.9,'dark'),null);const theme=bloom.themeGlitch(t,1,'dark');if(theme){assert.notEqual(theme,'dark');if(jump===null)jump=t;}}
assert.notEqual(jump,null);assert.equal(bloom.themeGlitch(jump+.3,1,'dark'),null,'Theme bursts return to selected palette');
console.log('PASS temporary alternate themes only above90%, automatic restoration');

for(const selected of ['light','dark'])for(let t=0;t<100;t+=.02){const v=bloom.themeGlitch(t,1,selected);assert(v===null||v==='light'||v==='dark');}
const sleeping={screensaver:true,screensaverSinceMs:1000,glitchIntensity:.4,uiTheme:'colour',blobsEnabled:false};
assert.equal(bloom.mode(sleeping,1000).intensity,.4);
assert.equal(bloom.mode(sleeping,1801000).intensity,.7);
assert.equal(bloom.mode(sleeping,3601000).intensity,1);
assert.equal(bloom.mode(sleeping,7201000).intensity,1);
assert.equal(bloom.mode(sleeping,1000).theme,'dark');assert(bloom.mode(sleeping,1000).blobs);
assert.equal(bloom.mode({...sleeping,screensaver:false},1000).intensity,0);
assert.equal(bloom.mode({...sleeping,screensaver:false,uiTheme:'dark'},1000).intensity,.4);
console.log('PASS saver ramp, forced blobs, restored preferences and monochrome-only glitches');

const rareEvents=new Set(),alertEvents=new Set(),words=new Set();let heldFrames=0,tornMessage=0;
for(let step=0;step<200000;step++){
 const t=step*.02,m=bloom.saverMessage(t,1,true),a=bloom.alert(t,1);
 if(a!==null)alertEvents.add(a);
 assert.equal(bloom.saverMessage(t,.99,true),null);assert.equal(bloom.saverMessage(t,1,false),null);
 if(m){rareEvents.add(m.event);words.add(m.text);if(m.glitch)tornMessage++;else heldFrames++;}
}
assert.deepEqual([...words].sort(),['CONFORM','FEED ME']);assert(rareEvents.size<alertEvents.size/3);
assert(heldFrames/tornMessage>10,'Two-second static hold precedes brief exit tear');
for(let t=0;t<40;t+=.001){const m=bloom.saverMessage(t,1,true);if(m&&!m.glitch){assert.equal(bloom.saverMessage(t+1.99,1,true)?.text,m.text);break;}}
console.log('PASS rare FEED ME/CONFORM schedule, two-second static hold, exit glitch, full-intensity saver only');
