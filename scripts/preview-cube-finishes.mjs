// Compare actual renderer finishes in isolated browser contexts. No live edits.
import { chromium } from 'playwright-core';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('../', import.meta.url));
const paletteReview = process.env.CSTIMER_CUBE_REVIEW === 'palette';
const out = root + '.data/implemented/' + (paletteReview ? 'palettes/' : 'finishes/');
await mkdir(out, { recursive: true });
const source = await readFile(root + 'src/js/local-cube-view.js', 'utf8');
const finishes = [
 { name: 'Natural plastic', note: 'Clean colors · satin finish', exposure: .95, ambient: .8, key: 3.1, fill: 1.65, environment: .5, roughness: .24, coat: .45,
   colors: { U: 0xe9edf2, R: 0xe94d59, F: 0x32b575, D: 0xecd44e, L: 0xeb954e, B: 0x438bd8 } },
 { name: 'Soft matte', note: 'Gentle colors · soft reflections', exposure: 1, ambient: .9, key: 2.6, fill: 1.5, environment: .4, roughness: .5, coat: .1,
   colors: { U: 0xd1d6df, R: 0xdc8991, F: 0x7db99a, D: 0xe3d18b, L: 0xe1ac83, B: 0x8baedb } },
 { name: 'Theme palette', note: 'Muted colors · matte finish', exposure: 1, ambient: 1, key: 2.2, fill: 1.4, environment: .35, roughness: .65, coat: 0,
   colors: { U: 0xc7c9da, R: 0xd18c9c, F: 0x9cb9a3, D: 0xd4c197, L: 0xd4a18a, B: 0x939fce } }
];
const variants = paletteReview ? [
 { ...finishes[0], name: 'Original option 1', note: 'Original natural plastic palette' },
 { ...finishes[0], name: 'Refined palette', note: 'Warmer red · leaf green · deeper blue',
   colors: { U: 0xe4e7eb, R: 0xdf454a, F: 0x43a96e, D: 0xe5cb55, L: 0xe58a4b, B: 0x447cc7 } }
] : finishes;
const browser = await chromium.launch({ executablePath: process.env.CSTIMER_BROWSER || '/etc/profiles/per-user/zhenyu/bin/brave', headless: true,
 args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
try {
 for (const [index, variant] of variants.entries()) {
  const context = await browser.newContext({ viewport: { width: 640, height: 560 }, deviceScaleFactor: 2, serviceWorkers: 'block' });
  const page = await context.newPage();
  let module = source.replace(/renderer.toneMappingExposure = [^;]+;/, `renderer.toneMappingExposure = ${variant.exposure};`)
   .replace(/new THREE.AmbientLight\(0xffffff, [^)]+\)/, `new THREE.AmbientLight(0xffffff, ${variant.ambient})`)
   .replace(/const key = new THREE.DirectionalLight\(0xffffff, [^)]+\)/, `const key = new THREE.DirectionalLight(0xffffff, ${variant.key})`)
   .replace(/const fill = new THREE.DirectionalLight\(0xffffff, [^)]+\)/, `const fill = new THREE.DirectionalLight(0xffffff, ${variant.fill})`)
   .replace(/scene.environmentIntensity = [^;]+;/, `scene.environmentIntensity = ${variant.environment};`)
   .replace(/const colors = \{[\s\S]*?\};/, `const colors = ${JSON.stringify(variant.colors)};`)
   .replace(/color, roughness: [^,]+, metalness: 0, clearcoat: [^,]+, clearcoatRoughness: [^}]+/, `color, roughness: ${variant.roughness}, metalness: 0, clearcoat: ${variant.coat}, clearcoatRoughness: .35 `);
  await page.route('**/js/local-cube-view.js', route => route.fulfill({ contentType: 'text/javascript', body: module }));
  await page.goto(process.env.CSTIMER_URL || 'http://localhost:3330');
  await page.waitForFunction(() => document.querySelector('#cube-canvas')?.dataset.ready === 'true');
  await page.addStyleTag({ content: '.cube-stage{position:fixed!important;inset:0!important;transform:none!important;width:640px!important;height:560px!important;background:#1e2233;z-index:1000}.cube-reset{display:none}' });
  await page.evaluate(() => { const c = new mathlib.CubieCube(); for (const m of "R U F2 L' D B".split(' ')) c.selfMoveStr(m); window.dispatchEvent(new CustomEvent('local-cube-state', { detail: { facelets: c.toFaceCube() } })); });
  await page.waitForTimeout(300);
  await page.locator('#cube-canvas').screenshot({ path: out + index + '.png' });
  await context.close();
 }
 const font = (await readFile(root + 'src/fonts/MapleMono-UI-Regular.woff2')).toString('base64');
 const cards = await Promise.all(variants.map(async (v, i) => `<article><img src="data:image/png;base64,${(await readFile(out + i + '.png')).toString('base64')}"/><h2>${i + 1} · ${v.name}</h2><p>${v.note}</p></article>`));
 const html = `<!doctype html><style>@font-face{font-family:Maple;src:url(data:font/woff2;base64,${font})}*{box-sizing:border-box}body{margin:0;padding:28px 24px;background:#1e2233;color:#e4e3ef;font:14px/1.5 Maple,monospace}header{padding:0 12px 16px;border-bottom:1px solid #ffffff15}header p{margin:4px 0 0;color:#adb1c7;font-size:12px}main{display:grid;grid-template-columns:repeat(${variants.length},1fr);gap:16px;padding-top:12px}article{padding:0 8px}article+article{border-left:1px solid #ffffff10}img{display:block;width:100%}h2{font:14px Maple;margin:4px 8px}article p{font-size:12px;color:#adb1c7;margin:4px 8px}</style><header>${paletteReview ? "Cube color refinement" : "Cube finish study"}<p>Actual 3D renderer · same cube, camera and scramble${paletteReview ? " · same finish" : ""}</p></header><main>${cards.join('')}</main>`;
 await writeFile(out + 'index.html', html);
 const page = await browser.newPage({ viewport: { width: paletteReview ? 960 : 1200, height: paletteReview ? 560 : 510 }, deviceScaleFactor: 2 });
 await page.setContent(html); await page.evaluate(() => document.fonts.ready);
 await page.screenshot({ path: out + 'comparison.png', fullPage: true });
 console.log(out + 'comparison.png');
} finally { await browser.close(); }
