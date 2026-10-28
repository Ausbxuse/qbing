import { spawn } from 'node:child_process';
import { mkdir, open } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import os from 'node:os';
import path from 'node:path';
const root = fileURLToPath(new URL('..', import.meta.url));
const port = Number(process.env.CSTIMER_PORT || 3327);
const url = `http://localhost:${port}`;
async function running() {
  try { return (await (await fetch(`http://127.0.0.1:${port}/.local-health`, { signal: AbortSignal.timeout(700) })).json()).app === 'cstimer-local'; }
  catch { return false; }
}
if (!await running()) {
  await mkdir(path.join(root, '.data'), { recursive: true });
  const log = await open(path.join(root, '.data/server.log'), 'a', 0o600);
  const server = spawn(process.execPath, [path.join(root, 'scripts/serve.mjs')], { cwd: root, env: process.env, detached: true, stdio: ['ignore', log.fd, log.fd] });
  server.unref();
  await log.close();
  for (let i = 0; i < 30 && !await running(); i++) await new Promise(resolve => setTimeout(resolve, 100));
  if (!await running()) throw new Error('Could not start csTimer; see .data/server.log');
}
console.log(url);
if (!process.argv.includes('--serve-only')) {
  const debugPort = Number(process.env.CSTIMER_DEBUG_PORT || 3329);
  // Activate this dedicated profile's existing app window before opening one.
  // The widget can therefore bring back the timer without another BLE owner.
  let activated = false;
  try {
    const targets = await (await fetch(`http://127.0.0.1:${debugPort}/json/list`, { signal: AbortSignal.timeout(500) })).json();
    const target = targets.find(tab => tab.type === 'page' && (tab.url === url || tab.url.startsWith(url + '/')));
    if (target) activated = (await fetch(`http://127.0.0.1:${debugPort}/json/activate/${encodeURIComponent(target.id)}`, { signal: AbortSignal.timeout(500) })).ok;
  } catch (_) {}
  if (activated) process.exit(0);
  const profile = path.join(process.env.XDG_DATA_HOME || path.join(os.homedir(), '.local/share'), 'cstimer-local/browser');
  await mkdir(profile, { recursive: true, mode: 0o700 });
  const browser = spawn(process.env.CSTIMER_BROWSER || 'brave', ['--enable-experimental-web-platform-features', '--disable-background-timer-throttling', '--disable-renderer-backgrounding', `--remote-debugging-port=${debugPort}`, '--remote-debugging-address=127.0.0.1', `--user-data-dir=${profile}`, `--app=${url}`], { detached: true, stdio: 'ignore' });
  browser.on('error', error => { console.error(`Could not launch browser: ${error.message}. Open ${url} in Chrome or Brave.`); process.exitCode = 1; });
  browser.unref();
}
