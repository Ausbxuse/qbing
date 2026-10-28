// Navigate from the displayed Best statistic in a private, in-memory Android profile.
import {spawnSync} from 'node:child_process';
import {readFile,writeFile} from 'node:fs/promises';
import path from 'node:path';
const root=path.resolve(new URL('..',import.meta.url).pathname),serial=process.env.ANDROID_SERIAL||'emulator-5580';
if(!serial.startsWith('emulator-'))throw Error('Isolated emulator required');
const adb=path.join(process.env.HOME,'Android/Sdk/platform-tools/adb');
function run(args,binary=false){const r=spawnSync(adb,['-s',serial,...args],{encoding:binary?null:'utf8',timeout:20000,maxBuffer:12*1024*1024});if(r.status!==0)throw Error(String(r.stderr||r.error));return r.stdout;}
const pause=ms=>new Promise(r=>setTimeout(r,ms));
async function tree(){for(let i=0;i<6;i++){if(run(['shell','uiautomator','dump','--compressed','/sdcard/facet-best.xml']).includes('dumped to:'))return run(['exec-out','cat','/sdcard/facet-best.xml']);await pause(300);}throw Error('UI not ready');}
function bounds(node){const m=/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/.exec(node);return m?.slice(1).map(Number);}
const fixture=JSON.parse(await readFile(path.join(root,'native/androidApp/src/debug/assets/parity-fixture.json'),'utf8'));
const scored=r=>r.penalty==='DNF'?Infinity:r.duration+(r.penalty==='+2'?2000:0);
const best=fixture.records.filter(r=>r.kind==='solve'&&r.session==='Session 1'&&Number.isFinite(scored(r))).reduce((a,b)=>scored(b)<scored(a)?b:a);
const expected=(scored(best)/1000).toFixed(2);
run(['install','-r',path.join(root,'native/androidApp/build/outputs/apk/debug/androidApp-debug.apk')]);run(['logcat','-c']);run(['shell','wm','size','390x844']);run(['shell','wm','density','160']);
run(['shell','am','force-stop','org.cstimer.nativeapp']);run(['shell','run-as','org.cstimer.nativeapp','rm','-f','files/parity-ready']);
run(['shell','am','start','-S','-n','org.cstimer.nativeapp/lab.cube.android.ParityActivity','--es','theme','carbon','--es','view','history']);
for(let i=0;i<90;i++){const ready=spawnSync(adb,['-s',serial,'exec-out','run-as','org.cstimer.nativeapp','cat','files/parity-ready'],{encoding:'utf8',timeout:5000});if(ready.status===0&&ready.stdout.trim()==='true')break;if(i===89)throw Error('Fixture not ready');await pause(250);}
let xml=await tree();const node=(xml.match(/<node\b[^>]*>/g)||[]).find(n=>n.includes('content-desc="Open best solve reconstruction"'));const b=node&&bounds(node);if(!b)throw Error('Best is not accessible');
run(['shell','input','tap',String(Math.round((b[0]+b[2])/2)),String(Math.round((b[1]+b[3])/2))]);
xml=await tree();const nodes=xml.match(/<node\b[^>]*>/g)||[];const title=nodes.find(n=>n.includes('text="Solve review"'));const t=title&&bounds(title);if(!t)throw Error('Review did not open');
const clock=nodes.find(n=>{const r=bounds(n);return n.includes('text="'+expected+'"')&&r&&Math.abs(r[0]-t[0])<=2&&r[1]>=t[3]&&r[1]<t[3]+90&&r[3]-r[1]>=30;});
if(!clock)throw Error('Review did not open the Best time '+expected);
if(run(['logcat','-d','-s','AndroidRuntime:E']).includes('FATAL EXCEPTION'))throw Error('Android runtime crashed');
await writeFile(path.join(root,'.data/native-parity/android-best-review.png'),run(['exec-out','screencap','-p'],true));
await writeFile(path.join(root,'.data/native-parity/android-best-review-checks.json'),JSON.stringify({synthetic:true,bestTime:expected,opensMatchingReview:true},null,2));
console.log('Android Best opens the matching solve review: '+expected+' s.');
