// One geometric source for the desktop icon and Android adaptive foregrounds.
import {mkdir,writeFile} from 'node:fs/promises';
import {chromium} from 'playwright-core';
const root=new URL('../',import.meta.url),assets=new URL('native/assets/app-icon/',root);
await mkdir(assets,{recursive:true});
// A cut F: three flat facets, with an open seam instead of a cube illustration.
const tiles=[
 {color:'#b9c1ff',path:'M35 30L53 20V94L35 104Z'},
 {color:'#f1f3ff',path:'M57 20H97L86 39H57Z'},
 {color:'#78d8c3',path:'M57 52H87L76 71H57Z'},
];
const mark=tiles.map(t=>`<path d="${t.path}" fill="${t.color}"/>`).join('');
const svg=`<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128" role="img" aria-label="Facet"><title>Facet</title><rect x="4" y="4" width="120" height="120" rx="27" fill="#1b2030"/>${mark}</svg>`;
await writeFile(new URL('facet.svg',assets),svg+'\n');
const res=new URL('native/androidApp/src/main/res/',root);
function vector(paths){return `<vector xmlns:android="http://schemas.android.com/apk/res/android" android:width="108dp" android:height="108dp" android:viewportWidth="128" android:viewportHeight="128"><group android:scaleX="0.8" android:scaleY="0.8" android:pivotX="64" android:pivotY="64">${paths}</group></vector>\n`;}
await writeFile(new URL('drawable/cube_icon.xml',res),vector(tiles.map(t=>`<path android:pathData="${t.path}" android:fillColor="${t.color}"/>`).join('')));
await writeFile(new URL('drawable/cube_icon_monochrome.xml',res),vector(tiles.map(t=>`<path android:pathData="${t.path}" android:fillColor="#ffffff"/>`).join('')));
await writeFile(new URL('values/colors.xml',res),'<resources><color name="icon_background">#1b2030</color></resources>\n');
await mkdir(new URL('mipmap-anydpi-v33/',res),{recursive:true});
await writeFile(new URL('mipmap-anydpi-v33/ic_launcher.xml',res),'<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android"><background android:drawable="@color/icon_background"/><foreground android:drawable="@drawable/cube_icon"/><monochrome android:drawable="@drawable/cube_icon_monochrome"/></adaptive-icon>\n');
// Raster exports preserve transparency and come from the same SVG, not a redraw.
const browser=await chromium.launch({executablePath:process.env.CSTIMER_BROWSER||'/etc/profiles/per-user/zhenyu/bin/brave',headless:true,args:['--no-sandbox']});
try{const page=await browser.newPage();for(const size of [32,48,64,128,256,512]){const data=await page.evaluate(async({svg,size})=>{const image=new Image();image.src='data:image/svg+xml;base64,'+btoa(svg);await image.decode();const canvas=document.createElement('canvas');canvas.width=canvas.height=size;canvas.getContext('2d').drawImage(image,0,0,size,size);return canvas.toDataURL('image/png').split(',')[1];},{svg,size});await writeFile(new URL(`facet-${size}.png`,assets),Buffer.from(data,'base64'));}}finally{await browser.close();}
console.log('Generated desktop SVG/PNG and Android adaptive/themed icons.');
