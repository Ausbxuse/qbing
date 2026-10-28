import {chromium} from 'playwright-core';
import {mkdir} from 'node:fs/promises';
const out=new URL('../design/interface-preview/palette-shots/',import.meta.url).pathname;await mkdir(out,{recursive:true});
const browser=await chromium.launch({executablePath:'/etc/profiles/per-user/zhenyu/bin/brave',headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
try{const page=await browser.newPage({viewport:{width:1440,height:1000}});for(const palette of ['lagoon','ember','orchid','cobalt','graphite','charcoal','stone']){
 await page.goto('http://localhost:3328/redesign.html?embed=1&palette='+palette);await page.waitForFunction(()=>document.querySelector('#cube-canvas')?.dataset.ready==='true'&&document.body.dataset.studyCube);await page.evaluate(()=>document.fonts.ready);await page.evaluate(()=>window.dispatchEvent(new Event('local-cube-redraw')));await page.waitForTimeout(150);await page.screenshot({path:out+palette+'.png'});
}console.log('Captured seven palette directions.');}finally{await browser.close();}
