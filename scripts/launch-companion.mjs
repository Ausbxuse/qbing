import { spawn } from 'node:child_process';
import { readFile, access } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import os from 'node:os';
import path from 'node:path';
const root = fileURLToPath(new URL('..', import.meta.url));
const data = process.env.XDG_DATA_HOME || path.join(os.homedir(), '.local/share');
const installed = path.join(data, 'cstimer-local');
const config = JSON.parse(await readFile(path.join(installed, 'companion-runtime.json'), 'utf8'));
const binary = path.join(installed, 'bin/cstimer-companion');
const url = process.env.CSTIMER_URL || `http://localhost:${process.env.CSTIMER_PORT || 3327}`;
await access(binary);
if (!process.argv.includes('--from-main')) {
 let online = false;
 try { online = (await (await fetch(url + '/api/companion/state', { signal: AbortSignal.timeout(750) })).json()).online; } catch (_) {}
 if (!online) {
  const main = spawn(process.execPath, [path.join(root, 'scripts/launch.mjs')], { detached: true, stdio: 'ignore' }); main.unref();
 }
}
const child = spawn(binary, process.argv.slice(2).filter(arg => arg !== '--from-main'), {
 detached: true, stdio: 'ignore', env: { ...process.env, LD_LIBRARY_PATH: config.libraries, CSTIMER_URL: url,
  CSTIMER_LAUNCHER: path.join(os.homedir(), '.local/bin/cstimer-local') }
});
child.on('error', error => { console.error(error.message); process.exitCode = 1; });
child.unref();
