import {chromium} from 'playwright-core';
import {mkdir,writeFile} from 'node:fs/promises';
const out=new URL('../.data/native-parity/',import.meta.url).pathname;await mkdir(out,{recursive:true});
const browser=await chromium.launch({executablePath:'/etc/profiles/per-user/zhenyu/bin/brave',headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
const contract={source:'Approved running csTimer interface. CSS px = native logical px.',views:[]};
try {
 const page=await browser.newPage();await page.goto(process.env.CSTIMER_URL||'http://localhost:3330');
 await page.waitForFunction(()=>window.localInterface&&document.querySelector('#cube-canvas')?.dataset.ready==='true');
 const records=await page.evaluate(()=>{
  const solution="R2 F2 D2 L2 B2 R U R' U' F R U R' U' F' R U' R U R U R U' R' U' R2".split(' '),scramble=solution.slice().reverse().map(m=>m.endsWith('2')?m:m.endsWith("'")?m.slice(0,-1):m+"'").join(' '),records=[];
  const initial=new mathlib.CubieCube();scramble.split(' ').forEach(m=>initial.selfMoveStr(m));
  for(let i=0;i<20;i++){
   const duration=10500+i*170,moves=solution.map((move,j)=>({move,t:Math.round(j*duration/(solution.length-1)),serial:j,raw:{cubeTimestamp:Math.round(j*duration/(solution.length-1)),receivedAt:Math.round(j*duration/(solution.length-1))}}));
   kernel.pushSignal('time',['',scramble,[0,duration],1700000000+i*30,[moves.map(m=>m.move+'@'+m.t).join(' '),'333']]);
   records.push({id:'synthetic-'+i,createdAt:new Date((1700000000+i*30)*1000).toISOString(),kind:'solve',source:'bluetooth',status:'complete',penalty:'',duration,initialState:initial.toFaceCube(),cross:'U',session:'Session 1',stage:'solve',scramble,moves,quality:[]});
  }
  return records;
 });
 await writeFile(out+'fixture.json',JSON.stringify({records,preferences:{},stats:[],synthetic:true},null,2));
 const selectors=['.app-header','.wordmark','.context-left','.desktop-nav','.header-actions','.workspace','.timer-pane','.scramble-region','.scramble-line','.cube-stage','.clock-anchor','#clock','.solve-metrics','.result-actions','.average-table','.phase-section','.history-pane','.history-heading','.history-summary','.session-pace','.history-scroll','.history-table','.history-footer','.analysis-pane','#review-tools','.mobile-nav'];
 for(const [name,width,height,palette,view] of [['desktop-light',1440,1000,'paper','timer'],['desktop-dark',1440,1000,'carbon','timer'],['laptop',1280,800,'carbon','timer'],['phone-light',390,844,'paper','timer'],['phone-dark',390,844,'carbon','timer'],['history-light',1440,1000,'paper','history'],['history-phone',390,844,'carbon','history']]){
  await page.setViewportSize({width,height});await page.evaluate(({palette,view})=>{kernel.setProp('localPalette',palette);document.querySelector(`.${innerWidth<=760?'mobile-nav':'desktop-nav'} [data-view="${view}"]`).click();},{palette,view});
  await page.evaluate(()=>document.fonts.ready);await page.waitForTimeout(180);
  const components=await page.evaluate(selectors=>Object.fromEntries(selectors.map(s=>{const e=document.querySelector(s);if(!e)return[s,null];const r=e.getBoundingClientRect(),c=getComputedStyle(e);return[s,{x:r.x,y:r.y,width:r.width,height:r.height,fontFamily:c.fontFamily,fontSize:c.fontSize,fontWeight:c.fontWeight,lineHeight:c.lineHeight,color:c.color,background:c.backgroundColor,gap:c.gap,padding:c.padding,display:c.display}]})),selectors);
  const colors=await page.evaluate(()=>Object.fromEntries(['bg','panel','surface','fg','muted','faint','accent','secondary','line','edge','good','error','phase-cross','phase-f2l','phase-oll','phase-pll'].map(k=>[k,getComputedStyle(document.documentElement).getPropertyValue('--'+k).trim()])));
  contract.views.push({name,width,height,palette,view,colors,components});await page.screenshot({path:out+'reference-'+name+'.png'});
 }
 await writeFile(new URL('../native/assets/design-contract.json',import.meta.url),JSON.stringify(contract,null,2));
 console.log('Reference screenshots, measured design contract and isolated synthetic fixture: '+out);
}finally{await browser.close();}
