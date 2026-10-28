import { chromium } from 'playwright-core';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
const url = process.env.CSTIMER_URL || 'http://localhost:3327';
const out = fileURLToPath(new URL('../.data/implemented/', import.meta.url));
await mkdir(out, { recursive: true });
const browser = await chromium.launch({ executablePath: process.env.CSTIMER_BROWSER || '/etc/profiles/per-user/zhenyu/bin/brave', headless: true, args: ['--no-sandbox', '--enable-experimental-web-platform-features', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
const checks = [], errors = [];
async function assertVisibleCube(page) {
 const clip = await page.locator('#cube-canvas').boundingBox();
 const frame = await page.screenshot();
 const colored = await page.evaluate(async ({base64,clip}) => {
  const image = new Image(); image.src = 'data:image/png;base64,' + base64; await image.decode();
  const canvas = document.createElement('canvas'); canvas.width = Math.round(clip.width); canvas.height = Math.round(clip.height);
  const context = canvas.getContext('2d'); context.drawImage(image, -clip.x, -clip.y);
  const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
  let count = 0;
  for (let i = 0; i < pixels.length; i += 4) {
   const high = Math.max(pixels[i], pixels[i+1], pixels[i+2]), low = Math.min(pixels[i], pixels[i+1], pixels[i+2]);
   if (high > 100 && high - low > 60) count++;
  }
  return count;
 }, {base64:frame.toString('base64'),clip});
 assert.ok(colored > 100, 'Composited cube must retain visible colored faces, found ' + colored + ' pixels');
 return frame;
}

try {
 const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
 const page = await context.newPage();
 page.on('pageerror', e => errors.push(e.message));
 let promptAnswer = null;
 page.on('dialog', dialog => promptAnswer != null ? dialog.accept(promptAnswer) : dialog.dismiss());
 await page.goto(url);
 await page.waitForFunction(() => window.localInterface && document.querySelector('#cube-canvas')?.dataset.ready === 'true');
 await page.evaluate(() => document.fonts.ready);
 await page.evaluate(() => navigator.serviceWorker.ready);
 assert.equal(await page.locator('#local-app').isVisible(), true);
 assert.equal(await page.locator('#cube-canvas').getAttribute('data-shading'),'toon');
 const cdp = await context.newCDPSession(page); await cdp.send('DOM.enable'); await cdp.send('CSS.enable');
 const { root } = await cdp.send('DOM.getDocument'); const { nodeId } = await cdp.send('DOM.querySelector', { nodeId: root.nodeId, selector: '#clock' });
 const { fonts } = await cdp.send('CSS.getPlatformFontsForNode', { nodeId });
 assert.ok(fonts.some(f => f.familyName === 'Maple Mono NF' && f.isCustomFont));
 const {nodeId:navNode}=await cdp.send('DOM.querySelector',{nodeId:root.nodeId,selector:'.desktop-nav .nav-button'});
 const uiFonts=await cdp.send('CSS.getPlatformFontsForNode',{nodeId:navNode});
 assert.ok(uiFonts.fonts.some(f=>f.familyName.startsWith('Geist')&&f.isCustomFont),'Navigation uses bundled Geist');
 assert.equal(await page.locator('.wordmark').evaluate(el=>getComputedStyle(el).fontWeight),'600');
 checks.push('real Maple rendering and ready cube');
 await page.screenshot({ path: out + 'ready.png' });
 await page.locator('#history-menu').click();
 assert.equal(await page.locator('#history-menu').getAttribute('aria-expanded'), 'true');
 assert.equal(await page.locator('#local-detail').isDisabled(), true);
 assert.equal(await page.locator('#local-disconnect').isDisabled(), true);
 const menuBox = await page.locator('#local-more').boundingBox(), menuAnchor = await page.locator('#history-menu').boundingBox();
 assert.ok(menuBox.x >= 8 && menuBox.x < menuAnchor.x && menuBox.y >= menuAnchor.y + menuAnchor.height, 'Session menu anchors beside its own button');
 await page.keyboard.press('End');
 assert.equal(await page.evaluate(() => document.activeElement.id), 'local-legacy');
 await page.keyboard.press('ArrowDown');
 assert.equal(await page.evaluate(() => document.activeElement.textContent), 'Export / import');
 await page.keyboard.press('Escape');
 assert.equal(await page.locator('#local-more').isVisible(), false);
 assert.equal(await page.locator('#history-menu').getAttribute('aria-expanded'), 'false');
 assert.equal(await page.evaluate(() => document.activeElement.id), 'history-menu');
 await page.keyboard.press('ArrowDown');
 assert.equal(await page.locator('#local-more').isVisible(), true);
 await page.locator('.timer-pane').click({position:{x:8,y:8}});
 assert.equal(await page.locator('#local-more').isVisible(), false);
 assert.equal(await page.evaluate(() => stats.getTimesStatsList().timesLen), 0);
 assert.ok(await page.evaluate(() => timer.status() <= 0));
 checks.push('anchored action menu, disabled unavailable actions, arrow navigation, Escape focus return and outside dismissal');

 await page.locator('#device-button').click();
 await page.waitForFunction(() => kernel.getProp('input') === 'g' && $('.dialog:visible').length > 0);
 await page.screenshot({ path: out + 'connect.png' });
 await page.evaluate(() => { kernel.hideDialog(); kernel.setProp('input', 't'); });
 checks.push('Connect uses upstream Bluetooth confirmation');
 await page.keyboard.down('Space'); await page.waitForTimeout(600); await page.keyboard.up('Space'); await page.waitForTimeout(240); await page.keyboard.press('Space');
 await page.waitForFunction(() => stats.getTimesStatsList().timesLen === 1);
 assert.equal(await page.locator('#local-tps').textContent(), '—');
 checks.push('stock keyboard timing; no invented manual TPS');
 await page.evaluate(() => {
  // Four genuine CFOP boundaries constructed from inverse stage algorithms.
  const solution = "R2 F2 D2 L2 B2 R U R' U' F R U R' U' F' R U' R U R U R U' R' U' R2".split(' ');
  const inverse = solution.slice().reverse().map(m => m.endsWith('2') ? m : m.endsWith("'") ? m.slice(0, -1) : m + "'").join(' ');
  window.testSolution = { solution, inverse };
  for (let i = 0; i < 20; i++) {
   const time = 10500 + i * 170;
   const timed = solution.map((m, j) => m + '@' + Math.round(j * time / (solution.length - 1))).join(' ');
   kernel.pushSignal('time', ['', inverse, [0, time], 1700000000 + i * 30, [timed, '333']]);
  }
 });
 await page.waitForFunction(() => document.querySelectorAll('#phase-rows tr').length === 4);
 const result = await page.evaluate(() => {
  const index = stats.getTimesStatsList().timesLen - 1, r = stats.timesAt(index), rec = stats.getExtraInfo('recons_cf4op', index);
  return { moves: stats.getExtraInfo('mvcnt_htm', index), tps: stats.getExtraInfo('mvcnt_htm', index) * 1000 / r[0][1], time: stats.pretty(r[0], false), duration: r[0][1], data: rec.data.map(p => p.slice(0, 4)), totals:localReviewData.groupPhases(rec).map(p=>{const el=document.createElement('span');el.innerHTML=kernel.pretty(Math.round(p.time));return el.textContent;}), rows: [...document.querySelectorAll('#phase-rows tr')].map(row => [...row.children].map(td => td.textContent)) };
 });
 assert.equal(await page.locator('#local-moves').textContent(), String(result.moves));
 assert.equal(await page.locator('#local-tps').textContent(), result.tps.toFixed(2));
 assert.equal(await page.locator('#clock').textContent(), result.time);
 assert.ok(result.rows.every(row => row[1] !== '—'));
 const phases = result.data.reduce((sum, p) => sum + p[2] - p[0], 0);
 assert.equal(phases, result.duration);
 assert.ok(result.totals.every(time=>Number(time)>0));
 assert.equal(await page.locator('.phase-time-pair .phase-pause-value').count(),4);
 assert.equal(await page.locator('.phase-time-pair .phase-execution-value').count(),4);
 const averageText = await page.locator('#local-average-rows').innerText();
 assert.match(averageText, /ao5/); assert.match(averageText, /ao12/); assert.doesNotMatch(averageText, /—/);
 checks.push('timed reconstruction, four CFOP stages and complete averages');
 assert.equal(await page.locator('#review-tools').isVisible(),true);
 await page.locator('#phase-rows tr').first().click();
 assert.equal(await page.locator('[data-review-tool="cross"]').getAttribute('aria-pressed'),'true');
 await page.locator('#phase-rows tr').nth(1).focus();await page.keyboard.press('Enter');
 assert.equal(await page.locator('[data-review-tool="technique"]').getAttribute('aria-pressed'),'true');
 checks.push('colored pause/execution pairs and phase selection opens relevant review tools');
 assert.equal(await page.locator('#phase-strip button').count(),4);
 await page.locator('#phase-strip button[data-phase="3"]').click();
 assert.equal(await page.locator('[data-review-tool="algorithms"]').getAttribute('aria-pressed'),'true');
 assert.equal(await page.locator('.pace-mean').count(),1);
 assert.match(await page.locator('#pace-caption').textContent(),/Mean/);
 await page.locator('#pace-plot [data-index="3"]').click();
 assert.equal(await page.locator('#clock').textContent(), await page.evaluate(() => stats.pretty(stats.timesAt(3)[0], false)));
 await page.locator('#pace-plot [data-index="3"]').focus(); await page.keyboard.press('ArrowLeft');
 assert.equal(await page.evaluate(() => document.activeElement.dataset.index), '2');
 await page.keyboard.press('Enter');
 assert.equal(await page.locator('#clock').textContent(), await page.evaluate(() => stats.pretty(stats.timesAt(2)[0], false)));
 assert.equal(await page.locator('#pace-plot [data-index="2"]').getAttribute('aria-pressed'), 'true');
 await page.locator('#pace-plot [data-index="20"]').click();
 await page.locator('#phase-strip button[data-phase="1"]').click();
 checks.push('proportional CFOP navigation, recorded phase baseline, clickable trace and arrow-key solve review');

 // Let the GPU compositor present after the synchronous reconstruction fixture batch.
 await page.waitForTimeout(500);
 await writeFile(out + 'session.png', await assertVisibleCube(page));
 await page.locator('#history-menu').click(); await page.screenshot({path:out+'menu.png'}); await page.keyboard.press('Escape');
 for (const palette of ['carbon','paper']) {
  await page.evaluate(palette => kernel.setProp('localPalette',palette),palette);
  await page.waitForFunction(palette => document.documentElement.dataset.palette===palette,palette);
  const contrast = await page.evaluate(() => {
   const rgb=s=>getComputedStyle(document.querySelector(s)).color.match(/[\d.]+/g).slice(0,3).map(Number);
   const bg=getComputedStyle(document.body).backgroundColor.match(/[\d.]+/g).slice(0,3).map(Number);
   const luminance=rgb=>rgb.map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4;}).reduce((sum,v,i)=>sum+v*[.2126,.7152,.0722][i],0);
   const base=luminance(bg);
   return ['#clock','#local-tps','.solve-metrics','.scramble-line','.phase-table thead th','.tool-note','.phase-name','.average-table td:nth-child(2)'].map(selector=>{const value=luminance(rgb(selector));return {selector,ratio:(Math.max(base,value)+.05)/(Math.min(base,value)+.05)};});
  });
  assert.ok(contrast.every(p=>p.ratio>=4.5),palette+' readable text contrast: '+JSON.stringify(contrast));
  assert.equal(await page.locator('#clock').textContent(),result.time);
  await page.waitForTimeout(150);
  await writeFile(out+'palette-'+palette+'.png',await assertVisibleCube(page));
 }
 checks.push('two persistent interface modes preserve solve data, readable text contrast and visible cube');

 await page.evaluate(() => kernel.setProp('statsrc', 'mmvcnt_htm'));
 assert.equal(await page.locator('#local-average-rows').innerText(), averageText);
 await page.evaluate(() => kernel.setProp('statsrc', 't'));
 const popup = await context.newPage(); await popup.goto(url + '/companion.html');
 await popup.waitForFunction(() => document.querySelector('#companion-time').textContent !== '—');
 assert.equal(await popup.locator('#companion-time').textContent(), result.time);
 assert.equal(await popup.locator('#companion-tps').textContent(), result.tps.toFixed(2));
 await popup.setViewportSize({ width: 320, height: 112 }); await popup.screenshot({ path: out + 'web-companion.png' });
 checks.push('companion receives the latest real solve');
 await page.locator('#history-rows tr[data-index="3"]').click();
 const prior = await page.evaluate(() => stats.timesAt(3)[0][1]);
 await page.locator('[data-penalty="plus"]').click();
 assert.equal(await page.evaluate(() => stats.timesAt(3)[0][0]), 2000);
 assert.equal(await page.locator('#clock').textContent(), await page.evaluate(() => stats.pretty(stats.timesAt(3)[0], false)));
 await page.locator('[data-penalty="plus"]').click(); assert.equal(await page.evaluate(() => stats.timesAt(3)[0][0]), 0);
 // Keep an older comment intact while organizing the record with labels.
 await page.evaluate(()=>{stats.timesAt(3)[2]='Reviewed sample solve';return stats.getSessionManager().save(3);});
 await page.locator('#favorite-button').click();
 await page.waitForFunction(()=>stats.timesAt(3)[5]?.localReview?.favorite===true);
 await page.locator('#tag-button').click();await page.locator('[data-tag-color="mint"]').click();
 await page.waitForFunction(()=>stats.timesAt(3)[5]?.localReview?.color==='mint');
 await page.locator('#tag-button').click();await page.locator('[data-tag-group="practice"]').click();
 await page.waitForFunction(()=>stats.timesAt(3)[5]?.localReview?.group==='practice');
 assert.equal(await page.evaluate(() => stats.timesAt(3)[2]), 'Reviewed sample solve');
 assert.equal(await page.evaluate(() => stats.timesAt(3)[0][1]), prior);
 await page.locator('#replay-button').click(); await page.waitForFunction(() => $('.dialog:visible').length > 0);
 await page.screenshot({ path: out + 'replay.png' }); await page.evaluate(() => kernel.hideDialog());
 checks.push('history selection, upstream penalties, record-attached favorites/color/groups, preserved notes and timed replay');
 await page.locator('#session-button').selectOption('2'); await page.waitForFunction(() => stats.getTimesStatsList().timesLen === 0);
 assert.equal(await page.locator('#session-pace').isVisible(), false);
 assert.equal(await page.locator('#favorite-button').isDisabled(),true);
 await page.locator('#session-button').selectOption('1'); await page.waitForFunction(() => stats.getTimesStatsList().timesLen === 21);
 checks.push('session switching preserves full history');
 await page.locator('.desktop-nav [data-view="practice"]').click();
 await page.locator('[data-drill="pll"]').click(); await page.waitForFunction(() => kernel.getProp('scrType') === 'pll');
 await page.locator('.desktop-nav [data-view="practice"]').click(); await page.locator('#local-case-filter').click();
 await page.waitForFunction(() => $('.dialog:visible').length > 0); await page.evaluate(() => kernel.hideDialog());
 await page.locator('.desktop-nav [data-view="timer"]').click();
 checks.push('real PLL scrambles and upstream case filter');
 // Use the real utility callback with a synthetic transport, never a physical cube.
 await page.evaluate(async () => {
  window.testConnected = true;
  GiikerCube.isConnected = () => window.testConnected;
  GiikerCube.getCube = () => ({ getBatteryLevel: () => Promise.resolve([80, 'Test GAN']) });
  await giikerutil.init();
  GiikerCube.callback(mathlib.SOLVED_FACELET, [], [1000, Date.now()], 'Test GAN', '333');
  const cube = new mathlib.CubieCube(); cube.selfMoveStr('R');
  window.testFacelets = cube.toFaceCube();
  GiikerCube.callback(window.testFacelets, ['R'], [1100, Date.now()], 'Test GAN', '333');
 });
 await page.waitForFunction(() => document.querySelector('#cube-canvas').dataset.facelets === window.testFacelets && document.querySelector('#cube-canvas').dataset.animating !== 'true');
 await page.waitForFunction(() => document.querySelector('#device-button').classList.contains('connected'));
 await page.evaluate(() => {
  kernel.pushSignal('scramble', ['333', 'R2 U D F', 0]);
  GiikerCube.callback(mathlib.SOLVED_FACELET, [], [1200, Date.now()], 'Test GAN', '333');
  window.guideCube = new mathlib.CubieCube();
  window.sendGuideMove = function(move, time) { window.guideCube.selfMoveStr(move); GiikerCube.callback(window.guideCube.toFaceCube(), [move], [time, Date.now()], 'Test GAN', '333'); };
  window.sendGuideMove('R', 1300);
 });
 assert.match(await page.locator('#scramble-line').textContent(), /R◐/);
 await page.evaluate(() => window.sendGuideMove('F', 1400));
 assert.match(await page.locator('#scramble-line .local-correction').textContent(), /F'/);
 assert.match(await page.locator('#scramble-line').textContent(), /U D F/);
 await page.evaluate(() => { window.sendGuideMove("F'", 1500); window.sendGuideMove('R', 1600); window.sendGuideMove('D', 1700); });
 assert.equal(await page.locator('#scramble-line .local-correction').count(), 0);
 assert.match(await page.locator('#scramble-line .smrtScrCur').textContent(), /^U/);
 const segments = await page.evaluate(() => localInterface.snapshot().segments);
 assert.ok(segments.every(s => /\s$/.test(s.text)), 'Companion preserves spaces between highlighted moves');
 checks.push('inline undo, half-turn symbol, commuting opposite faces and widget move spacing');
 await page.evaluate(() => { kernel.setProp('input', 'g'); kernel.hideDialog(); timer.status(1); timer.lcd.val(12345); });
 await page.waitForFunction(() => document.querySelector('#clock').textContent.startsWith('12.34'));
 await page.evaluate(() => { timer.status(-1); kernel.setProp('input', 't'); window.testConnected = false; });
 await page.waitForFunction(() => !document.querySelector('#device-button').classList.contains('connected'));
 checks.push('utility callback cube state, animated turns, Bluetooth clock path and disconnect indicator');
 await page.setViewportSize({ width: 390, height: 844 }); await page.evaluate(() => localInterface.selectSolve(20));
 await page.waitForFunction(() => document.querySelector('.cube-stage').parentElement.classList.contains('timer-pane'));
 await page.locator('#phase-rows tr').first().click();
 assert.equal(await page.locator('#review-tools').isVisible(),true);
 await page.locator('#review-tools-close').click();
 assert.equal(await page.locator('#review-tools').isVisible(),false);
 await writeFile(out + 'phone.png', await assertVisibleCube(page));
 for (const [width,height] of [[320,740],[390,844],[1024,768],[1280,640],[1440,1000]]) {
  await page.setViewportSize({width,height});
  await page.waitForFunction(() => {
   const mobile=matchMedia('(max-width:760px)').matches;
   const rail=!mobile&&(matchMedia('(max-width:1100px)').matches||matchMedia('(max-height:740px)').matches);
   return document.querySelector('.average-table').parentElement.classList.contains(rail?'history-pane':'clock-anchor') && document.querySelector('#session-button').parentElement.classList.contains(mobile?'context-left':'history-heading');
  });
  await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
  const layout = await page.evaluate(() => {
   const rect = s => document.querySelector(s).getBoundingClientRect();
   const hit = (a,b) => a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top;
   return {overflow:document.documentElement.scrollWidth>innerWidth, cubeOffCenter:Math.abs((rect('.cube-stage').left+rect('.cube-stage').right)/2-innerWidth/2)>1, cubeClock:hit(rect('.cube-stage'),rect('#clock')), scrambleCube:hit(rect('.scramble-region'),rect('.cube-stage')), phaseAverage:hit(rect('.phase-section'),rect('.average-table'))};
  });

  const connectLabel=page.locator('#device-button>span');
  assert.equal(await connectLabel.isVisible(),width>760);
  assert.ok(await page.locator('#device-button').getAttribute('aria-label'));
  if(width<=760) {
   const hit=await page.locator('#device-button').boundingBox();
   assert.ok(hit.width>=44&&hit.height>=44,'Mobile Bluetooth has a full touch target');
  }
  if (width===1024) await page.screenshot({path:out+'tablet.png'});
  if (Object.values(layout).some(Boolean)) {
   const geometry=await page.evaluate(()=>({parent:document.querySelector('.average-table').parentElement.className,average:document.querySelector('.average-table').getBoundingClientRect().toJSON(),phase:document.querySelector('.phase-section').getBoundingClientRect().toJSON(),analysis:document.querySelector('.analysis-pane').getBoundingClientRect().toJSON(),view:document.body.dataset.view,columns:getComputedStyle(document.querySelector('.workspace')).gridTemplateColumns}));
   assert.fail(JSON.stringify({width,height,...layout,geometry}));
  }
 }
 await page.setViewportSize({width:390,height:844});
 checks.push('review tools sheet closes on phone; dedicated desktop tools and no overlaps at five viewport sizes');
 assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
 await page.locator('.mobile-nav [data-view="history"]').click(); await page.screenshot({ path: out + 'phone-history.png' });
 await page.locator('#history-rows tr').last().scrollIntoViewIfNeeded();
 await page.locator('#history-rows tr').first().scrollIntoViewIfNeeded(); await page.locator('#history-rows tr').first().click();
 checks.push('phone views and scrollable history');
 const clockSizeBefore = await page.locator('#clock').evaluate(el=>parseFloat(getComputedStyle(el).fontSize));
 await page.evaluate(() => kernel.setProp('timerSize', 11));
 await page.waitForFunction(previous => parseFloat(getComputedStyle(document.querySelector('#clock')).fontSize)<previous,clockSizeBefore);
 await page.reload(); await page.waitForFunction(() => stats.getTimesStatsList().timesLen === 21 && window.localInterface);
 assert.equal(await page.evaluate(() => kernel.getProp('timerSize')), 11);
 assert.deepEqual(await page.evaluate(()=>LocalSolveTags.read(stats.timesAt(3))),{favorite:true,color:'mint',group:'practice'});
 assert.equal(await page.evaluate(() => kernel.getProp('localPalette')), 'paper');
 assert.equal(await page.evaluate(() => document.documentElement.dataset.palette), 'paper');
 assert.equal(await page.evaluate(() => stats.timesAt(3)[2]), 'Reviewed sample solve');
 await context.setOffline(true); await page.reload(); await page.waitForFunction(() => window.localInterface && stats.getTimesStatsList().timesLen === 21);
 await context.setOffline(false);
 checks.push('settings/history persistence and offline startup');
 await page.evaluate(() => kernel.setProp('localPalette','lagoon')); await page.reload();
 await page.waitForFunction(() => window.localInterface && stats.getTimesStatsList().timesLen===21);
 assert.equal(await page.evaluate(() => kernel.getProp('localPalette')), 'paper');
 assert.equal(await page.evaluate(() => document.documentElement.dataset.palette), 'paper');
 assert.equal(await page.evaluate(() => stats.timesAt(3)[2]), 'Reviewed sample solve');
 checks.push('retired palette preference migrates without losing solve history or comments');


 // Full history: filters only change presentation, never the scorebook windows.
 await page.setViewportSize({width:1440,height:1000});
 await page.locator('.desktop-nav [data-view="history"]').click();
 await page.waitForFunction(()=>document.body.dataset.view==='history');
 assert.equal(await page.locator('.history-summary-stat').count(),4);
 assert.equal(await page.locator('.history-summary-average .history-summary-metrics b').count(),4);
 assert.equal(await page.locator('.history-summary-metrics b').count(),8);
 assert.equal(await page.locator('#history-rows .history-when').first().isVisible(),true);
 assert.deepEqual(await page.locator('#history-rows tr[data-index="20"] .history-phase').allTextContents(),result.totals);
 const expectedPause=await page.evaluate(data=>{const total=data.reduce((sum,p)=>sum+p[1]-p[0],0);const span=document.createElement('span');span.innerHTML=kernel.pretty(Math.round(total));return span.textContent;},result.data);
 assert.equal(await page.locator('#history-rows tr[data-index="20"] .history-pause').textContent(),expectedPause);
 assert.ok((await page.locator('#history-rows tr[data-index="0"] .history-phase').allTextContents()).every(text=>text==='—'));
 assert.ok((await page.locator('.history-table th.history-tags').boundingBox()).width<=150);

 assert.equal(await page.locator('.history-distribution').isVisible(),true);
 async function assertHistogramLayout() {
  const box = await page.locator('.history-distribution').boundingBox();
  const tools = await page.locator('.history-tools').boundingBox();
  const insights = await page.locator('.history-insights').boundingBox();
  assert.ok(insights.y+insights.height<=tools.y,'Insights stay above filters');
  if((await page.viewportSize()).width>760) {
   assert.ok(box.width<tools.width*0.8,'Desktop histogram shares its row with insights');
   assert.ok(box.x+box.width<insights.x,'Desktop insights sit beside histogram');
  } else assert.ok(insights.y>=box.y+box.height,'Phone insights sit below histogram');

  for (const tick of await page.locator('.distribution-tick').all()) {
   if (!await tick.isVisible()) continue;
   const label = await tick.boundingBox();
   assert.ok(label.y + label.height <= box.y + box.height, 'Axis labels fit inside chart');
   assert.ok(label.y + label.height < tools.y, 'Axis labels clear filters and separators');
  }
  const bars = await page.locator('.distribution-bar').evaluateAll(nodes=>nodes.map(n=>{const r=n.getBoundingClientRect();return {left:r.left,right:r.right,color:getComputedStyle(n).backgroundColor};}));
  for (let i=1;i<bars.length;i++) {
   assert.ok(Math.abs(bars[i].left-bars[i-1].right)<1, 'Histogram bins touch');
   assert.notEqual(bars[i].color,bars[i-1].color, 'Adjacent bins have different colors');
  }
 }
 await assertHistogramLayout();

 assert.equal(await page.locator('.distribution-bin').evaluateAll(nodes=>nodes.reduce((sum,n)=>sum+Number(n.dataset.count),0)),21);
 await page.locator('.distribution-bin').first().focus();
 const chartBeforeFocus=await page.locator('.history-distribution').boundingBox();
 await page.keyboard.press('ArrowRight');
 assert.equal(await page.locator('.distribution-bin').nth(1).evaluate(el=>document.activeElement===el),true);
 assert.equal(await page.locator('.distribution-bin[tabindex="0"]').count(),1);
 await page.keyboard.press('End');
 assert.equal(await page.locator('.distribution-bin').last().evaluate(el=>document.activeElement===el),true);
 await page.keyboard.press('Home');
 assert.equal(await page.locator('.distribution-bin').first().evaluate(el=>document.activeElement===el),true);
 assert.deepEqual(await page.locator('.history-distribution').boundingBox(),chartBeforeFocus,'Bin inspection never shifts layout');

 assert.match(await page.locator('#distribution-detail').textContent(), /s · \d+ \([\d.]+%\)/);
 await page.keyboard.press('Space');
 assert.equal(await page.evaluate(()=>timer.status()),-1);
 await page.locator('.distribution-bin').first().evaluate(el=>el.blur());
 const historyAverages=await page.locator('#local-average-rows').innerText();
 await page.screenshot({path:out+'history-desktop.png'});
 await page.evaluate(()=>kernel.setProp('localPalette','carbon'));
 await page.screenshot({path:out+'history-dark-desktop.png'});
 await page.locator('[data-history-filter="moves"]').click();
 assert.equal(await page.locator('#history-rows tr[data-index]').count(),20);
 assert.equal(await page.locator('#local-average-rows').innerText(),historyAverages);
 await page.locator('[data-history-filter="favorites"]').click();
 await page.locator('#history-query').fill('REVIEWED');
 await page.waitForFunction(()=>document.querySelectorAll('#history-rows tr[data-index]').length===1);
 assert.equal(await page.locator('#history-rows tr[data-index]').getAttribute('data-index'),'3');
 assert.equal(await page.locator('.distribution-bin').evaluateAll(nodes=>nodes.reduce((sum,n)=>sum+Number(n.dataset.count),0)),1);
 await page.screenshot({path:out+'history-filtered.png'});
 await page.locator('#history-rows .history-row-detail').click();
 await page.waitForFunction(()=>$('.dialog:visible').length>0);
 assert.equal(await page.evaluate(()=>document.body.dataset.view),'history');
 await page.evaluate(()=>kernel.hideDialog());
 await page.locator('[data-history-filter="dnf"]').click();
 assert.match(await page.locator('#history-rows').innerText(),/No matching solves/);
 assert.equal(await page.locator('.distribution-bin').count(),0);
 assert.equal(await page.locator('.distribution-empty').textContent(),'No timed solves');
 assert.deepEqual(await page.locator('.history-insights dd').allTextContents(),['—','—','—','—']);
 await page.locator('#history-query-clear').click();
 await page.locator('[data-history-filter="all"]').click();
 await page.locator('#history-query').fill('#4');
 await page.waitForFunction(()=>document.querySelectorAll('#history-rows tr[data-index]').length===1);
 assert.equal(await page.locator('#history-rows tr[data-index]').getAttribute('data-index'),'3');
 await page.locator('#history-query').press('Escape');
 await page.waitForFunction(()=>document.querySelectorAll('#history-rows tr[data-index]').length===21);
 assert.equal(await page.evaluate(()=>stats.getTimesStatsList().timesLen),21);
 assert.equal(await page.locator('#local-average-rows').innerText(),historyAverages);
 checks.push('history search, recorded-move/note/DNF filters, original solve numbering, inline details and unchanged averages');
 await page.setViewportSize({width:390,height:844});
 await page.waitForFunction(()=>document.querySelector('#session-button').parentElement.classList.contains('context-left'));
 await page.screenshot({path:out+'history-dark-phone.png'});
 assert.equal(await page.locator('#history-rows .history-when').first().isVisible(),false);
 assert.equal(await page.locator('#history-rows .history-phase').first().isVisible(),false);
 await assertHistogramLayout();
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 await page.setViewportSize({width:320,height:740});
 await page.screenshot({path:out+'history-small-phone.png'});
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
 await page.setViewportSize({width:390,height:844});
 // Add enough manual records to exercise lazy appending across the page boundary.
 await page.evaluate(()=>{
  for(let i=0;i<110;i++)kernel.pushSignal('time',['','',[i%17===0?-1:0,15000+i*25],1700010000+i*30]);
 });
 await page.waitForFunction(()=>stats.getTimesStatsList().timesLen===131);
 assert.equal(await page.locator('#history-rows tr[data-index]').count(),100);
 await page.locator('.history-scroll').evaluate(el=>el.scrollTop=el.scrollHeight);
 await page.waitForFunction(()=>document.querySelectorAll('#history-rows tr[data-index]').length===131);
 assert.equal(await page.locator('#history-rows tr[data-index]').last().getAttribute('data-index'),'0');
 const firstRow=page.locator('#history-rows tr[data-index]').first();
 await firstRow.focus();
 assert.equal(await page.locator('#history-rows button[tabindex="0"]').count(),0);
 await page.keyboard.press('ArrowRight');
 assert.equal(await page.evaluate(()=>document.activeElement.classList.contains('history-row-detail')),true);
 await page.keyboard.press('ArrowLeft');
 assert.equal(await page.evaluate(()=>document.activeElement.dataset.index),'130');
 await page.keyboard.press('ArrowDown');
 assert.equal(await page.evaluate(()=>document.activeElement.dataset.index),'129');
 assert.equal(await page.locator('#history-rows tr[tabindex="0"]').count(),1);
 await page.keyboard.press('End');assert.equal(await page.evaluate(()=>document.activeElement.dataset.index),'0');
 await page.keyboard.press('Home');assert.equal(await page.evaluate(()=>document.activeElement.dataset.index),'130');
 await page.keyboard.press('Enter');
 assert.equal(await page.evaluate(()=>document.body.dataset.view),'timer');
 assert.equal(await page.evaluate(()=>stats.getTimesStatsList().timesLen),131);
 assert.equal(await page.locator('#local-tps').textContent(),'—');
 checks.push('history lazily appends beyond 100 solves, maintains keyboard focus and reviews without inventing manual TPS');
 assert.deepEqual(errors, []);
 await writeFile(out + 'checks.json', JSON.stringify({ checks, errors, syntheticRecords: true, renderedFonts: fonts, cfopFixture: result }, null, 2));
 console.log('Live interface workflows passed: ' + checks.length + '; screenshots: ' + out);
} finally { await browser.close(); }
