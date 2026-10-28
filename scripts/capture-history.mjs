// Built app, isolated browser profile, synthetic recorded solves only.
import {chromium} from 'playwright-core';
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
const source=await readFile(new URL('./browser-interface.mjs',import.meta.url),'utf8');
const fixtureStart=source.indexOf(' await page.evaluate(() => {\n  // Four genuine');
const fixtureEnd=source.indexOf(" await page.waitForFunction(() => document.querySelectorAll('#phase-rows tr').length === 4);",fixtureStart);
const fixtureTimes=[10500,11220,11540,11720,11930,12080,12120,12250,12310,12400,12480,12520,12580,12600,12710,12880,13100,13420,14180,15360];
const rawFixture=source.slice(fixtureStart,fixtureEnd).replace('10500 + i * 170',JSON.stringify(fixtureTimes)+'[i]').replace("kernel.pushSignal('time', ['', inverse,", "kernel.pushSignal('time', [i===4?'Cross pause':i===15?'Try alternate PLL':'', inverse,");
// Persist in the seeding task before any pending startup callback can replace the in-memory session.
const fixture=rawFixture.replace('page.evaluate(() => {','page.evaluate(async () => {').replace(/\n \}\);\s*$/, '\n  await stats.setReviewTags(15,{favorite:true,color:"blue",group:"practice"});\n  await stats.setReviewTags(4,{favorite:false,color:"mint",group:"review"});\n  await stats.getSessionManager().save();\n });');
const captures=[];
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
 await page.evaluate(()=>window.dispatchEvent(new Event('local-cube-redraw')));await page.waitForTimeout(300);
 assert.deepEqual(errors,[]);
 await page.locator('.mobile-nav [data-view="history"]').click();
 await page.waitForFunction(()=>document.body.dataset.view==='history');
 assert.equal(await page.locator('.desktop-nav [data-view="history"]').getAttribute('aria-current'),'page');
 assert.deepEqual(await page.locator('.history-summary-label').allTextContents(),['Best','Mean','ao5','ao12']);
 await page.waitForTimeout(200);
 for(const [name,width,height,mode] of [['history-dark-phone',390,844,'carbon'],['history-desktop',1440,1000,'paper'],['history-dark-desktop',1440,1000,'carbon'],['history-small-phone',320,780,'carbon']]) {
  await page.setViewportSize({width,height});await page.evaluate(mode=>kernel.setProp('localPalette',mode),mode);
  await page.waitForFunction(()=>document.querySelector('#session-button').parentElement.classList.contains(innerWidth<=760?'context-left':'history-heading'));
  await page.waitForTimeout(150);
  assert.equal(await page.evaluate(()=>stats.getTimesStatsList().timesLen),20,'Preview requires 20 persisted sample solves');
  assert.equal(await page.locator('#history-rows tr[data-index]').count(),20);
  assert.equal(await page.locator('.distribution-bin').evaluateAll(nodes=>nodes.reduce((sum,n)=>sum+Number(n.dataset.count),0)),20);
  const chart=await page.locator('.history-distribution').boundingBox();
  const tools=await page.locator('.history-tools').boundingBox();
  const insights=await page.locator('.history-insights').boundingBox();
  const pace=await page.locator('.session-pace').boundingBox();
  if(width>760) {
   assert.ok(chart.width<(tools.width*0.8));
   assert.ok(Math.abs(insights.x-pace.x)<1,'Insights align with recent-times column');
   assert.ok(chart.x+chart.width<insights.x,'Plot and insights have separate columns');
  } else assert.ok(insights.y>=chart.y+chart.height,'Phone insights sit below plot');
  assert.ok(insights.y+insights.height<=tools.y,'Insights clear search and filters');
  const expected=await page.evaluate(()=>[12440,1000,13420].map(ms=>{const el=document.createElement('span');el.innerHTML=kernel.pretty(Math.round(ms));return el.textContent;}));
  assert.deepEqual(await page.locator('.history-insights dd').allTextContents(),[...expected,'0%']);

  for(const tick of await page.locator('.distribution-tick').all()) {
   const label=await tick.boundingBox();
   assert.ok(label.y+label.height<=chart.y+chart.height);
   assert.ok(label.y+label.height<tools.y);
  }
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  assert.equal(await page.locator('#device-button>span').isVisible(),width>760);
  const uiFont=await page.locator('.history-heading h2').evaluate(el=>getComputedStyle(el).fontFamily);
  assert.match(uiFont,/csLocalGeist/);
  assert.match(await page.locator('.history-summary-value').first().evaluate(el=>getComputedStyle(el).fontFamily),/Preview Maple/);

  const bars=await page.locator('.distribution-bar').evaluateAll(nodes=>nodes.map(n=>{const r=n.getBoundingClientRect();return {left:r.left,right:r.right,color:getComputedStyle(n).backgroundColor};}));
  for(let i=1;i<bars.length;i++) {
   assert.ok(Math.abs(bars[i].left-bars[i-1].right)<1);
   assert.notEqual(bars[i].color,bars[i-1].color);
  }


  captures.push({name,buffer:await page.screenshot()});
 }
 await page.setViewportSize({width:1440,height:1000});
 await page.waitForFunction(()=>document.querySelector('#session-button').parentElement.classList.contains('history-heading'));
 await page.locator('[data-history-filter="favorites"]').click();
 await page.locator('#history-query').fill('practice');
 await page.waitForFunction(()=>document.querySelectorAll('#history-rows tr[data-index]').length===1);
 assert.equal(await page.locator('#history-rows tr[data-index]').getAttribute('data-index'),'15');
 assert.equal(await page.locator('.distribution-bin').evaluateAll(nodes=>nodes.reduce((sum,n)=>sum+Number(n.dataset.count),0)),1);
 captures.push({name:'history-filtered',buffer:await page.screenshot()});
 // Publish only after every sample count and layout assertion passes.
 for(const {name,buffer} of captures) {
  await writeFile(new URL('../.data/implemented/'+name+'.png',import.meta.url),buffer);
  await writeFile(new URL('../design/interface-preview/installed-shots/'+name+'.png',import.meta.url),buffer);
 }
 const review=new URL('../design/interface-preview/history-review.html',import.meta.url);
 const html=await readFile(review,'utf8');
 await writeFile(review,html.replace(/(installed-shots\/history-[a-z-]+\.png)(?:\?v=\d+)?/g,'$1?v='+Date.now()));
 console.log('Published all five history previews after verifying persisted sample solves and filtered counts.');
} finally {await browser.close();}
