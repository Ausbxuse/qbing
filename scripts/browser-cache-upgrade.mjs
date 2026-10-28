// Simulate an older cache-first worker in an isolated browser context.
import {chromium} from 'playwright-core';
import assert from 'node:assert/strict';
import {writeFile,rm} from 'node:fs/promises';
const base=new URL('../.local-build/',import.meta.url),worker=new URL('__upgrade-worker.js',base),boot=new URL('__upgrade-boot.html',base);
await writeFile(boot,'<!doctype html><title>Isolated cache upgrade check</title>');
await writeFile(worker,`
self.addEventListener('install',e=>e.waitUntil(caches.open('cstimer-upgrade-test').then(async cache=>{
 await cache.put('/js/local-interface.js',new Response('window.staleInterfaceLoaded=true;',{headers:{'Content-Type':'text/javascript'}}));
 await cache.put('/js/local-cube-geometry.js',new Response('throw new Error("Stale geometry loaded");',{headers:{'Content-Type':'text/javascript'}}));
 await self.skipWaiting();
})));
self.addEventListener('activate',e=>e.waitUntil(self.clients.claim()));
self.addEventListener('fetch',e=>e.respondWith(e.request.mode==='navigate'?fetch(e.request):caches.match(e.request).then(hit=>hit||fetch(e.request))));
`);
const browser=await chromium.launch({executablePath:'/etc/profiles/per-user/zhenyu/bin/brave',headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
try{
 const context=await browser.newContext(),page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
 const root=process.env.CSTIMER_URL||'http://localhost:3330';await page.goto(root+'/__upgrade-boot.html');
 await page.evaluate(async()=>{await navigator.serviceWorker.register('/__upgrade-worker.js',{scope:'/'});await navigator.serviceWorker.ready;});await page.waitForFunction(()=>!!navigator.serviceWorker.controller);
 assert.match(await page.evaluate(async()=>await(await fetch('/js/local-interface.js')).text()),/staleInterfaceLoaded/);
 // Keep the old worker active for this load, rather than letting the new
 // worker win the race and mask a mixed-version startup bug.
 await page.addInitScript(()=>Object.defineProperty(navigator.serviceWorker,'register',{value:async()=>navigator.serviceWorker.getRegistration()}));
 await page.goto(root);await page.waitForFunction(()=>window.localInterface&&window.localCubeView&&localCubeView.stats().frames>0);
 assert.equal(await page.evaluate(()=>!!window.staleInterfaceLoaded),false);
 assert.ok(await page.locator('#local-cube-module').getAttribute('src').then(src=>src.includes('?build=')));
 assert.ok((await page.evaluate(()=>localCubeView.stats().triangles))<60000);assert.deepEqual(errors,[]);
 console.log('Cache upgrade passed: one navigation loads current interface and renderer through a stale cache-first worker.');
}finally{await browser.close();await rm(worker,{force:true});await rm(boot,{force:true});}
