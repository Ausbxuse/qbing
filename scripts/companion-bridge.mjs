import { access } from 'node:fs/promises';
import { constants } from 'node:fs';
import { spawn } from 'node:child_process';
import path from 'node:path';
import os from 'node:os';

export function cleanSnapshot(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value) || typeof value.owner !== 'string' || value.owner.length > 128) throw new Error('Invalid snapshot');
  const string = (s, max) => typeof s === 'string' ? s.slice(0, max) : '';
  return { version: 1, owner: value.owner, connected: value.connected === true, device: string(value.device, 128),
    scramble: string(value.scramble, 8192),
    segments: Array.isArray(value.segments) ? value.segments.slice(0, 512).map(s => ({ text: string(s.text, 256), kind: ['done', 'current', 'correction'].includes(s.kind) ? s.kind : '' })) : [],
    time: string(value.time, 48) || '—', tps: Number.isFinite(value.tps) && value.tps >= 0 ? value.tps : null,
    moves: Number.isFinite(value.moves) && value.moves >= 0 ? value.moves : null,
    scrambleTps: Number.isFinite(value.scrambleTps) && value.scrambleTps >= 0 ? value.scrambleTps : null,
    running: value.running === true };
}
export function createCompanionBridge({ now = Date.now, openNative } = {}) {
  let latest = { connected: false, device: '', scramble: '', segments: [], time: '—', tps: null, moves: null }, written = 0;
  const state = () => ({ ...latest, updatedAt: written, online: written > 0 && now() - written < 5000,
    connected: latest.connected && written > 0 && now() - written < 5000 });
  async function handle(req, res, pathname) {
    if (!pathname.startsWith('/api/companion/')) return false;
    res.setHeader('Content-Type', 'application/json'); res.setHeader('Cache-Control', 'no-store');
    const reply = (code, data) => { res.writeHead(code).end(JSON.stringify(data)); };
    if (req.method === 'GET' && pathname === '/api/companion/state') { reply(200, state()); return true; }
    const host = req.headers.host;
    if (req.method !== 'POST' || !/^application\/json(?:;|$)/i.test(req.headers['content-type'] || '') ||
        req.headers.origin !== `http://${host}` || !/^(?:localhost|127\.0\.0\.1):\d+$/.test(host || '')) {
      reply(403, { error: 'Same-origin JSON request required' }); return true;
    }
    try {
      const input = await new Promise((resolve, reject) => {
        let text = '', size = 0;
        req.on('data', chunk => { size += chunk.length; if (size > 65536) reject(new Error('Body too large')); else text += chunk; });
        req.on('end', () => { try { resolve(JSON.parse(text)); } catch (e) { reject(e); } });
        req.on('error', reject);
      });
      if (pathname === '/api/companion/state') {
        const value = cleanSnapshot(input);
        if (written && now() - written < 3000 && latest.owner !== value.owner) { reply(409, { error: 'Another timer owns the live connection' }); return true; }
        latest = value; written = now(); reply(200, { ok: true });
      } else if (pathname === '/api/companion/open') {
        if (openNative) await openNative();
        else {
          const launcher = path.join(os.homedir(), '.local/bin/cstimer-companion');
          await access(launcher, constants.X_OK);
          await new Promise((resolve, reject) => { const child = spawn(launcher, ['--from-main'], { detached: true, stdio: 'ignore' }); child.once('spawn', () => { child.unref(); resolve(); }); child.once('error', reject); });
        }
        reply(200, { ok: true });
      } else reply(404, { error: 'Unknown endpoint' });
    } catch (error) { reply(pathname.endsWith('/open') ? 503 : 400, { error: pathname.endsWith('/open') ? 'Native companion is not installed' : 'Invalid snapshot' }); }
    return true;
  }
  return { handle, state };
}
