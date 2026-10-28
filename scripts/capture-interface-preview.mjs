import { chromium } from 'playwright-core';
import { mkdir, writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';

const out = fileURLToPath(new URL('../.data/interface-preview/', import.meta.url));
await mkdir(out, { recursive: true });
const browser = await chromium.launch({
  executablePath: process.env.CSTIMER_BROWSER || '/etc/profiles/per-user/zhenyu/bin/brave',
  headless: true, args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader']
});
const evidence = [];
const errors = [];
try {
  const page = await browser.newPage();
  page.on('pageerror', error => errors.push(error.message));
  for (const [name, width, height, state, view] of [
    ['desktop-review', 1440, 1000, 'review', 'timer'],
    ['desktop-scramble', 1440, 1000, 'scramble', 'timer'],
    ['desktop-solve', 1440, 1000, 'solve', 'timer'],
    ['phone-review', 390, 844, 'review', 'timer'],
    ['phone-scramble', 390, 844, 'scramble', 'timer'],
    ['phone-history', 390, 844, 'review', 'history']
  ]) {
    await page.setViewportSize({ width, height });
    await page.goto(`http://localhost:3328/?state=${state}&view=${view}`);
    await page.evaluate(() => document.fonts.ready);
    await page.waitForSelector('#cube-canvas[data-ready="true"]', { state: 'attached' });
    await page.waitForTimeout(300);
    const geometry = await page.evaluate(() => {
      const clock = document.querySelector('#clock');
      const r = clock?.getBoundingClientRect();
      return {
        overflow: document.documentElement.scrollWidth > innerWidth,
        clock: r ? { x: r.x + r.width / 2, y: r.y + r.height / 2 } : null,
        previewLabel: document.body.textContent.includes('sample data')
      };
    });
    assert.equal(geometry.overflow, false, name + ': no horizontal overflow');
    assert.equal(geometry.previewLabel, true, 'Sample data must be identified');
    await page.screenshot({ path: out + name + '.png', fullPage: true });
    evidence.push({ name, width, height, state, view, ...geometry });
  }
  for (const current of evidence.slice(1, 3)) {
    assert.ok(current.clock && evidence[0].clock, 'Timer present in every desktop state');
    assert.ok(Math.abs(current.clock.x - evidence[0].clock.x) < 1 && Math.abs(current.clock.y - evidence[0].clock.y) < 1,
      'Timer stays in place across ready/running/review');
  }
  assert.deepEqual(errors, []);
  await writeFile(out + 'checks.json', JSON.stringify({ evidence, errors, scope: 'Visual prototype with sample data; no timer or Bluetooth integration' }, null, 2));
  console.log('Interface previews captured: ' + out);
} finally { await browser.close(); }
