// Built app, isolated browser profile, synthetic recorded solves only.
import {chromium} from 'playwright-core';
import assert from 'node:assert/strict';
import {readFile,writeFile,cp} from 'node:fs/promises';
const source=await readFile(new URL('./browser-interface.mjs',import.meta.url),'utf8');
const fixtureStart=source.indexOf(' await page.evaluate(() => {\n  // Four genuine');
const fixtureEnd=source.indexOf(" await page.waitForFunction(() => document.querySelectorAll('#phase-rows tr').length === 4);",fixtureStart);
const rawFixture=source.slice(fixtureStart,fixtureEnd);
// Persist in the seeding task before any pending startup callback can replace the in-memory session.
const fixture=rawFixture.replace('page.evaluate(() => {','page.evaluate(async () => {').replace(/\n \}\);\s*$/, '\n  await stats.getSessionManager().save();\n });');
const browser=await chromium.launch({executablePath:'/etc/profiles/per-user/zhenyu/bin/brave',headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
try {
 const context=await browser.newContext({viewport:{width:390,height:844}});
 const page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto('http://localhost:3330');
 await page.waitForFunction(()=>window.localInterface&&document.querySelector('#cube-canvas')?.dataset.ready==='true');
 await page.evaluate(()=>document.fonts.ready);
 await page.evaluate(()=>navigator.serviceWorker.ready);
 await page.evaluate(()=>stats.getSessionManager().load());
 await page.evaluate(()=>kernel.setProp('localPalette','carbon'));
 await (new Function('page','return (async()=>{'+fixture+'})()'))(page);
 assert.equal(await page.evaluate(async()=>(await storage.get(kernel.getProp('session'))).length),20);
 await page.reload();
 await page.waitForFunction(()=>window.localInterface&&stats.getTimesStatsList().timesLen===20&&document.querySelector('#cube-canvas')?.dataset.ready==='true');
 await page.evaluate(()=>document.fonts.ready);
 assert.equal(await page.locator('#device-button>span').isVisible(),false);
 assert.ok((await page.locator('#device-button').boundingBox()).width>=44);
 await page.evaluate(()=>window.dispatchEvent(new Event('local-cube-redraw')));await page.waitForTimeout(300);
 assert.deepEqual(errors,[]);
 const path=new URL('../.data/implemented/phone-dark.png',import.meta.url);
 await page.screenshot({path:path.pathname});
 await cp(path,new URL('../design/interface-preview/installed-shots/phone-dark.png',import.meta.url));
 console.log('Captured dark phone view from built app at 390 × 844.');
} finally {await browser.close();}
