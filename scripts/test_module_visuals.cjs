// Focused parameter-graph checks; no synth, hardware or browser is controlled.
const assert=require('assert'),fs=require('fs'),vm=require('vm');
const graphs=require('../screens/module-visuals.js');
const parameter=(index,value=64,max=127)=>({index,value,min:0,max,display:String(value)});
const params=(count)=>Array.from({length:count},(_,i)=>parameter(i));
const adsr=params(6);adsr[0]=parameter(0,0,2);adsr[5]=parameter(5,0,1);
let a=graphs.render('ADSR',adsr);assert(a&&a.kind==='adsr');assert(a.paths[0].d.startsWith('M 2.00,130.00'),'Non-inverted envelope starts at bottom baseline');
for(const i of [1,2,3,4]){const next=structuredClone(adsr);next[i].value=10;assert.notEqual(graphs.render('ADSR',next).paths[0].d,a.paths[0].d,`ADSR parameter ${i} must affect shape`);}
for(const shape of [1,2]){const next=structuredClone(adsr);next[0].value=shape;assert.notEqual(graphs.render('ADSR',next).paths[0].d,a.paths[0].d);}
const inverted=structuredClone(adsr);inverted[5].value=1;assert(graphs.render('ADSR',inverted).inverse);assert(graphs.render('ADSR',inverted).paths[0].d.startsWith('M 2.00,2.00'));assert.notEqual(graphs.render('ADSR',inverted).paths[0].d,a.paths[0].d);
const mod=params(9);mod[8]=parameter(8,0,1);assert(graphs.render('Mod-Env',mod));mod[8].value=1;assert(graphs.render('Mod-Env',mod).inverse);
const types=['FilterA','FilterB','FilterC','FilterD','FilterE','FilterF'];
const mapping={FilterA:[0],FilterB:[0],FilterC:[0,1],FilterD:[0,2],FilterE:[3,6],FilterF:[0,2]};
const all=[];
for(const type of types){
 const p=params(10);p[0]=parameter(0,type==='FilterE'?0:64,type==='FilterE'?3:127);p[1]=parameter(1,type==='FilterE'?1:64);p[5]=parameter(5,type==='FilterF'?0:64,type==='FilterF'?2:127);p[6]=parameter(6,type==='FilterF'?0:64,type==='FilterF'?1:127);p[7]=parameter(7,type==='FilterE'?1:64,type==='FilterE'?1:127);p[9]=parameter(9,0,1);
 if(type==='FilterC')p[2]=parameter(2,1,1);if(type==='FilterE')p[1]=parameter(1,1,1);
 const baseline=graphs.render(type,p);assert(baseline,type);all.push(baseline);
 for(const index of mapping[type]){const next=structuredClone(p);next[index].value=20;assert.notDeepEqual(graphs.render(type,next).paths,baseline.paths,`${type} parameter${index} must affect curve`);}
 if(type==='FilterC'||type==='FilterD')assert.equal(baseline.paths.length,3);
 if(type==='FilterE')for(let mode=0;mode<4;mode++){const next=structuredClone(p);next[0].value=mode;all.push(graphs.render(type,next));}
 if(type==='FilterF')for(let slope=0;slope<3;slope++){const next=structuredClone(p);next[5].value=slope;all.push(graphs.render(type,next));}
 if(type==='FilterE'||type==='FilterF'){const next=structuredClone(p);next[type==='FilterE'?9:6].value=1;assert(graphs.render(type,next).title.includes('BYPASS'));}
}
for(const model of [a,graphs.render('Mod-Env',mod),...all]){assert(!/NaN|Infinity|undefined/.test(model.svg));assert(model.svg.includes('clipPath'));}
const osc=params(10);osc[4]=parameter(4,0,3);osc[3]=parameter(3,64);const waves=new Set();for(let wave=0;wave<4;wave++){osc[4].value=wave;const model=graphs.render('OscA',osc);assert(model&&model.kind==='oscillator');waves.add(model.paths[0].d);assert(!/NaN|undefined|Infinity/.test(model.svg));}assert.equal(waves.size,4);
osc[3]=parameter(3,3,3);osc[7]=parameter(7,64);assert(graphs.render('OscB',osc));assert(graphs.render('OscC',[]));assert(graphs.render('OscSlvB',params(5)));assert(graphs.render('OscSlvC',[]));assert(graphs.render('OscSlvD',[]));assert(graphs.render('OscSlvE',[]));
const pulse=graphs.render('OscA',osc);osc[3].value=10;assert.notEqual(graphs.render('OscA',osc).paths[0].d,pulse.paths[0].d);
assert.equal(graphs.render('OscA',[]),null);assert.equal(graphs.render('ADSR',[]),null);
const html=fs.readFileSync('screens/g1.html','utf8'),signature=html.slice(html.indexOf('function visualSignature('),html.indexOf('\nwindow.encdr=',html.indexOf('function visualSignature(')));
const state={moduleEdit:true,moduleReady:true,moduleType:'Mod-Env',moduleParameters:structuredClone(mod),labels:Array(8).fill('UNASSIGNED'),displayValues:Array(8).fill('—'),page:1,pageCount:2,engine:'G1-EMU / LIVE'};
const changed=structuredClone(state);changed.moduleParameters[0].value=2;
for(const SIDE of ['left','right']){const before=vm.runInNewContext(signature+';visualSignature(state)',{SIDE,LEFT:SIDE==='left',START:SIDE==='left'?0:4,state});const after=vm.runInNewContext(signature+';visualSignature(state)',{SIDE,LEFT:SIDE==='left',START:SIDE==='left'?0:4,state:changed});assert.equal(before===after,SIDE==='left','Off-page diagram values must repaint physical right screen only');}
console.log('PASS oscillator waveform selection/pulse width and fixed slaves; ADSR/Mod-Env stages, shape/invert; Filter A–F modes/slopes/bypass; off-page right-screen invalidation; unsupported/missing data');

for(const [type,index,labels] of [['FilterE',0,['LP','BP','HP','BR']],['FilterE',7,['12 dB','24 dB']],['FilterF',5,['12 dB','18 dB','24 dB']]])for(const [value,label] of labels.entries())assert.equal(graphs.parameterDisplay(type,index,value,String(value)),label);
assert.equal(graphs.parameterDisplay('ADSR',0,1,'LIN'),'LIN');assert(!graphs.render('ADSR',adsr).svg.includes('<text'));
