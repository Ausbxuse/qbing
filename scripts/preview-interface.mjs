// Isolated visual proposal. No access to the timer's storage or Bluetooth.
import http from 'node:http';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../design/interface-preview/', import.meta.url));
const fonts = fileURLToPath(new URL('../src/fonts/', import.meta.url));
const mime = { '.html': 'text/html; charset=utf-8', '.css': 'text/css; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.png': 'image/png', '.svg': 'image/svg+xml', '.ttf': 'font/ttf', '.woff2': 'font/woff2' };
const port = Number(process.env.CSTIMER_PREVIEW_PORT || 3328);
http.createServer(async (req, res) => {
  if (!['GET', 'HEAD'].includes(req.method)) { res.writeHead(405).end(); return; }
  try {
    const url = new URL(req.url, 'http://localhost');
    if (url.pathname === '/.preview-health') { res.setHeader('Content-Type', 'application/json'); res.end('{"app":"cstimer-interface-preview"}'); return; }
    const isFont = url.pathname.startsWith('/fonts/');
    const base = isFont ? fonts : root;
    const relative = decodeURIComponent(isFont ? url.pathname.slice(7) : url.pathname === '/' ? 'index.html' : url.pathname.slice(1));
    const filename = path.resolve(base, relative);
    if (path.relative(base, filename).startsWith('..')) { res.writeHead(403).end(); return; }
    const data = await readFile(filename);
    res.setHeader('Content-Type', mime[path.extname(filename)] || 'application/octet-stream');
    res.setHeader('Cache-Control', 'no-store');
    res.setHeader('X-Content-Type-Options', 'nosniff');
    res.writeHead(200).end(req.method === 'HEAD' ? undefined : data);
  } catch { res.writeHead(404).end('Not found'); }
}).listen(port, '127.0.0.1', () => console.log(`Interface preview: http://localhost:${port}`));
