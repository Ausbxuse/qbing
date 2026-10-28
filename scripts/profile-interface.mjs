// Synthetic performance probe in a fresh browser profile. No hardware or user history.
import {chromium} from 'playwright-core';
import {readFile,writeFile,mkdir,stat,readdir} from 'node:fs/promises';
const out=new URL('../.data/',import.meta.url);await mkdir(out,{recursive:true});
const browser=await chromium.launch({executablePath:'/etc/profiles/per-user/zhenyu/bin/brave',headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
try{
 const context=await browser.newContext({viewport:{width:1440,height:1000}}),page=await context.newPage();
 await page.addInitScript(()=>{
  const counts=window.__cubeProfile={frames:0,draws:0,triangles:0,lastDraws:0,lastTriangles:0,frameDraws:0,frameTriangles:0};
  const get=HTMLCanvasElement.prototype.getContext,wrapped=new WeakSet();
  HTMLCanvasElement.prototype.getContext=function(...args){const gl=get.apply(this,args);if(this.id!=='cube-canvas'||!gl||wrapped.has(gl)||!gl.drawElements)return gl;wrapped.add(gl);
   const clear=gl.clear.bind(gl);gl.clear=function(...v){if(counts.frameDraws){counts.lastDraws=counts.frameDraws;counts.lastTriangles=counts.frameTriangles;counts.frames++;}counts.frameDraws=counts.frameTriangles=0;return clear(...v);};
   for(const name of ['drawElements','drawArrays','drawElementsInstanced','drawArraysInstanced']){if(!gl[name])continue;const original=gl[name].bind(gl);gl[name]=function(...v){counts.draws++;counts.frameDraws++;const triangles=v[0]===gl.TRIANGLES?(name.includes('Elements')?v[1]:v[2])/3:0;const instances=name.endsWith('Instanced')?v[name==='drawElementsInstanced'?4:3]:1;counts.triangles+=triangles*instances;counts.frameTriangles+=triangles*instances;return original(...v);};}
   return gl;
  };
 });
 const started=Date.now();await page.goto(process.env.CSTIMER_URL||'http://localhost:3330');await page.waitForFunction(()=>window.localInterface&&document.querySelector('#cube-canvas')?.dataset.ready==='true');await page.evaluate(()=>document.fonts.ready);await page.evaluate(()=>navigator.serviceWorker.ready);await page.waitForTimeout(400);
 const readyMs=Date.now()-started;
 const cdp=await context.newCDPSession(page);await cdp.send('Performance.enable');
 const metrics=async()=>Object.fromEntries((await cdp.send('Performance.getMetrics')).metrics.map(v=>[v.name,v.value]));
 const before=await metrics(),idleStart=await page.evaluate(()=>({...window.__cubeProfile}));await page.waitForTimeout(1200);const after=await metrics(),idleEnd=await page.evaluate(()=>({...window.__cubeProfile}));
 const cube=await page.evaluate(async()=>{
  const state=new mathlib.CubieCube(),moves=['R','U',"F'",'D2','L','B2'];let target;
  const frames=[];let active=true,last=performance.now();function sample(now){frames.push(now-last);last=now;if(active)requestAnimationFrame(sample);}requestAnimationFrame(sample);
  const start={...__cubeProfile};const began=performance.now();
  for(let i=0;i<36;i++){const move=moves[i%moves.length];state.selfMoveStr(move);target=state.toFaceCube();window.dispatchEvent(new CustomEvent('local-cube-state',{detail:{facelets:target,moves:[move]}}));await new Promise(r=>setTimeout(r,25));}
  const endAt=performance.now();await new Promise(resolve=>{const check=()=>{const c=document.querySelector('#cube-canvas');if(c.dataset.facelets===target&&c.dataset.animating!=='true')resolve();else setTimeout(check,10);};check();});active=false;
  const end={...__cubeProfile};frames.sort((a,b)=>a-b);
  return {drawCallsPerFrame:end.lastDraws,trianglesPerFrame:end.lastTriangles,draws:end.draws-start.draws,renderFrames:end.frames-start.frames,p95FrameMs:frames[Math.floor(frames.length*.95)],elapsedMs:performance.now()-began,settleMs:performance.now()-endAt,correctState:document.querySelector('#cube-canvas').dataset.facelets===target};
 });
 const build=new URL('../.local-build/',import.meta.url);let bytes=0,files=0;
 for(const name of await readdir(build,{recursive:true})){const info=await stat(new URL(name,build));if(info.isFile()&&!/\.(?:br|gz)$/.test(name)){bytes+=info.size;files++;}}
 const report={readyMs,probeMs:Date.now()-started,cube,uncompressedBuildBytes:bytes,files,heapBytes:after.JSHeapUsedSize,idle:{cubeDraws:idleEnd.draws-idleStart.draws,taskMs:(after.TaskDuration-before.TaskDuration)*1000,layouts:after.LayoutCount-before.LayoutCount},renderer:await page.evaluate(()=>window.localCubeView&&localCubeView.stats?localCubeView.stats():null)};
 const name=process.env.CSTIMER_PROFILE_NAME||'current';await writeFile(new URL('performance-'+name+'.json',out),JSON.stringify(report,null,2));console.log(JSON.stringify(report));
}finally{await browser.close();}
