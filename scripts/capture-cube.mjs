// Capture the actual renderer at UI size and enlarged for geometry review.
import { chromium } from 'playwright-core';
import { mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
const out = fileURLToPath(new URL('../.data/implemented/', import.meta.url));
await mkdir(out, { recursive: true });
const browser = await chromium.launch({
 executablePath: process.env.CSTIMER_BROWSER || '/etc/profiles/per-user/zhenyu/bin/brave',
 headless: true,
 args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader']
});
try {
 const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
 const errors = [];
 page.on('pageerror', error => errors.push(error.message));
 page.on('console', message => {
  if (message.type() === 'error' && /THREE|WebGL|shader|GL_INVALID/.test(message.text())) errors.push(message.text());
 });
 await page.goto(process.env.CSTIMER_URL || 'http://localhost:3330');
 await page.waitForFunction(() => document.querySelector('#cube-canvas')?.dataset.ready === 'true');
 await page.waitForFunction(() => document.querySelector('#cube-canvas')?.dataset.centerMark === 'ready');
 await page.waitForTimeout(300);
 await page.screenshot({ path: out + 'cube-redesign-desktop.png' });
 await page.addStyleTag({ content: '.cube-stage{position:fixed!important;inset:0!important;transform:none!important;width:640px!important;height:560px!important;background:#17191d;z-index:1000}.cube-reset{display:none}' });
 await page.waitForTimeout(300);
 await page.locator('#cube-canvas').screenshot({ path: out + 'cube-redesign-detail.png' });
 await page.evaluate(() => {
  const cube = new mathlib.CubieCube();
  for (const move of "R U F2 L' D B".split(' ')) cube.selfMoveStr(move);
  window.dispatchEvent(new CustomEvent('local-cube-state', { detail: { facelets: cube.toFaceCube() } }));
 });
 await page.locator('#cube-canvas').screenshot({ path: out + 'cube-redesign-scrambled.png' });
 await page.locator('.cube-stage').evaluate(stage => { stage.style.background = '#fff'; });
 await page.locator('#cube-canvas').screenshot({ path: out + 'cube-gan-white.png' });
 if (errors.length) throw new Error('Cube rendering failed: ' + errors.join('\n'));
 console.log('Actual cube renderer captured: solved, scrambled, desktop. Synthetic cube state; no Bluetooth connection.');
} finally { await browser.close(); }
