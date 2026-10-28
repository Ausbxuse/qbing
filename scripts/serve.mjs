import http from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createCompanionBridge } from './companion-bridge.mjs';
const root = fileURLToPath(new URL('../.local-build/', import.meta.url));
await stat(path.join(root, 'index.html')).catch(() => { throw new Error('Build first: nix develop --command npm run build'); });
const types = { '.html':'text/html; charset=utf-8', '.js':'text/javascript; charset=utf-8', '.css':'text/css; charset=utf-8', '.json':'application/json', '.webmanifest':'application/manifest+json', '.png':'image/png', '.svg':'image/svg+xml', '.ttf':'font/ttf', '.woff2':'font/woff2' };
const port = Number(process.env.CSTIMER_PORT || 3327);
const bridge = createCompanionBridge();
const server = http.createServer(async (req, res) => {
  try {
    const url = new URL(req.url, 'http://localhost');
    if (await bridge.handle(req, res, url.pathname)) return;
    if (req.method !== 'GET' && req.method !== 'HEAD') { res.writeHead(405).end(); return; }
    if (url.pathname === '/.local-health') { res.setHeader('Content-Type', 'application/json'); res.end(JSON.stringify({ app: 'cstimer-local' })); return; }
    const filename = path.resolve(root, '.' + decodeURIComponent(url.pathname === '/' ? '/index.html' : url.pathname));
    if (path.relative(root, filename).startsWith('..')) { res.writeHead(403).end(); return; }
    const data = await readFile(filename);
    res.setHeader('Content-Type', types[path.extname(filename)] || 'text/plain; charset=utf-8');
    res.setHeader('Cache-Control', 'no-cache');
    res.setHeader('X-Content-Type-Options', 'nosniff');
    res.writeHead(200).end(req.method === 'HEAD' ? undefined : data);
  } catch { res.writeHead(404).end('Not found'); }
});
server.listen(port, '127.0.0.1', () => console.log(`csTimer Local: http://localhost:${port}`));
