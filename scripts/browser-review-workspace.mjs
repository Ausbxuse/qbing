// Isolated evidence for review tools and scorebook metadata, never user data.
import {chromium} from 'playwright-core';
import assert from 'node:assert/strict';
import {readFile,mkdir} from 'node:fs/promises';
const source=await readFile(new URL('./browser-interface.mjs',import.meta.url),'utf8');
const start=source.indexOf(' await page.evaluate(() => {\n  // Four genuine');
const end=source.indexOf(" await page.waitForFunction(() => document.querySelectorAll('#phase-rows tr').length === 4);",start);
assert.ok(start>=0&&end>start);
const fixture=source.slice(start,end).replace('page.evaluate(() => {','page.evaluate(async () => {').replace(/\n \}\);\s*$/, '\n  await stats.getSessionManager().save();\n });');
const out=new URL('../.data/implemented/',import.meta.url).pathname;await mkdir(out,{recursive:true});
const browser=await chromium.launch({executablePath:'/etc/profiles/per-user/zhenyu/bin/brave',headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
try {
 const context=await browser.newContext({viewport:{width:1440,height:1000}}),page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(process.env.CSTIMER_URL||'http://localhost:3330');await page.waitForFunction(()=>window.localInterface&&document.querySelector('#cube-canvas')?.dataset.ready==='true');
 await page.evaluate(()=>navigator.serviceWorker.ready);await page.evaluate(()=>stats.getSessionManager().load());
 await (new Function('page','return (async()=>{'+fixture+'})()'))(page);
 await page.reload();await page.waitForFunction(()=>window.localInterface&&stats.getTimesStatsList().timesLen===20);await page.evaluate(()=>document.fonts.ready);
 await page.evaluate(()=>kernel.setProp('localPalette','carbon'));
 await page.evaluate(async()=>{
  const rec=stats.getExtraInfo('recons_cf4op',19), pll="R U U' R' R U R' U' R' F R2 U' R' U' R U R' F'".split(' ');
  const solution=rec.data.slice(1).reverse().flatMap(p=>p[5].map(m=>m[0].trim())).concat(pll);
  const inverse=solution.slice().reverse().map(m=>m.endsWith('2')?m:m.endsWith("'")?m.slice(0,-1):m+"'").join(' ');
  const duration=500+(solution.length-1)*200, timed=solution.map((m,j)=>m+'@'+(500+j*200)).join(' ');
  kernel.pushSignal('time',['Legacy note',inverse,[0,duration],1700001000,[timed,'333']]);await stats.getSessionManager().save();
 });
 await page.waitForFunction(()=>document.querySelectorAll('#phase-rows tr[data-phase]').length===4&&document.querySelector('#review-tool-solve').textContent==='#21');
 assert.match(await page.locator('#pace-caption').textContent(),/Mean/);assert.equal(await page.locator('.pace-mean').count(),1);
 const mean=await page.evaluate(()=>Array.from({length:stats.getTimesStatsList().timesLen},(_,i)=>stats.timeAt(i)).filter(t=>t>=0).reduce((s,t)=>s+t,0)/stats.getTimesStatsList().timesLen);
 const meanY=await page.locator('.pace-mean').getAttribute('y1');assert.ok(Number(meanY)>=10&&Number(meanY)<=52);assert.ok(mean>0);
 const pairs=await page.locator('.phase-time-pair').evaluateAll(nodes=>nodes.map(n=>({pause:n.children[0].textContent,execution:n.children[2].textContent,description:n.getAttribute('aria-label')})));
 assert.equal(pairs.length,4);assert.ok(pairs.every(p=>p.description.includes('Pause')&&p.description.includes('execution')));
 await page.locator('[data-review-tool="cross"]').click();
 const scrambleBefore=await page.evaluate(()=>tools.getCurScramble()[1]);
 await page.getByRole('button',{name:'Find cross routes',exact:true}).click();
 await page.waitForFunction(()=>document.querySelectorAll('.tool-route').length===6);
 assert.equal(await page.evaluate(()=>tools.getCurScramble()[1]),scrambleBefore,'Review must not replace the active scramble');
 const routes=await page.locator('.tool-route .tool-alg').allTextContents();
 const crossValid=await page.evaluate(routes=>routes.map(alg=>{
  const cube=new mathlib.CubieCube();LocalReviewTools.tokens(stats.timesAt(20)[1]).concat(LocalReviewTools.tokens(alg==='Already solved'?'':alg)).forEach(m=>cube.selfMoveStr(m));
  return cubeutil.getStepProgress('cross',cube.toFaceCube())===0;
 }),routes);assert.ok(crossValid.every(Boolean),'Every upstream route solves a cross');
 await page.screenshot({path:out+'review-cross.png'});
 await page.getByLabel('Cross route type').selectOption('xcross');
 await page.getByRole('button',{name:'Find X-cross routes',exact:true}).click();
 await page.waitForFunction(()=>document.querySelectorAll('.tool-route').length===1);
 const xValid=await page.evaluate(()=>{
  const c=new mathlib.CubieCube();LocalReviewTools.tokens(stats.timesAt(20)[1]).concat(LocalReviewTools.tokens(document.querySelector('.tool-alg').textContent)).forEach(m=>c.selfMoveStr(m));
  return cubeutil.getProgress(c,'cf4op')<=5;
 });assert.equal(xValid,true,'Upstream X-cross route solves cross plus a pair');
 await page.locator('[data-review-tool="algorithms"]').click();
 await page.waitForFunction(()=>document.querySelector('#review-tool-content').textContent.includes('Variants to test'));
 assert.match(await page.locator('.tool-case-title').textContent(),/T perm/);
 const variants=await page.locator('.tool-route .tool-alg').allTextContents();assert.ok(variants.length>0);
 const verified=await page.evaluate(variants=>variants.map(alg=>{
  const rec=stats.getExtraInfo('recons_cf4op',20),c=new mathlib.CubieCube();c.invFrom(rec.data[0][4]);LocalReviewTools.tokens(alg).forEach(m=>c.selfMoveStr(m));return c.ca.every((v,i)=>v===i)&&c.ea.every((v,i)=>v===i*2);
 }),variants);assert.ok(verified.every(Boolean),'Every suggested PLL actually solves the reconstructed case');
 await page.screenshot({path:out+'review-algorithms.png'});
 await page.locator('[data-review-tool="technique"]').click();assert.match(await page.locator('#review-tool-content').textContent(),/Largest move gap/);await page.screenshot({path:out+'review-technique.png'});
 await page.locator('#favorite-button').click();await page.waitForFunction(()=>stats.timesAt(20)[5]?.localReview?.favorite);
 await page.locator('#tag-button').click();await page.locator('[data-tag-color="blue"]').click();await page.waitForFunction(()=>stats.timesAt(20)[5]?.localReview?.color==='blue');
 await page.locator('#tag-button').click();await page.locator('[data-tag-group="review"]').click();await page.waitForFunction(()=>stats.timesAt(20)[5]?.localReview?.group==='review');
 assert.equal(await page.evaluate(()=>stats.timesAt(20)[2]),'Legacy note');
 await page.evaluate(()=>stats.setPenalty(20,2000));await page.waitForFunction(()=>stats.timesAt(20)[0][0]===2000);
 assert.deepEqual(await page.evaluate(()=>LocalSolveTags.read(stats.timesAt(20))),{favorite:true,color:'blue',group:'review'});
 const exported=await page.evaluate(()=>storage.exportAll());assert.equal(exported['session1'][20][5].localReview.favorite,true);
 await page.reload();await page.waitForFunction(()=>window.localInterface&&stats.getTimesStatsList().timesLen===21);
 assert.deepEqual(await page.evaluate(()=>LocalSolveTags.read(stats.timesAt(20))),{favorite:true,color:'blue',group:'review'});
 await page.locator('.desktop-nav [data-view="history"]').click();
 assert.equal(await page.locator('.history-summary-metrics b').count(),8);
 await page.locator('[data-history-filter="favorites"]').click();assert.equal(await page.locator('#history-rows tr[data-index]').count(),1);
 await page.locator('#history-rows tr[data-index="20"] .history-favorite').focus();await page.keyboard.press('Space');
 await page.waitForFunction(()=>!stats.timesAt(20)[5].localReview.favorite&&document.activeElement.dataset.historyFilter==='favorites');
 assert.equal(await page.evaluate(()=>timer.status()),-1);
 await page.evaluate(()=>stats.setReviewTags(20,Object.assign(LocalSolveTags.read(stats.timesAt(20)),{favorite:true})));

 await page.getByLabel('Filter color or group').selectOption('group:review');assert.equal(await page.locator('#history-rows tr[data-index]').count(),1);
 await page.getByLabel('Filter color or group').selectOption('color:mint');assert.equal(await page.locator('#history-rows tr[data-index]').count(),0);
 await page.getByLabel('Filter color or group').selectOption('all');await page.locator('[data-history-filter="all"]').click();
 await page.locator('#history-rows tr[data-index="20"] .history-favorite').focus();await page.keyboard.press('Space');
 await page.waitForFunction(()=>!stats.timesAt(20)[5].localReview.favorite&&document.activeElement.classList.contains('history-favorite'));
 assert.equal(await page.evaluate(()=>timer.status()),-1);
 await page.keyboard.press('Space');await page.waitForFunction(()=>stats.timesAt(20)[5].localReview.favorite);
 await page.locator('#history-rows tr[data-index="20"] .history-tags-edit').click();assert.equal(await page.locator('#local-tag-menu').isVisible(),true);await page.screenshot({path:out+'review-tags.png'});await page.keyboard.press('Escape');
 await page.setViewportSize({width:390,height:844});await page.locator('.mobile-nav [data-view="timer"]').click();await page.locator('#review-tools-toggle').click();assert.equal(await page.locator('#review-tools').isVisible(),true);await page.screenshot({path:out+'review-tools-phone.png'});await page.locator('#review-tools-close').click();
 await page.locator('#tag-button').click();
 await page.locator('#session-button').selectOption('2');await page.waitForFunction(()=>stats.getTimesStatsList().timesLen===0);
 assert.equal(await page.locator('#local-tag-menu').isVisible(),false,'Changing sessions closes record-tag actions');
 await page.locator('#session-button').selectOption('1');await page.waitForFunction(()=>stats.getTimesStatsList().timesLen===21);
 // OLL alternatives must preserve F2L and orient the entire last layer.
 await page.evaluate(async()=>{
  const rec=stats.getExtraInfo('recons_cf4op',19);
  const solution=rec.data.slice(2).reverse().flatMap(p=>p[5].map(m=>m[0].trim())).concat("R U R' U R U2 R' R U R' U' R' F R2 U' R' U' R U R' F'".split(' '));
  const inverse=solution.slice().reverse().map(m=>m.endsWith('2')?m:m.endsWith("'")?m.slice(0,-1):m+"'").join(' ');
  kernel.pushSignal('time',['',inverse,[0,500+(solution.length-1)*200],1700002000,[solution.map((m,j)=>m+'@'+(500+j*200)).join(' '),'333']]);await stats.getSessionManager().save();
 });
 await page.setViewportSize({width:1440,height:1000});
 await page.locator('[data-review-tool="algorithms"]').click();await page.getByRole('button',{name:'OLL',exact:true}).click();
 await page.waitForFunction(()=>document.querySelector('#review-tool-content').textContent.includes('Variants to test'));
 const oll=await page.locator('.tool-route .tool-alg').allTextContents();assert.ok(oll.length>0);
 assert.ok((await page.evaluate(variants=>variants.map(alg=>{
  const rec=stats.getExtraInfo('recons_cf4op',21),c=new mathlib.CubieCube();c.invFrom(rec.data[1][4]);LocalReviewTools.tokens(alg).forEach(m=>c.selfMoveStr(m));return cubeutil.getProgress(c,'cfop')<=1;
 }),oll)).every(Boolean));
 await page.screenshot({path:out+'review-oll.png'});
 // Metadata must not destroy unrelated extension fields or accept bad labels.
 await page.evaluate(async()=>{stats.timesAt(20)[5].other='preserved';await stats.setReviewTags(20,LocalSolveTags.read(stats.timesAt(20)));});
 assert.equal(await page.evaluate(()=>stats.timesAt(20)[5].other),'preserved');
 assert.equal(await page.evaluate(async()=>{try{await stats.setReviewTags(20,{favorite:true,color:'invalid',group:''});return false;}catch(_){return true;}}),true);
 await context.setOffline(true);await page.reload();
 await page.waitForFunction(()=>window.localInterface&&stats.getTimesStatsList().timesLen===22);
 await page.getByRole('button',{name:'Find cross routes',exact:true}).click();await page.waitForFunction(()=>document.querySelectorAll('.tool-route').length===6);
 await page.locator('[data-review-tool="algorithms"]').click();await page.getByRole('button',{name:'OLL',exact:true}).click();
 await page.waitForFunction(()=>document.querySelector('#review-tool-content').textContent.includes('Variants to test'));
 await context.setOffline(false);
 assert.deepEqual(errors,[]);console.log('Review workspace passed: cross/X-cross goals, verified OLL/PLL variants, offline tools, metadata export/persistence, filters, CFOP pairs and mobile tools.');
}finally{await browser.close();}
