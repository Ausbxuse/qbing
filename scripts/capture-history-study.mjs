import {chromium} from 'playwright-core';
import {mkdir} from 'node:fs/promises';
const out=new URL('../design/interface-preview/history-shots/',import.meta.url).pathname;await mkdir(out,{recursive:true});
const browser=await chromium.launch({executablePath:'/etc/profiles/per-user/zhenyu/bin/brave',headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
try{const page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));for(const [palette,width,height,state,empty] of [['desktop',1440,1000,'review',false],['phone',390,844,'review',false],['small',320,740,'review',false],['dark-phone',390,844,'review',false]]){
 await page.setViewportSize({width,height});
 await page.goto('http://localhost:3328/history-study.html?embed=1&state='+state+(empty?'&session=empty':'')+(palette==='dark-phone'?'&palette=carbon':''));await page.waitForFunction(()=>document.querySelector('#cube-canvas')?.dataset.ready==='true'&&document.body.dataset.studyCube);await page.evaluate(()=>document.fonts.ready);await page.evaluate(()=>window.dispatchEvent(new Event('local-cube-redraw')));await page.waitForTimeout(150);if(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth))throw new Error('Horizontal overflow');await page.screenshot({path:out+palette+'.png'});
}if(errors.length)throw new Error(errors.join('\n'));console.log('Captured four history previews without page errors or overflow.');}finally{await browser.close();}
