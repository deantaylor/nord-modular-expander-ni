// SPDX-License-Identifier: GPL-3.0-only
// ADSR and Filter E/F schematic geometry adapted from Animatek NME,
// Source/ui/PatchCanvasPainting.cpp (JTEnvelopeDisplay / FilterE / FilterF),
// revision 0ded1bb77ca2431b7ff52a8b55076430d85ac48d. See THIRD_PARTY_NOTICES.md.
// Parameter shapes only: no live signal, modulation phase or DSP measurement.
const ModuleVisuals=(()=>{
 const clamp=(v,a=0,b=1)=>Math.max(a,Math.min(b,v));
 const escape=s=>String(s).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&apos;'}[c]));
 const number=v=>Number(v).toFixed(2);
 const X=v=>2+460*v,Y=v=>2+128*v;
 const point=(x,y)=>`${number(X(x))},${number(Y(y))}`;
 const norm=p=>clamp((p.value-p.min)/Math.max(1,p.max-p.min));
 const parameterMap=parameters=>new Map((parameters||[]).map(p=>[Number(p.index),p]));
 const ready=(p,ids)=>ids.every(id=>{const v=p.get(id);return v&&Number.isFinite(v.value)&&Number.isFinite(v.min)&&Number.isFinite(v.max)&&v.max>=v.min;});
 const display=p=>p.display===undefined?String(p.value):String(p.display);
 function envelope(type,p){
  const mod=type==='Mod-Env',ids=mod?[0,1,2,3,8]:[0,1,2,3,4,5];if(!ready(p,ids))return null;
  const attack=p.get(mod?0:1),decay=p.get(mod?1:2),sustain=p.get(mod?2:3),release=p.get(mod?3:4);
  const a=norm(attack),d=norm(decay),s=norm(sustain),r=norm(release),inverse=p.get(mod?8:5).value!==0;
  const shape=mod?0:p.get(0).value;
  const y=v=>inverse?v:1-v;
  const P=(x,v)=>point(x/4,y(v));
  let path=`M ${P(0,0)}`;
  if(shape===1)path+=` L ${P(a,1)}`;
  else if(shape===2)path+=` C ${P(a,0)} ${P(a,0.25)} ${P(a,1)}`;
  else path+=` C ${P(0,0.25)} ${P(0,1)} ${P(a,1)}`;
  const endDecay=a+d*(1-s),endSustain=4-r*s;
  path+=` C ${P(a,(1-s)*.5+s)} ${P((endDecay+a)*.5,s)} ${P(endDecay,s)}`;
  path+=` L ${P(endSustain,s)} C ${P(endSustain,r*s)} ${P(endSustain,0)} ${P(4,0)}`;
  return {kind:'adsr',title:`${mod?'MOD ENV':'ADSR'} · ${['LOG','LIN','EXP'][shape]||'LOG'}${inverse?' · INV':''}`,
   summary:`A ${display(attack)}   D ${display(decay)}   S ${Math.round(s*100)}%   R ${display(release)}`,
   paths:[{d:path}],markers:[['A',Math.max(.03,a/8)],['D',Math.max(.075,(a+endDecay)/8)],['S',(endDecay+endSustain)/8],['R',(endSustain+4)/8]],
   footer:'BASE ENVELOPE',nodes:[[0,y(0)],[a/4,y(1)],[endDecay/4,y(s)],[endSustain/4,y(s)],[1,y(0)]],inverse};
 }
 function bezier(sx,sy,ex,ey,d1,d2,a1,a2){const r=Math.PI/180;return `${point(sx+d1*Math.cos(a1*r),sy+d1*Math.sin(a1*r))} ${point(ex+d2*Math.cos(a2*r),ey+d2*Math.sin(a2*r))} ${point(ex,ey)}`;}
 function filterPath(mode,cut,reso,slope,gain,classic=false){
  const co=cut*.8+.1,amp=.45,offset=gain===0?1:.5;let path='';
  const M=(x,y)=>{path+=`M ${point(x,y)} `;},L=(x,y)=>{path+=`L ${point(x,y)} `;};
  const C=(...args)=>{path+='C '+bezier(...args)+' ';};
  if(classic){
   const peak=(1-reso)*amp,sl=(2-slope)*.1,x2=co-.2,x4=co+.3+sl;
   M(-.1,amp);L(x2,amp);C(x2,amp,co,peak,.1,.1,0,110+70*peak/amp);C(co,peak,x4,1.1,.1,.1,70-70*peak/amp,-110);return path;
  }
  if(mode==='HP'){
   const x1=co-.5+slope*.25,y1=1.1+amp*.5*reso*gain,peak=amp-reso*amp*offset,x3=co+.1+slope*.15,y3=amp+amp*.5*reso*gain;
   M(x1,y1);C(x1,y1,co,peak,.05,.2+.1*reso,-70+40*slope,180-80*reso);C(co,peak,x3,y3,.05*reso,.05+.05*reso,80*reso,180);L(1.5,y3);
  }else if(mode==='BP'){
   const left=(slope===0?-.65+.12*reso:-.45+.06*reso)+co,right=(slope===0?.65-.12*reso:.45-.06*reso)+co;
   const base=1.1+amp*.5*reso*gain,peak=amp-reso*amp*offset,se=slope===1?.25:.25+.15*reso,a=-50-40*reso-20*slope,b=180-80*reso;
   M(-.1,base);L(left,base);C(left,base,co,peak,.25,se,a,b);C(co,peak,right,base,se,.25,180-b,180-a);L(1.1,base);
  }else if(mode==='BR'){
   const base=amp+amp*(gain===0?.35:0)*reso,left=co-.45+(.15-.1*slope)*reso-slope*.05,right=co+.45-(.15-.1*slope)*reso+slope*.05;
   const lo=slope===0?.3:.2,le=slope===0?.2:.4,depth=slope===0?1:1+amp*.8*(1-reso);
   M(-.5,base);L(left,base);C(left,base,co,depth,lo,le,0,-90);C(co,depth,right,base,le,lo,-90,180);L(1.1,base);
  }else{
   const base=amp+amp*.5*reso*gain,left=co-.1-slope*.15,peak=amp-reso*amp*offset,right=co+.5-slope*.25,tail=1.1+amp*.5*reso*gain;
   M(-.3,base);L(left,base);C(left,base,co,peak,.05+.05*reso,.05*reso,0,180-80*reso);C(co,peak,right,tail,.2+.1*reso,.05,(85-15*slope)*reso,-150+40*slope);
  }
  return path;
 }
 function onePole(mode,cut){let path='';const co=cut*.8+.1;for(let i=0;i<=96;i++){const x=i/96,r=Math.pow(2,(x-co)*10),amplitude=(mode==='HP'?r:1)/Math.sqrt(1+r*r),db=20*Math.log10(Math.max(amplitude,1e-6));path+=(i?' L ':'M ')+point(x,clamp(.45-db/60,-.2,1.2));}return path;}
 function filter(type,p){
  const definitions={FilterA:{cut:0,mode:'LP',db:6},FilterB:{cut:0,mode:'HP',db:6},FilterC:{cut:0,res:1,gain:2,multi:true,db:12},FilterD:{cut:0,res:2,multi:true,db:12},FilterE:{cut:3,res:6,gain:1,slope:7,mode:0,bypass:9},FilterF:{cut:0,res:2,slope:5,bypass:6,classic:true}};
  const def=definitions[type];if(!def)return null;
  const required=[def.cut,...['res','gain','slope','bypass'].filter(k=>def[k]!==undefined).map(k=>def[k])];if(typeof def.mode==='number')required.push(def.mode);
  if(!ready(p,required))return null;
  const cut=norm(p.get(def.cut)),res=def.res===undefined?0:norm(p.get(def.res)),gain=def.gain===undefined?0:p.get(def.gain).value;
  const slope=def.slope===undefined?0:p.get(def.slope).value,db=def.db||(def.classic?[12,18,24][slope]:[12,24][slope]);
  const bypass=def.bypass!==undefined&&p.get(def.bypass).value!==0;
  const mode=typeof def.mode==='string'?def.mode:def.classic?'LP':def.multi?'LP / BP / HP':['LP','BP','HP','BR'][p.get(def.mode).value];
  if(!mode||!Number.isFinite(db))return null;
  const modes=def.multi?['LP','BP','HP']:[mode];
  const paths=bypass?[{d:`M ${point(0,.45)} L ${point(1,.45)}`}]:modes.map((m,i)=>({label:m,d:db===6?onePole(m,cut):filterPath(m,cut,res,slope,gain,def.classic),opacity:i===0?1:.65,dash:i===1?'5 3':i===2?'1 3':null}));
  return {kind:'filter',title:`${mode} · ${db} dB/OCT${bypass?' · BYPASS':''}`,summary:`Cutoff ${display(p.get(def.cut))}${def.res===undefined?'':'   Res '+Math.round(res*100)+'%'}`,paths,
   cutoffX:cut*.8+.1,markers:[['LOW',0],['HIGH',1]],footer:'BASE RESPONSE · SCHEMATIC'};
 }
 // Oscillator enums use fmtOscillatorWave (not the different LFO enum).
 function oscillator(type,p){
  const selectors={OscA:4,OscB:3},fixed={OscC:0,OscSlvB:3,OscSlvC:2,OscSlvD:1,OscSlvE:0};
  let wave=fixed[type];if(selectors[type]!==undefined){if(!ready(p,[selectors[type]]))return null;wave=p.get(selectors[type]).value;}
  if(!Number.isInteger(wave)||wave<0||wave>3)return null;
  const widthIndex=type==='OscA'?3:type==='OscB'?7:type==='OscSlvB'?2:null,width=widthIndex===null?null:p.get(widthIndex);
  // Upstream fmtPulseWidth gives the base duty percentage. No modulation is inferred.
  const duty=width&&ready(p,[widthIndex])?clamp((width.value+1)*.0077165354330708658,.0077,.988):.5;
  let path='';const add=(x,v)=>{path+=(path?' L ':'M ')+point(x,.5-v*.44);};
  if(wave===3){for(let cycle=0;cycle<2;cycle++){const start=cycle/2;add(start,1);add(start+duty/2,1);add(start+duty/2,-1);add((cycle+1)/2,-1);}}
  else if(wave===2){for(let cycle=0;cycle<2;cycle++){add(cycle/2,-1);add((cycle+1)/2,1);}}
  else{for(let i=0;i<=192;i++){const x=i/192,phase=x*2;add(x,wave===0?Math.sin(phase*Math.PI*2):2/Math.PI*Math.asin(Math.sin(phase*Math.PI*2)));}}
  const name=['SINE','TRIANGLE','SAW','PULSE'][wave];
  return {kind:'oscillator',title:name,summary:wave===3&&width?'Width '+display(width):'2 CYCLES',paths:[{d:path}],markers:[['0',0],['1',.5],['2',1]],footer:'BASE WAVEFORM · NO MODULATION',wave,duty};
 }
 function svg(model){
  const grid=[0,.25,.5,.75,1].map(v=>`<path d="M ${point(v,0)} L ${point(v,1)} M ${point(0,v)} L ${point(1,v)}"/>`).join('');
  const fills=model.paths.map(p=>`<path class="module-fill" d="${p.d} L 462,130 L 2,130 Z"/>`).join('');
  const nodes=(model.nodes||[]).map(([x,y])=>`<circle class="module-node" cx="${number(X(x))}" cy="${number(Y(y))}" r="2.5"/>`).join('');
  const curves=model.paths.map(p=>`<path class="module-curve" d="${p.d}" stroke-opacity="${p.opacity||1}"${p.dash?` stroke-dasharray="${p.dash}"`:''}/>`).join('');
  const marker=model.cutoffX===undefined?'':`<path class="module-guide" d="M ${point(model.cutoffX,0)} L ${point(model.cutoffX,1)}"/>`;
  return `<svg class="graph-${model.kind}" viewBox="0 0 464 132" role="img" aria-label="${escape(model.title+' '+model.summary+(model.footer.includes('PENDING')?' · PENDING':''))}" xmlns="http://www.w3.org/2000/svg"><defs><linearGradient id="graph-fill" x1="0" y1="0" x2="0" y2="1"><stop offset="0%" stop-color="currentColor" stop-opacity=".22"/><stop offset="100%" stop-color="currentColor" stop-opacity=".02"/></linearGradient><clipPath id="module-clip"><rect x="2" y="2" width="460" height="128" rx="2"/></clipPath></defs><rect class="module-panel" x="2" y="2" width="460" height="128" rx="2"/><g class="module-grid">${grid}</g><g clip-path="url(#module-clip)">${fills}${marker}${curves}</g>${nodes}</svg>`;
 }
 function render(type,parameters){const p=parameterMap(parameters);const model=type==='ADSR'||type==='Mod-Env'?envelope(type,p):filter(type,p)||oscillator(type,p);if(model&&(parameters||[]).some(p=>p.pending))model.footer+=' · PENDING';return model?{...model,svg:svg(model)}:null;}
 // Pinned NME fmtFilterType / fmtFilterSlope1 / fmtFilterSlope2 enums.
 // Some descriptors omit their formatter even though the registry defines it.
 function parameterDisplay(type,index,value,fallback){
  const choices=type==='FilterE'&&index===0?['LP','BP','HP','BR']:type==='FilterE'&&index===7?['12 dB','24 dB']:type==='FilterF'&&index===5?['12 dB','18 dB','24 dB']:null;
  return choices&&Number.isInteger(value)&&choices[value]!==undefined?choices[value]:fallback;
 }
 return {render,parameterDisplay};
})();
if(typeof module!=='undefined'&&module.exports)module.exports=ModuleVisuals;
