// Render the real Rust window against an isolated csTimer browser session.
import { chromium } from 'playwright-core';
import { spawn, spawnSync } from 'node:child_process';
import { mkdir, access, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
const root = fileURLToPath(new URL('..', import.meta.url));
const out = path.join(root, '.data/implemented');
const url = process.env.CSTIMER_URL || 'http://localhost:3330';
await mkdir(out, { recursive: true });
const metadata = spawnSync('cargo', ['metadata', '--manifest-path', 'companion/Cargo.toml', '--no-deps', '--format-version', '1'], { cwd: root, encoding: 'utf8' });
if (metadata.status !== 0) throw new Error('Run this capture in nix develop');
const binary = path.join(JSON.parse(metadata.stdout).target_directory, 'release/cstimer-companion');
const browser = await chromium.launch({ executablePath: process.env.CSTIMER_BROWSER || '/etc/profiles/per-user/zhenyu/bin/brave', headless: true, args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
let xvfb;
try {
 const page = await browser.newPage();
 await page.goto(url);
 await page.waitForFunction(() => window.localInterface && window.giikerutil);
 await page.evaluate(async () => {
  const solution = "R2 F2 D2 L2 B2 R U R' U' F R U R' U' F' R U' R U R U R U' R' U' R2".split(' ');
  const scramble = solution.slice().reverse().map(m => m.endsWith('2') ? m : m.endsWith("'") ? m.slice(0, -1) : m + "'").join(' ');
  const timed = solution.map((m, i) => m + '@' + Math.round(i * 12440 / (solution.length - 1))).join(' ');
  kernel.pushSignal('time', ['Synthetic native capture', scramble, [0, 12440], 1700000000, [timed, '333']]);
  GiikerCube.isConnected = () => true;
  GiikerCube.getCube = () => ({ getBatteryLevel: () => Promise.resolve([80, 'Test cube']) });
  await giikerutil.init();
  GiikerCube.callback(mathlib.SOLVED_FACELET, [], [1000, Date.now()], 'Test cube', '333');
 });
 const owner = await page.evaluate(() => window.localInterfaceId);
 let snapshot;
 for (let i = 0; i < 40; i++) {
  snapshot = await (await fetch(url + '/api/companion/state')).json();
  if (snapshot.owner === owner && snapshot.connected && snapshot.time === '12.44') break;
  await new Promise(resolve => setTimeout(resolve, 250));
 }
 if (snapshot.owner !== owner || !snapshot.connected || snapshot.time !== '12.44') throw new Error('Isolated timer did not publish its fixture');
 const display = ':' + (180 + process.pid % 40);
 xvfb = spawn('Xvfb', [display, '-screen', '0', '800x600x24', '-nolisten', 'tcp'], { stdio: 'ignore' });
 await new Promise((resolve, reject) => { xvfb.once('error', reject); setTimeout(resolve, 700); });
 const output = path.join(out, 'native-companion.png');
 const child = spawn(binary, ['--screenshot', output], { env: { ...process.env, DISPLAY: display, LIBGL_ALWAYS_SOFTWARE: '1', CSTIMER_URL: url }, stdio: ['ignore', 'pipe', 'pipe'] });
 let logs = ''; child.stderr.on('data', data => { logs += data; });
 const timeout = setTimeout(() => child.kill('SIGTERM'), 15000);
 const code = await new Promise((resolve, reject) => { child.once('error', reject); child.once('exit', resolve); });
 clearTimeout(timeout);
 if (code !== 0) throw new Error('Native capture failed: ' + logs);
 await access(output);
 await writeFile(path.join(out, 'native-companion.json'), JSON.stringify({ synthetic: true, native: 'Rust / eframe', snapshot, screenshot: output }, null, 2));
 console.log('Real Rust companion captured from live local bridge: ' + output);
} finally { if (xvfb) xvfb.kill('SIGTERM'); await browser.close(); }
