// One continuous halftone field across the 960×272 surface.
const MidiBloom=(()=>{
 const random=seed=>{const n=Math.sin(seed*127.1+311.7)*43758.5453;return n-Math.floor(n);};
 function glitch(seconds,energy=0,intensity=.5){
  const i=Math.max(0,Math.min(1,intensity)),period=3/(.4+1.2*i),epoch=Math.floor(seconds/period),phase=seconds-epoch*period,start=.1*period+random(epoch+4)*period*.7333333333;
  return {active:i>0&&random(epoch+1)>1-.8*i&&phase>=start&&phase<start+(.12+Math.max(0,energy)*.06)*(.7+.6*i),y:random(epoch+2)*272,height:8+random(epoch+3)*18,dx:(random(epoch+5)-.5)*(14+energy*14)*(.5+i),seed:epoch};
 }
 function alert(seconds,intensity){
  if(intensity<.999999)return null;
  const epoch=Math.floor(seconds/8),phase=seconds-epoch*8,start=1+random(epoch+91)*5;
  return phase>=start&&phase<start+.2?epoch:null;
 }
 // Shared wall-clock schedule keeps both displays on the same rare message.
 function saverMessage(seconds,intensity,sleeping){
  if(!sleeping||intensity<.999999)return null;
  const epoch=Math.floor(seconds/40),phase=seconds-epoch*40,start=4+random(epoch+701)*30;
  if(phase<start||phase>=start+2.18)return null;
  return {text:random(epoch+702)<.5?'FEED ME':'CONFORM',glitch:phase>=start+2,event:epoch};
 }
 function themeGlitch(seconds,intensity,selected){
  if(intensity<=.9)return null;
  const power=Math.min(1,(intensity-.9)/.1),period=2.2-.8*power,epoch=Math.floor(seconds/period),phase=seconds-epoch*period,start=.15*period+random(epoch+218)*period*.65;
  if(random(epoch+219)>.35+.45*power||phase<start||phase>=start+.1+.08*power)return null;
  const choices=['light','dark'].filter(t=>t!==selected),step=Math.floor((phase-start)/.05);
  return choices[(Math.floor(random(epoch+220)*2)+step)%choices.length];
 }
 function nextAlert(phase,last,event,intensity){
  if(intensity<.999999)return {phase:0,last};
  if(phase>0)return {phase:phase>=3?0:phase+1,last};
  return event!==null&&event!==last?{phase:1,last:event}:{phase:0,last};
 }
 function mode(data,nowMs){
  const saved=Math.max(0,Math.min(1,Number(data.glitchIntensity??.5))),sleeping=!!data.screensaver,selected=data.uiTheme||'light';
  const progress=sleeping?Math.max(0,Math.min(1,(nowMs-Number(data.screensaverSinceMs||nowMs))/3600000)):0;
  return {sleeping,theme:sleeping&&selected==='colour'?'dark':selected,blobs:sleeping||data.blobsEnabled!==false,intensity:sleeping?saved+(1-saved)*progress:selected==='colour'?0:saved};
 }
 function activity(midi,nowMs){
  if(midi.connected===false)return {energy:0,amount:0};
  if(Number(midi.activeNotes)>0)return {energy:Math.min(1,Number(midi.heldVelocity||midi.velocity||0)/127),amount:Math.min(1,Math.sqrt(Number(midi.heldPeakNotes||midi.activeNotes))/2.3)};
  const released=Number(midi.releaseTimestampMs||0);if(!released)return {energy:0,amount:0};
  const age=Math.max(0,nowMs-released);
  return {energy:Math.min(1,Number(midi.releaseVelocity||0)/127)*Math.exp(-age/900),amount:Math.min(1,Math.sqrt(Number(midi.releaseNotes||0))/2.3)*Math.exp(-age/1100)};
 }
 function frame(seconds,energy=0,amount=0,intensity=.5){
  const e=Math.max(0,Math.min(1,energy)),a=Math.max(0,Math.min(1,amount)),dots=[],fault=glitch(seconds,e,intensity===true?.5:intensity===false?0:intensity);
  const cx=480+Math.sin(seconds*.21)*310,cy=136+Math.cos(seconds*.29)*54;
  const rx=105+Math.sin(seconds*.8)*7+e*120+a*100,ry=66+Math.cos(seconds*.7)*5+e*48+a*35;
  const lobes=[{x:cx,y:cy,rx,ry,strength:1}];
  if(a>.08)lobes.push({x:480+Math.cos(seconds*.17+2)*370,y:136+Math.sin(seconds*.31+1)*85,rx:75+a*150,ry:45+a*60,strength:a*.85});
  if(a>.35)lobes.push({x:480+Math.sin(seconds*.19+4)*400,y:136+Math.cos(seconds*.23)*90,rx:70+a*100,ry:40+a*50,strength:(a-.35)*1.2});
  for(let row=0;row<=34;row++)for(let col=0;col<=120;col++){
   const gx=col*8,gy=row*8;let field=0;
   for(const l of lobes){const x=(gx-l.x)/l.rx,y=(gy-l.y)/l.ry,angle=Math.atan2(y,x),r=Math.hypot(x,y),edge=1+.1*Math.sin(angle*3+seconds*.51)+.06*Math.cos(angle*5-seconds*.39);field+=Math.max(0,1-r*r/(edge*edge))*l.strength;}
   if(field<.015)continue;
   const tear=fault.active&&Math.abs(gy-fault.y)<fault.height/2;if(tear&&random(col*13+row+fault.seed)>.82)continue;
   let x=gx+Math.sin(gy*.025+seconds*.47)*(1+e*3),y=gy+Math.cos(gx*.019-seconds*.39)*(1+e*3);
   if(tear)x=Math.max(-3,Math.min(963,x+fault.dx));
   const light=Math.max(0,Math.min(1,.5-(gx-cx)/rx*.2-(gy-cy)/ry*.3)),depth=Math.min(1,.75*Math.sqrt(Math.min(1,field))+.25*light);
   dots.push({x,y,depth,glitch:tear,r:Math.min(3.2,.2+2.8*Math.pow(Math.min(1,field),.75)),alpha:Math.min(1,field*2)});
  }
  return dots;
 }
 return {frame,activity,glitch,alert,nextAlert,themeGlitch,mode,saverMessage};
})();
if(typeof module!=='undefined'&&module.exports)module.exports=MidiBloom;
