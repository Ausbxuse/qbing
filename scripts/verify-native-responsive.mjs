import {readFile,writeFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {decodePng,coloredPixels} from './native-png.mjs';
const root=new URL('../.data/native-parity/',import.meta.url),results=[];
for(const size of ['360x400','450x560','640x480','800x600','960x540','960x1080','1080x720','1440x540'])for(const theme of ['paper','carbon']){
 const file=`${size}-result-1x-${theme}`,m=JSON.parse(await readFile(new URL(file+'.json',root),'utf8')),b=m.bounds;
 const [w,h]=size.split('x').map(Number),mobile=w<=760;
 const rect=key=>{assert.ok(b[key],file+key);const [x,y,width,height]=b[key];return {x,y,width,height,right:x+width,bottom:y+height}};
 if(b['.history-pane']){const rail=rect('.history-pane'),content=rect('.history-rail-content');assert.ok(content.x-rail.x>=16&&rail.right-content.right>=16,file+': rail keeps consistent side insets');}
 const cube=rect('.cube-stage'),clock=rect('.clock-anchor'),scramble=rect('.scramble-region'),phase=rect('.phase-section');
 for(const r of [cube,clock,scramble,phase]){assert.ok(r.x>=0&&r.y>=0&&r.right<=w+.1&&r.bottom<=h-(mobile?48:0)+.1,file+': content stays in workspace');}
 assert.ok(cube.y>=scramble.bottom-.1,file+': scramble stays above cube');
 assert.ok(clock.y>=cube.bottom-.1,file+': cube stays above clock');
 assert.ok(phase.y>=clock.bottom+26,file+': clock metrics stay above dock');
 if(b['.average-table']){const a=rect('.average-table');assert.ok(a.bottom<=phase.y,file+': averages stay above dock');}
 const image=decodePng(await readFile(new URL(file+'.png',root)));assert.equal(image.width,w);assert.equal(image.height,h);const pixels=coloredPixels(image,b['.cube-stage']);assert.ok(pixels>40,file+': visible colored cube');
 assert.ok(m.gpu.triangles<60000&&m.gpu.meshDraws<=2,file+': static cube remains cached');
 results.push({size,theme,cubePixels:pixels,railsVisible:!!b['.history-pane'],clockFontRegion:b['.clock-anchor']});
}
for(const size of ['360x400','450x560','960x540']){const m=JSON.parse(await readFile(new URL(`${size}-history-1x-carbon.json`,root),'utf8'));assert.ok(m.bounds['.history-list'][3]>=40,size+': short history retains a scrollable list');}
await writeFile(new URL('responsive-checks.json',root),JSON.stringify({runtime:'Real eframe/Glow windows resized from 1440×1000 on isolated Xvfb',results},null,2));console.log(`Responsive screenshots verified: ${results.length} resized native windows.`);
