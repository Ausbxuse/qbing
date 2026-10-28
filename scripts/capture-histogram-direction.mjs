// Isolated visual study. Never attaches to a user profile or changes app assets.
import {chromium} from 'playwright-core';
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
const directory=new URL('../design/interface-preview/histogram-direction/',import.meta.url);
await mkdir(directory,{recursive:true});
const source=await readFile(new URL('./browser-interface.mjs',import.meta.url),'utf8');
const start=source.indexOf(' await page.evaluate(() => {\n  // Four genuine');
const end=source.indexOf(" await page.waitForFunction(() => document.querySelectorAll('#phase-rows tr').length === 4);",start);
assert.ok(start>=0&&end>start);
const times=[10500,11220,11540,11720,11930,12080,12120,12250,12310,12400,12480,12520,12580,12600,12710,12880,13100,13420,14180,15360];
const rawFixture=source.slice(start,end).replace('10500 + i * 170',JSON.stringify(times)+'[i]');
// Persist in the seeding task before any pending startup callback can replace the in-memory session.
const fixture=rawFixture.replace('page.evaluate(() => {','page.evaluate(async () => {').replace(/\n \}\);\s*$/, '\n  await stats.getSessionManager().save();\n });');
const css=`
.distribution-bin:nth-child(odd) { --bin-color:#7895b2; }
.distribution-bin:nth-child(even) { --bin-color:#8ca8c2; }
:root[data-palette=paper] .distribution-bin:nth-child(odd) { --bin-color:#567a9e; }
:root[data-palette=paper] .distribution-bin:nth-child(even) { --bin-color:#7092b3; }
.distribution-bar { background:var(--bin-color); }
.distribution-bin:hover .distribution-bar,.distribution-bin:focus .distribution-bar { background:color-mix(in srgb,var(--bin-color) 85%,var(--fg)); }
`;
await writeFile(new URL('study.css',directory),css);
const browser=await chromium.launch({executablePath:'/etc/profiles/per-user/zhenyu/bin/brave',headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
try {
 const context=await browser.newContext({viewport:{width:1440,height:1000}});
 const page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto('http://localhost:3330');
 await page.waitForFunction(()=>window.localInterface&&document.querySelector('#cube-canvas')?.dataset.ready==='true');
 await page.evaluate(()=>navigator.serviceWorker.ready);
 await page.evaluate(()=>stats.getSessionManager().load());
 await (new Function('page','return (async()=>{'+fixture+'})()'))(page);
 assert.equal(await page.evaluate(async()=>(await storage.get(kernel.getProp('session'))).length),20);
 await page.reload();
 await page.waitForFunction(()=>window.localInterface&&stats.getTimesStatsList().timesLen===20);
 await page.evaluate(()=>document.fonts.ready);
 await page.addStyleTag({content:css});
 await page.locator('.desktop-nav [data-view="history"]').click();
 for(const [name,width,height,mode] of [['dark',1440,1000,'carbon'],['light',1440,1000,'paper'],['phone',390,844,'carbon']]) {
  await page.setViewportSize({width,height});
  await page.evaluate(mode=>kernel.setProp('localPalette',mode),mode);
  await page.waitForFunction(()=>document.querySelector('#session-button').parentElement.classList.contains(innerWidth<=760?'context-left':'history-heading'));
  await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
  assert.equal(await page.locator('#history-rows tr[data-index]').count(),20);
  assert.equal(await page.locator('.distribution-bin').evaluateAll(nodes=>nodes.reduce((sum,n)=>sum+Number(n.dataset.count),0)),20);
  const bars=await page.locator('.distribution-bar').evaluateAll(nodes=>nodes.map(n=>{const r=n.getBoundingClientRect();return {left:r.left,right:r.right,color:getComputedStyle(n).backgroundColor};}));
  assert.equal(new Set(bars.map(b=>b.color)).size,2);
  for(let i=1;i<bars.length;i++) assert.ok(Math.abs(bars[i].left-bars[i-1].right)<1);
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  await page.screenshot({path:new URL(name+'.png',directory).pathname});
  if(name==='dark') await page.locator('.history-analysis').screenshot({path:new URL('detail.png',directory).pathname});
 }
 assert.deepEqual(errors,[]);
 console.log('Captured cool blue histogram study in dark, light and phone layouts with verified sample solves.');
} finally {await browser.close();}
