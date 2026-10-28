import { spawnSync } from 'node:child_process';
import { mkdir, readFile, writeFile, cp, readdir, rm } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = fileURLToPath(new URL('..', import.meta.url));
const out = path.join(root, '.local-build');
function run(command, args, capture = false) {
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', stdio: capture ? 'pipe' : 'inherit' });
  if (result.error || result.status !== 0) throw new Error(result.error?.message || result.stderr || `${command} failed`);
  return result.stdout;
}
await mkdir(out, { recursive: true });
run('make', ['dist/js/cstimer.js', 'dist/css/style.css']);
// Render upstream's source template, which contains no production analytics.
let html = run('php', ['-d', 'include_path=src', 'src/index.php'], true);
let inserted = false;
html = html.replace(/<script type="text\/javascript" src="[^"]+"><\/script>/g, () => {
  if (inserted) return '';
  inserted = true;
  return '<script src="js/local-defaults.js"></script>\n<script src="js/jquery.min.js"></script>\n<script src="js/cstimer.js"></script>';
});
html = html.replace('width=500, user-scalable=no', 'width=device-width, initial-scale=1, viewport-fit=cover');
html = html.replace('</head>', '<link rel="stylesheet" href="css/local.css">\n<link rel="stylesheet" href="css/local-metrics.css">\n<link id="local-interface-css" rel="stylesheet" href="css/local-interface.css">\n<script type="importmap">{"imports":{"three":"./vendor/three/three.module.min.js"}}</script>\n<script src="js/local-appearance.js"></script>\n<script src="js/local-metrics.js"></script>\n<script src="js/local-scramble-guide.js"></script>\n<script src="js/local-action-menu.js"></script>\n<script src="js/local-review-data.js"></script>\n<script src="js/local-solve-tags.js"></script>\n<script src="js/local-review-tools.js"></script>\n<script src="js/local-interface.js"></script>\n</head>');
html = html.replace('</body>', (await readFile(path.join(root, 'src/interface/app.html'), 'utf8')) + '</body>');
html = html.replace(/<title>.*?<\/title>/, '<title>Qbing</title>');
if (!inserted || html.includes('<?php')) throw new Error('Unexpected upstream HTML template');
await writeFile(path.join(out, 'index.html'), html);
for (const [source, dest] of [
  ['dist/js/cstimer.js', 'js/cstimer.js'], ['dist/js/twisty.js', 'js/twisty.js'],
  ['dist/js/jquery.min.js', 'js/jquery.min.js'], ['src/css/style.css', 'css/style.css'],
  ['src/css/local.css', 'css/local.css'], ['src/css/local-metrics.css', 'css/local-metrics.css'],
  ['src/js/local-defaults.js', 'js/local-defaults.js'], ['src/js/local-appearance.js', 'js/local-appearance.js'],
  ['src/js/local-solve-tags.js', 'js/local-solve-tags.js'], ['src/js/local-review-tools.js', 'js/local-review-tools.js'], ['src/js/local-cross-worker.js', 'js/local-cross-worker.js'],
  ['src/js/lib/isaac.js', 'vendor/solver/isaac.js'], ['src/js/lib/mathlib.js', 'vendor/solver/mathlib.js'], ['src/js/tools/cross.js', 'vendor/solver/cross.js'],
  ['src/js/local-action-menu.js', 'js/local-action-menu.js'], ['src/js/local-review-data.js', 'js/local-review-data.js'], ['src/js/local-metrics.js', 'js/local-metrics.js'], ['src/fonts', 'fonts'],
  ['src/js/local-scramble-guide.js', 'js/local-scramble-guide.js'], ['src/js/local-interface.js', 'js/local-interface.js'], ['src/js/local-cube-view.js', 'js/local-cube-view.js'],
  ['src/js/local-cube-motion.js', 'js/local-cube-motion.js'],
  ['src/js/local-cube-geometry.js', 'js/local-cube-geometry.js'],
  ['src/css/local-interface.css', 'css/local-interface.css'], ['src/vendor', 'vendor'],
  ['src/assets', 'assets'],
  ['src/interface/companion.html', 'companion.html'], ['src/css/companion.css', 'css/companion.css'],
  ['src/js/companion.js', 'js/companion.js'],
  ['src/cstimer.webmanifest', 'cstimer.webmanifest'], ['dist/cstimer512x512.png', 'cstimer512x512.png'],
  ['LICENSE', 'LICENSE']
]) {
  await mkdir(path.dirname(path.join(out, dest)), { recursive: true });
  await cp(path.join(root, source), path.join(out, dest), { recursive: true, filter: filename => {
    const name = path.basename(filename);
    return !/\.hdr$/i.test(name) && !['Geist-Light.ttf', 'JetBrainsMono-Regular.ttf', 'RGBELoader.js'].includes(name);
  } });
}
// Remove only known obsolete generated assets; their licensed sources remain.
for (const obsolete of ['assets/studio_small_09_1k.hdr','fonts/Geist-Light.ttf','fonts/JetBrainsMono-Regular.ttf','vendor/three/RGBELoader.js','js/local-cube-shaders.js']) await rm(path.join(out, obsolete), {force:true});
await writeFile(path.join(out, 'build.json'), JSON.stringify({ upstream: run('git', ['rev-parse', 'upstream/master'], true).trim(), builtAt: new Date().toISOString() }, null, 2));
// Fingerprint code URLs before publishing HTML. An older cache-first worker
// must not pair new HTML with the previous interface/renderer implementation.
const assets = (await readdir(out, { recursive: true, withFileTypes: true }))
  .filter(entry => entry.isFile() && !['sw.js', 'build.json'].includes(entry.name))
  .map(entry => path.relative(out, path.join(entry.parentPath, entry.name))).sort();
