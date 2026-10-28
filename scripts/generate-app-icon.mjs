// The Qbing SVG is the source of truth for raster and Android exports.
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {chromium} from 'playwright-core';
const root=new URL('../',import.meta.url),assets=new URL('native/assets/app-icon/',root);
const svg=await readFile(new URL('qbing.svg',assets),'utf8');
const paths=[...svg.matchAll(/<path\s+([^>]+)\/>/g)].map(([,attributes])=>Object.fromEntries([...attributes.matchAll(/([\w-]+)="([^"]*)"/g)].map(([,key,value])=>[key,value])));
const res=new URL('native/androidApp/src/main/res/',root);
function vector(monochrome){return `<vector xmlns:android="http://schemas.android.com/apk/res/android" android:width="108dp" android:height="108dp" android:viewportWidth="128" android:viewportHeight="128"><group android:scaleX="0.8" android:scaleY="0.8" android:pivotX="64" android:pivotY="64">${paths.map(p=>`<path android:pathData="${p.d}" android:fillColor="#00000000" android:strokeColor="${monochrome?'#ffffff':p.stroke}" android:strokeWidth="${p['stroke-width']}"/>`).join('')}</group></vector>\n`;}
await writeFile(new URL('drawable/cube_icon.xml',res),vector(false));
await writeFile(new URL('drawable/cube_icon_monochrome.xml',res),vector(true));
await writeFile(new URL('values/colors.xml',res),'<resources><color name="icon_background">#1b2030</color></resources>\n');
await mkdir(new URL('mipmap-anydpi-v33/',res),{recursive:true});
await writeFile(new URL('mipmap-anydpi-v33/ic_launcher.xml',res),'<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android"><background android:drawable="@color/icon_background"/><foreground android:drawable="@drawable/cube_icon"/><monochrome android:drawable="@drawable/cube_icon_monochrome"/></adaptive-icon>\n');
const browser=await chromium.launch({executablePath:process.env.CSTIMER_BROWSER||'/etc/profiles/per-user/zhenyu/bin/brave',headless:true,args:['--no-sandbox']});
try{const page=await browser.newPage();for(const size of [32,48,64,128,256,512]){const data=await page.evaluate(async({svg,size})=>{const image=new Image();image.src='data:image/svg+xml;base64,'+btoa(svg);await image.decode();const canvas=document.createElement('canvas');canvas.width=canvas.height=size;canvas.getContext('2d').drawImage(image,0,0,size,size);return canvas.toDataURL('image/png').split(',')[1];},{svg,size});await writeFile(new URL(`qbing-${size}.png`,assets),Buffer.from(data,'base64'));}}finally{await browser.close();}
console.log('Generated Qbing desktop and Android icons.');
