// Publish the actual renderer into the isolated design preview, without BLE.
import { cp, mkdir, readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('../', import.meta.url));
const preview = root + 'design/interface-preview/';
await mkdir(preview + 'js', { recursive: true });
for (const name of ['local-cube-view.js', 'local-cube-shaders.js', 'local-cube-geometry.js', 'local-cube-motion.js']) {
 await cp(root + 'src/js/' + name, preview + 'js/' + name);
}
await cp(root + 'src/vendor/three', preview + 'vendor/three', { recursive: true });
await cp(root + 'src/assets', preview + 'assets', { recursive: true });
// The detail view changes only camera framing, retaining the production mesh,
// materials and the same texture. It is never loaded by the timer.
const centerView = (await readFile(root + 'src/js/local-cube-view.js', 'utf8'))
 .replace('new THREE.PerspectiveCamera(25,', 'new THREE.PerspectiveCamera(14,')
 .replace(/camera.position.set\(6\.2,\s*4\.8,\s*9\.1\);/, 'camera.position.set(0, 8, 0); camera.up.set(0, 0, -1);')
 .replace(/camera.lookAt\(0,\s*0,\s*0\);/, 'camera.lookAt(0, 1.5, 0);');
await writeFile(preview + 'js/local-cube-center.js', centerView);
for (const [source, name] of [['src/js/lib/mathlib.js', 'mathlib.js'], ['src/js/lib/isaac.js', 'isaac.js'], ['dist/js/jquery.min.js', 'jquery.min.js']]) {
 await cp(root + source, preview + 'js/' + name);
}
await writeFile(preview + 'cube-surface.html', `<!doctype html>
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Cartoon cube preview</title>
<style>
@font-face{font-family:Maple;src:url(fonts/MapleMono-UI-Regular.woff2)}
*{box-sizing:border-box}body{margin:0;background:#17191d;color:#e4e3ef;font:14px/1.5 Maple,monospace}
main{width:min(760px,100%);margin:36px auto;padding:0 24px}header{padding-bottom:16px;border-bottom:1px solid #ffffff15}
p{font-size:12px;color:#adb1c7;margin:6px 0}.cube-stage{position:relative;width:min(520px,100%);height:440px;margin:12px auto 0}
canvas{display:block;width:100%;height:100%;touch-action:none;cursor:grab}canvas:active{cursor:grabbing}
button{font:inherit;color:#adb1c7;background:transparent;border:1px solid #ffffff18;padding:6px 12px;cursor:pointer}
button:hover,button:focus-visible{color:#b4befe;border-color:#b4befe}.cube-reset{position:absolute;right:8px;bottom:12px}
footer{border-top:1px solid #ffffff15;padding-top:12px;display:flex;flex-wrap:wrap;gap:8px;align-items:center}a{color:#b4befe;font-size:12px;margin-left:auto}
@media(max-width:520px){main{margin-top:20px}.cube-stage{height:330px}}
</style>
<script type="importmap">{"imports":{"three":"./vendor/three/three.module.min.js"}}</script>
<script>var DEBUG=false;</script><script src="js/jquery.min.js"></script><script src="js/isaac.js"></script><script src="js/mathlib.js"></script>
<main><header>Cartoon cube preview<p>Actual app renderer · rounded pieces and matte toon shading</p></header>
<figure class="cube-stage"><canvas id="cube-canvas" aria-label="Interactive cube"></canvas><button id="cube-reset" class="cube-reset">Reset view</button></figure>
<footer><button data-move="R">Turn R</button><button data-move="U">Turn U</button><button id="backdrop">White backdrop</button><p id="surface-status">Loading surface…</p><a href="cube-center.html">Inspect center</a></footer></main>
<script type="module">
import './js/local-cube-view.js';
document.querySelector('#surface-status').textContent='Rounded caps · matte colors · banded light';
const reviewCube=new mathlib.CubieCube();
document.querySelectorAll('[data-move]').forEach(button=>button.onclick=()=>{
 const move=button.dataset.move;reviewCube.selfMoveStr(move);
 window.dispatchEvent(new CustomEvent('local-cube-state',{detail:{facelets:reviewCube.toFaceCube(),moves:[move]}}));
});
let white=false;
document.querySelector('#backdrop').onclick=event=>{white=!white;document.querySelector('.cube-stage').style.background=white?'#fff':'transparent';event.target.textContent=white?'App backdrop':'White backdrop';};
</script>`);
const centerPage = (await readFile(preview + 'cube-surface.html', 'utf8'))
 .replaceAll('Cartoon cube preview', 'Center cap detail')
 .replace('Actual app renderer · rounded pieces and matte toon shading', 'Production model · camera framed directly above the white center')
 .replace("import './js/local-cube-view.js'", "import './js/local-cube-center.js'")
 .replace('<a href="cube-center.html">Inspect center</a>', '<a href="cube-surface.html">Whole cube</a>');
await writeFile(preview + 'cube-center.html', centerPage);
console.log('Surface preview: http://localhost:3328/cube-surface.html');
