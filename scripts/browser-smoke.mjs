import { chromium } from 'playwright-core';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
const root = fileURLToPath(new URL('..', import.meta.url));
const base = process.env.CSTIMER_URL || 'http://localhost:3327';
const url = base + (base.includes('?') ? '&' : '?') + 'interface=classic';
const out = process.env.CSTIMER_SCREENSHOTS || root + '/.data/browser';
await mkdir(out, { recursive: true });
const browserPath = process.env.CSTIMER_BROWSER || spawnSync(process.platform === 'win32' ? 'where' : 'which', ['brave'], { encoding: 'utf8' }).stdout?.trim().split(/\r?\n/)[0];
if (!browserPath) throw new Error('Set CSTIMER_BROWSER to a Chrome/Brave executable path.');
const browser = await chromium.launch({ executablePath: browserPath, headless: true, args: ['--no-sandbox', '--enable-experimental-web-platform-features', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
const errors = [];
const fontEvidence = {};
async function renderedFonts(page, selector) {
  await page.evaluate(() => document.fonts.ready);
  const cdp = await page.context().newCDPSession(page);
  try {
    await cdp.send('DOM.enable');
    await cdp.send('CSS.enable');
    const { root } = await cdp.send('DOM.getDocument');
    const { nodeId } = await cdp.send('DOM.querySelector', { nodeId: root.nodeId, selector });
    return (await cdp.send('CSS.getPlatformFontsForNode', { nodeId })).fonts.filter(font => font.glyphCount > 0);
  } finally { await cdp.detach(); }
}
async function fontSize(page, selector) {
  return page.locator(selector).evaluate(element => parseFloat(getComputedStyle(element).fontSize));
}
async function assertExistingPreferences(storageState, expectedHistory) {
  const cases = [
    { name: 'unspecified decimals', edits: {}, expected: false },
    { name: 'explicit small decimals', edits: { smallADP: true }, expected: true },
    { name: 'explicit full decimals', edits: { smallADP: false }, expected: false },
    { name: 'appearance disabled', edits: { localAppearance: false }, expected: true }
  ];
  for (const test of cases) {
    const state = structuredClone(storageState);
    const origin = state.origins.find(item => item.origin === new URL(url).origin);
    const entry = origin.localStorage.find(item => item.name === 'properties');
    const properties = JSON.parse(entry.value);
    delete properties.localTypographyVersion;
    delete properties.smallADP;
    Object.assign(properties, { timerSize: 17, font: 'Arial', 'col-font': '#abc' }, test.edits);
    entry.value = JSON.stringify(properties);
    const context = await browser.newContext({ storageState: state });
    try {
      const page = await context.newPage();
      page.on('pageerror', error => errors.push(error.message));
      await page.goto(url);
      await page.waitForFunction(count => window.stats && stats.getTimesStatsList().timesLen === count, expectedHistory.length);
      const result = await page.evaluate(() => ({
        small: kernel.getProp('smallADP'), version: kernel.getProp('localTypographyVersion'),
        size: kernel.getProp('timerSize'), font: kernel.getProp('font'), color: kernel.getProp('col-font'),
        appearance: kernel.getProp('localAppearance'),
        history: Array.from({ length: stats.getTimesStatsList().timesLen }, (_, index) => stats.timesAt(index))
      }));
      assert.equal(result.small, test.expected, test.name);
      assert.equal(result.version, 2, test.name);
      assert.equal(result.size, 17, test.name);
      assert.equal(result.font, 'Arial', test.name);
      assert.equal(result.color, '#abc', test.name);
      assert.equal(result.appearance, properties.localAppearance !== false, test.name);
      assert.deepEqual(result.history, expectedHistory, 'Migration preserves complete solves and reconstruction');
      if (test.name === 'unspecified decimals') {
        // Upstream removes values equal to its default from stored properties.
        // The migration marker must prevent changing that later user choice.
        await page.evaluate(() => kernel.setProp('smallADP', true));
        assert.equal(await page.evaluate(() => Object.hasOwn(JSON.parse(localStorage.properties), 'smallADP')), false);
        await page.reload();
        await page.waitForFunction(() => window.stats && kernel.getProp('localTypographyVersion') === 2);
        assert.equal(await page.evaluate(() => kernel.getProp('smallADP')), true);
      }
    } finally { await context.close(); }
  }
}
async function assertAverageMetricsUncovered(page) {
  assert.equal(await page.evaluate(() => Array.from(document.querySelectorAll('#local-average-metrics .local-average-row > span')).every(span => {
    const r = span.getBoundingClientRect();
    const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
    return r.width > 0 && r.height > 0 && hit?.closest('#local-average-metrics');
  })), true, 'Average TPS/moves must not be covered by the tools window');
}
try {
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const page = await context.newPage();
  page.on('pageerror', error => errors.push(error.message));
  page.on('dialog', dialog => dialog.dismiss());
  await page.goto(url);
  await page.waitForFunction(() => window.stats && stats.getTimesStatsList().timesLen === 0 && document.querySelector('#scrambleTxt')?.textContent.length > 5);
  await page.evaluate(() => document.fonts.ready);
  await page.evaluate(() => navigator.serviceWorker.ready);
  assert.equal(await page.evaluate(() => kernel.getProp('input')), 't');
  assert.equal(await page.evaluate(() => kernel.getProp('vrcMP')), 'cfop');
  assert.equal(await page.evaluate(() => kernel.getProp('giiVRC')), 'v');
  assert.equal(await page.evaluate(() => !!navigator.bluetooth), true);
  assert.equal(await page.evaluate(() => kernel.getProp('smallADP')), false);
  fontEvidence.clock = await renderedFonts(page, '#lcd');
  assert.ok(fontEvidence.clock.some(font => font.familyName === 'Geist Mono' && font.isCustomFont), 'Clock must render the bundled Geist Mono font, not a fallback');
  assert.equal(await page.locator('#lcd').evaluate(element => getComputedStyle(element).fontWeight), '500');
  assert.equal(await fontSize(page, '#lcd'), 96);
  assert.equal(await page.locator('#lcd').evaluate(element => Array.from(element.querySelectorAll('span')).every(span => getComputedStyle(span).fontSize === getComputedStyle(element).fontSize)), true, 'Default decimals use the same size as the integer');
  await page.evaluate(() => kernel.setProp('smallADP', true));
  assert.equal(await page.locator('#lcd').evaluate(element => Array.from(element.querySelectorAll('span')).some(span => parseFloat(getComputedStyle(span).fontSize) === parseFloat(getComputedStyle(element).fontSize) * 0.75)), true, 'Upstream small-decimal preference remains functional');
  await page.evaluate(() => { kernel.setProp('smallADP', false); kernel.setProp('timerSize', 18); kernel.setProp('scrSize', 20); });
  assert.equal(await fontSize(page, '#lcd'), 144);
  assert.equal(await fontSize(page, '#scrambleTxt'), 40);
  await page.evaluate(() => { kernel.setProp('timerSize', 12); kernel.setProp('scrSize', 15); kernel.setProp('zoom', '1.25'); });
  assert.equal(await fontSize(page, '#lcd'), 120);
  assert.equal(await fontSize(page, '#scrambleTxt'), 37.5);
  await page.evaluate(() => { kernel.setProp('zoom', '1'); kernel.setProp('scrMono', false); });
  fontEvidence.scrambleSans = await renderedFonts(page, '#scrambleTxt');
  assert.ok(fontEvidence.scrambleSans.some(font => font.isCustomFont && font.familyName.startsWith('Geist') && !font.familyName.includes('Mono')));
  assert.equal(fontEvidence.scrambleSans.some(font => font.familyName.includes('Mono')), false);
  await page.evaluate(() => kernel.setProp('scrMono', true));
  fontEvidence.scrambleMono = await renderedFonts(page, '#scrambleTxt');
  assert.ok(fontEvidence.scrambleMono.some(font => font.familyName === 'Geist Mono' && font.isCustomFont));
  await page.screenshot({ path: out + '/ready.png' });

  // Opening csTimer's connection confirmation must select the correct input,
  // without requesting or pairing a physical device in this test.
  await page.getByText('Click to connect', { exact: false }).click();
  await page.waitForFunction(() => kernel.getProp('input') === 'g' && $('.dialog:visible').length > 0);
  await page.screenshot({ path: out + '/connect.png' });
  await page.keyboard.press('Escape');
  await page.evaluate(() => { kernel.hideDialog(); kernel.setProp('input', 't'); });

  // Exercise stock keyboard timing rather than a replacement timer.
  await page.keyboard.down('Space');
  await page.waitForTimeout(600);
  await page.keyboard.up('Space');
  await page.waitForTimeout(250);
  await page.keyboard.press('Space');
  await page.waitForFunction(() => stats.getTimesStatsList().timesLen === 1);
  assert.equal(await page.evaluate(() => !!stats.timesAt(0)[4]), false);

  // Valid synthetic inverse solutions through the real save/reconstruction path.
  await page.evaluate(() => {
    const scramble = "R U F2 L D' R2 B U2 F' L2 D B2 R' U F2";
    const solution = "F2 U' R B2 D' L2 F U2 B' R2 D L' F2 U' R'".split(' ');
    for (let i = 0; i < 15; i++) {
      const duration = 9500 + i * 310;
      const timed = solution.map((move, index) => move + '@' + Math.round(index * duration / (solution.length - 1))).join(' ');
      kernel.pushSignal('time', ['Synthetic browser test', scramble, [0, duration], 1700000000 + i * 30, [timed, '333']]);
    }
  });
  await page.waitForFunction(() => document.querySelectorAll('#local-average-metrics .local-average-row').length === 2);
  assert.equal(await page.evaluate(() => stats.getExtraInfo('mvcnt_htm', 15)), 15);
  assert.ok(await page.evaluate(() => stats.getExtraInfo('recons_cf4op', 15)));
  const metrics = await page.locator('#local-average-metrics').innerText();
  assert.match(metrics, /ao5/); assert.match(metrics, /ao12/); assert.match(metrics, /15\.0 moves/);
  await assertAverageMetricsUncovered(page);
  await page.screenshot({ path: out + '/session.png' });
  await page.evaluate(() => kernel.setProp('statsrc', 'mmvcnt_htm'));
  assert.equal(await page.evaluate(() => kernel.getProp('statsrc')), 'mmvcnt_htm');
  assert.equal(await page.evaluate(() => stats.getTimesStatsTable().timeAt(15)), 15, 'History must actually switch from milliseconds to move counts');
  assert.equal(await page.locator('#local-average-metrics').innerText(), metrics);
  await page.evaluate(() => { kernel.setProp('statsrc', 't'); kernel.setProp('localAppearance', false); });
  assert.equal(await page.evaluate(() => document.body.classList.contains('local-appearance')), false);
  await page.evaluate(() => { kernel.setProp('localAppearance', true); kernel.setProp('timerSize', 11); });
  await page.reload();
  await page.waitForFunction(() => stats.getTimesStatsList().timesLen === 16);
  assert.equal(await page.evaluate(() => kernel.getProp('timerSize')), 11);
  assert.equal(await fontSize(page, '#lcd'), 88);
  assert.equal(await page.locator('#local-average-metrics').innerText(), metrics);
  await page.evaluate(() => replay.popupReplay(stats.timesAt(15)[1], stats.timesAt(15)[4][0], '333'));
  await page.waitForFunction(() => $('.dialog:visible').length > 0);
  await page.waitForTimeout(500);
  await page.screenshot({ path: out + '/replay.png' });
  await page.evaluate(() => kernel.hideDialog());
  await page.setViewportSize({ width: 390, height: 844 });
  await page.waitForTimeout(200);
  assert.equal(await fontSize(page, '#lcd'), 44, 'Phone clock respects saved timer size');
  await page.evaluate(() => kernel.setProp('timerSize', 12));
  assert.equal(await fontSize(page, '#lcd'), 48);
  assert.equal(await fontSize(page, '#scrambleTxt'), 24);
  await page.screenshot({ path: out + '/phone.png' });
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  assert.equal(await page.locator('#local-average-metrics').isVisible(), true);
  await assertAverageMetricsUncovered(page);
  // Exercise the stock offline worker with the local extension assets included.
  await context.setOffline(true);
  await page.reload();
  await page.waitForFunction(() => stats.getTimesStatsList().timesLen === 16);
  assert.equal(await page.evaluate(() => document.body.classList.contains('local-appearance')), true);
  await context.setOffline(false);
  const expectedHistory = await page.evaluate(() => Array.from({ length: stats.getTimesStatsList().timesLen }, (_, index) => stats.timesAt(index)));
  await assertExistingPreferences(await context.storageState({ indexedDB: true }), expectedHistory);
  assert.deepEqual(errors, []);
  await writeFile(out + '/checks.json', JSON.stringify({ passed: ['fresh defaults', 'rendered Geist Mono font', 'decimal and size preferences', 'scramble font toggle', 'zoom scaling', 'Web Bluetooth API', 'Connect selects Bluetooth timing', 'stock keyboard solve', 'real reconstruction', 'trimmed averages', 'actual metric selection independence', 'settings and history persistence', 'existing preference migration', 'migration applies once', 'timed replay', 'phone layout and typography', 'offline reload'], renderedFonts: fontEvidence, consoleErrors: errors }, null, 2));
  console.log('Browser workflows passed; screenshots: ' + out);
} finally { await browser.close(); }
