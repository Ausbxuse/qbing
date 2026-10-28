// Screenshot-only visual study. All records and connections are sample data.
import { chromium } from 'playwright-core';
import { mkdir, writeFile, cp } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('../', import.meta.url));
const out = root + '.data/focus/';
const gallery = root + 'design/interface-preview/focus-shots/';
await mkdir(out, { recursive: true }); await mkdir(gallery, { recursive: true });
const browser = await chromium.launch({ executablePath: process.env.CSTIMER_BROWSER || '/etc/profiles/per-user/zhenyu/bin/brave', headless: true,
 args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
const errors = [], layouts = [];
const scenes = [
 ['carbon',1440,1000,'review','timer'],['desktop',1440,1000,'review','timer'],['laptop',1280,800,'review','timer'],['compact',1280,640,'review','timer'],['tablet',1024,768,'review','timer'],
 ['scrambling',1440,1000,'scramble','timer'],['solving',1440,1000,'solve','timer'],['history',1440,1000,'review','history'],['empty',1440,1000,'review','timer',true],
 ['menu',1440,1000,'review','timer'],['phone-menu',390,844,'review','timer'],
 ['phone',390,844,'review','timer'],['phone-scrambling',390,844,'scramble','timer'],['phone-solving',390,844,'solve','timer'],
 ['phone-history',390,844,'review','history'],['phone-empty',390,844,'review','timer',true],['phone-small',320,740,'review','timer']
];
try {
 const page = await browser.newPage();
 page.on('pageerror', error => errors.push(error.message));
 page.on('console', message => { if (message.type() === 'error' && /THREE|WebGL|shader/.test(message.text())) errors.push(message.text()); });
 for (const [name,width,height,state,view,empty] of scenes) {
  await page.setViewportSize({ width,height });
  await page.goto(`http://localhost:3328/focus.html?embed=1&state=${state}&view=${view}${empty ? '&session=empty' : ''}${name === 'carbon' ? '&palette=carbon' : ''}`);
  await page.waitForFunction(() => document.querySelector('#cube-canvas')?.dataset.centerMark === 'ready' && document.body.dataset.studyCube && document.querySelector('#cube-canvas').dataset.ready === 'true');
  await page.evaluate(() => document.fonts.ready);
  await page.evaluate(() => window.dispatchEvent(new Event('local-cube-redraw')));
  await page.waitForTimeout(150);
  if (name === 'menu') await page.locator('#history-menu').click();
  if (name === 'phone-menu') { await page.locator('.mobile-nav [data-view=history]').click(); await page.locator('#history-menu').click(); }
  if (view === 'timer' && name !== 'phone-menu') layouts.push(await page.evaluate(name => {
   const rect = selector => document.querySelector(selector).getBoundingClientRect();
   const clock = rect('#clock'), cube = rect('.cube-stage'), avg = rect('.average-table'), phase = rect('.phase-section'), scramble = rect('.scramble-region');
   const intersects = (a,b) => a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top;
   return { name, overflow: document.documentElement.scrollWidth > innerWidth, cubeClock: intersects(cube,clock), averageClock: intersects(avg,clock), phaseGap: phase.top - avg.bottom, scrambleGap: cube.top - scramble.bottom, phaseOverlap: intersects(phase,avg), scrambleOverlap: intersects(scramble,cube) };
  },name));
  await page.screenshot({ path: out + name + '.png' }); await cp(out + name + '.png', gallery + name + '.png');
 }
 await writeFile(out + 'checks.json', JSON.stringify({ synthetic:true,errors,layouts },null,2));
 if (errors.length) throw new Error(errors.join('\n'));
 if (layouts.some(x => x.overflow || x.cubeClock || x.averageClock || x.phaseOverlap || x.scrambleOverlap)) throw new Error('Layout overlaps: ' + JSON.stringify(layouts));
 console.log(`Captured ${scenes.length} design workflows; no rendering errors or layout overlaps.`);
} finally { await browser.close(); }
