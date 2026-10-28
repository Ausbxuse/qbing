// Deterministic resource/animation checks; no device-specific FPS promise.
import {chromium} from 'playwright-core';
import assert from 'node:assert/strict';
import {writeFile,mkdir} from 'node:fs/promises';
const browser=await chromium.launch({executablePath:'/etc/profiles/per-user/zhenyu/bin/brave',headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
const root=process.env.CSTIMER_URL||'http://localhost:3330';
try{
 const context=await browser.newContext({viewport:{width:1440,height:1000}}),page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(root);await page.waitForFunction(()=>window.localCubeView&&localCubeView.stats().frames>0&&document.querySelector('#scramble-line').textContent.trim());await page.evaluate(()=>document.fonts.ready);await page.waitForTimeout(300);
 const budget=await page.evaluate(()=>localCubeView.stats());assert.ok(budget.drawCalls<=4);assert.ok(budget.triangles<60000);assert.ok(budget.pixelRatio<=2);
 const idle=await page.evaluate(()=>localCubeView.stats().frames);await page.waitForTimeout(600);assert.equal(await page.evaluate(()=>localCubeView.stats().frames),idle,'Cube does no GPU rendering at rest');
 await page.evaluate(async()=>{
  const c=new mathlib.CubieCube();c.fromFacelet(document.querySelector('#cube-canvas').dataset.facelets);
  for(let i=0;i<30;i++){const move=['R','U',"F'",'D2','L','B'][i%6];c.selfMoveStr(move);window.dispatchEvent(new CustomEvent('local-cube-state',{detail:{facelets:c.toFaceCube(),moves:[move]}}));await new Promise(r=>setTimeout(r,25));}
  window.performanceTarget=c.toFaceCube();
 });
 await page.waitForFunction(()=>document.querySelector('#cube-canvas').dataset.facelets===window.performanceTarget&&!localCubeView.stats().animating);
 assert.equal(await page.evaluate(()=>localCubeView.stats().queue),0);
 await page.locator('.desktop-nav [data-view="history"]').click();await page.waitForFunction(()=>!localCubeView.stats().visible);
 const hidden=await page.evaluate(()=>localCubeView.stats().frames);
 await page.evaluate(()=>{const c=new mathlib.CubieCube();c.fromFacelet(performanceTarget);for(const move of ['U','U','R2',"F'"]){c.selfMoveStr(move);window.dispatchEvent(new CustomEvent('local-cube-state',{detail:{facelets:c.toFaceCube(),moves:[move]}}));}window.performanceTarget=c.toFaceCube();});
 await page.waitForTimeout(150);assert.equal(await page.evaluate(()=>localCubeView.stats().frames),hidden,'Hidden cube does no GPU rendering');assert.equal(await page.evaluate(()=>document.querySelector('#cube-canvas').dataset.facelets),await page.evaluate(()=>performanceTarget));
 await page.locator('.desktop-nav [data-view="timer"]').click();await page.waitForFunction(()=>localCubeView.stats().visible);
 await page.emulateMedia({reducedMotion:'reduce'});const beforeStatic=await page.evaluate(()=>localCubeView.stats().frames);
 await page.evaluate(()=>{const c=new mathlib.CubieCube();c.fromFacelet(performanceTarget);c.selfMoveStr('R');window.performanceTarget=c.toFaceCube();window.dispatchEvent(new CustomEvent('local-cube-state',{detail:{facelets:performanceTarget,moves:['R']}}));});
 assert.equal(await page.evaluate(()=>localCubeView.stats().animating),false);await page.waitForFunction(frames=>localCubeView.stats().frames>frames,beforeStatic);
 await page.emulateMedia({reducedMotion:'no-preference'});
 const supportsLoss=await page.evaluate(()=>{const gl=document.querySelector('#cube-canvas').getContext('webgl2');window.testContextLoss=gl.getExtension('WEBGL_lose_context');if(!testContextLoss)return false;testContextLoss.loseContext();return true;});
 if(supportsLoss){await page.waitForFunction(()=>!localCubeView.stats().visible);await page.evaluate(()=>{const c=new mathlib.CubieCube();c.fromFacelet(performanceTarget);c.selfMoveStr('U2');window.performanceTarget=c.toFaceCube();window.dispatchEvent(new CustomEvent('local-cube-state',{detail:{facelets:performanceTarget,moves:['U2']}}));});await page.evaluate(()=>{window.restoreFrame=localCubeView.stats().frames;testContextLoss.restoreContext();});await page.waitForFunction(()=>localCubeView.stats().visible&&localCubeView.stats().frames>restoreFrame);await page.waitForFunction(()=>document.querySelector('#cube-canvas').dataset.facelets===performanceTarget);}
 const restoredFrame=await page.locator('#cube-canvas').screenshot();
 const restoredColors=await page.evaluate(async base64=>{const image=new Image();image.src='data:image/png;base64,'+base64;await image.decode();const c=document.createElement('canvas');c.width=image.width;c.height=image.height;const ctx=c.getContext('2d');ctx.drawImage(image,0,0);const pixels=ctx.getImageData(0,0,c.width,c.height).data;let count=0;for(let i=0;i<pixels.length;i+=4)if(Math.max(pixels[i],pixels[i+1],pixels[i+2])>100&&Math.max(pixels[i],pixels[i+1],pixels[i+2])-Math.min(pixels[i],pixels[i+1],pixels[i+2])>60)count++;return count;},restoredFrame.toString('base64'));
 assert.ok(restoredColors>100,'Restored context actually draws colored cube faces');
 const batched=await page.evaluate(async()=>{let count=0;const handler=()=>count++;window.addEventListener('local-cube-redraw',handler);for(let i=0;i<20;i++)kernel.pushSignal('time',['','R U',[0,10000+i*10],1700000000+i]);await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));window.removeEventListener('local-cube-redraw',handler);return count;});assert.ok(batched<=3,'Bulk statistics updates coalesce into display frames');
 assert.deepEqual(errors,[]);
 const classic=await context.newPage(),requests=[];classic.on('request',r=>requests.push(r.url()));await classic.goto(root+'/?interface=classic');await classic.waitForFunction(()=>window.localInterface);await classic.waitForTimeout(250);
 assert.equal(await classic.evaluate(()=>!!window.localCubeView),false);assert.ok(requests.every(url=>!url.includes('three.module')&&!url.includes('local-cube-view')),'Classic mode never initializes the 3D renderer');
 const report={budget,batchedUpdates:batched,contextRecovery:supportsLoss,idleGpuFrames:0,hiddenGpuFrames:0,rapidTurnsCorrect:true,classic3DLoaded:false};await mkdir(new URL('../.data/',import.meta.url),{recursive:true});await writeFile(new URL('../.data/performance-checks.json',import.meta.url),JSON.stringify(report,null,2));console.log('Performance checks passed: '+JSON.stringify(report));
}finally{await browser.close();}