const hash = createHash('sha256').update('local-code-urls-v1\0');
for (const asset of assets) hash.update(asset).update(await readFile(path.join(out, asset)));
const revision = hash.digest('hex').slice(0, 20);
for (const asset of assets) {
  if (!asset.endsWith('.js')) continue;
  const file = path.join(out, asset);
  const source = await readFile(file, 'utf8');
  await writeFile(file, source.replace(/(['"])((?:\.\.?\/|js\/)[^'"\s?]+\.js)\1/g,
    (_, quote, url) => quote + url + '?build=' + revision + quote));
}
for (const asset of assets) {
  if (!asset.endsWith('.html') || asset === 'index.html') continue;
  const file = path.join(out, asset), markup = await readFile(file, 'utf8');
  await writeFile(file, markup.replace(/((?:src|href)="[^"?]+\.(?:js|css))"/g, '$1?build=' + revision + '"'));
}
html = html.replace(/((?:src|href)="[^"?]+\.(?:js|css))"/g, '$1?build=' + revision + '"')
  .replace(/("three":"[^"?]+\.js)"/, '$1?build=' + revision + '"');
await writeFile(path.join(out, 'index.html'), html);
const cachedAssets = assets.map(asset => /\.(?:js|css)$/.test(asset) ? asset + '?build=' + revision : asset);
const worker = (await readFile(path.join(root, 'dist/sw.js'), 'utf8'))
  .replace("self.addEventListener('fetch', function(event) {", "self.addEventListener('fetch', function(event) {\n\tif (new URL(event.request.url).pathname.startsWith('/api/')) return;\n\tif (event.request.mode === 'navigate') {\n\t\tevent.respondWith(fetch(event.request).catch(function() { return caches.match('index.html'); }));\n\t\treturn;\n\t}")
  .replace(/var urlsToCache = \[[\s\S]*?\];/, 'var urlsToCache = ' + JSON.stringify(['.', ...cachedAssets]) + ';')
  .replace(/var CACHE_NAME = "[^"]*";/, 'var CACHE_NAME = "cstimer_local_' + revision + '";');
await writeFile(path.join(out, 'sw.js'), worker);
console.log(`Built upstream csTimer with local extensions: ${out}`);
