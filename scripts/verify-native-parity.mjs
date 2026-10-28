import {readFile,writeFile} from 'node:fs/promises';import assert from 'node:assert/strict';import {decodePng,coloredPixels} from './native-png.mjs';
const root=new URL('..',import.meta.url),load=async p=>JSON.parse(await readFile(new URL(p,root),'utf8')),contract=await load('native/assets/design-contract.json'),results=[];
for(const[reference,file,platform]of [['desktop-light','1440x1000-result-1x-paper','rust'],['desktop-dark','1440x1000-result-1x-carbon','rust'],['laptop','1280x800-result-1x-carbon','rust'],['phone-light','390x844-result-1x-paper','rust'],['phone-dark','390x844-result-1x-carbon','rust'],['desktop-light','android-desktop-light','android'],['phone-light','android-phone-light','android'],['phone-dark','android-phone-dark','android']]){
 const ref=contract.views.find(v=>v.name===reference),metadata=await load('.data/native-parity/'+file+'.json'),bounds=platform==='rust'?metadata.bounds:metadata,checks=[];
 for(const selector of ['.app-header','.cube-stage','.clock-anchor','.average-table','.phase-section']){
  const a=bounds[selector],b=ref.components[selector];assert.ok(a,platform+' '+selector+' measured');const expected=[b.x,b.y,b.width,b.height];
  // The web pane's minimum height extends behind the fixed phone navigation.
  // Compare its visible intersection with the workspace, as native layout does.
  if(selector==='.timer-pane'&&ref.width<=760)expected[3]=Math.min(expected[3],ref.height-b.y-48);
  const delta=Math.max(...a.map((n,i)=>Math.abs(n-expected[i])));checks.push({selector,delta:Math.round(delta*100)/100,actual:a,reference:expected});
 }
 const image=decodePng(await readFile(new URL('.data/native-parity/'+file+'.png',root)));assert.equal(image.width,ref.width);assert.equal(image.height,ref.height);const count=coloredPixels(image,bounds['.cube-stage']);assert.ok(count>500,file+': actual colored cube pixels');const bg=ref.colors.bg.replace('#',''),expected=[0,2,4].map(i=>parseInt(bg.slice(i,i+2),16));assert.deepEqual(image.pixel(ref.width/2,ref.width<760?93:61),expected,file+' background');
 if(platform==='rust'){assert.ok(metadata.gpu.meshDraws<=2,'Static cube geometry stays cached across UI redraws');assert.ok(metadata.gpu.composites>metadata.gpu.meshDraws);assert.ok(metadata.gpu.triangles<60000);}
 results.push({platform,reference,checks,coloredPixels:count,gpu:metadata.gpu});
}
const report={scope:'Logical layout and actual renderer screenshots. Native and browser text rasterization can differ.',results};await writeFile(new URL('.data/native-parity/parity-checks.json',root),JSON.stringify(report,null,2));
const mismatches=results.flatMap(v=>v.checks.filter(c=>c.delta>1).map(c=>`${v.platform} ${v.reference} ${c.selector}: ${c.delta}px`));console.log(mismatches.length?'Layout differences:\n'+mismatches.join('\n'):'Native desktop and Android key layouts match within 1 logical pixel.');
if(mismatches.length)process.exitCode=1;
