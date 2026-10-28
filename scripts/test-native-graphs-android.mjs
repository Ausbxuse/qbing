// Native metric selectors and distributions against the isolated synthetic profile.
import{spawnSync}from'node:child_process';import{writeFile}from'node:fs/promises';import path from'node:path';
const root=path.resolve(new URL('..',import.meta.url).pathname),adb=path.join(process.env.HOME,'Android/Sdk/platform-tools/adb'),serial=process.env.ANDROID_SERIAL||'emulator-5580';if(!serial.startsWith('emulator-'))throw Error('Isolated emulator required');
function run(args,binary=false){const r=spawnSync(adb,['-s',serial,...args],{encoding:binary?null:'utf8',timeout:20000,maxBuffer:12000000});if(r.status!==0)throw Error(String(r.stderr||r.error));return r.stdout;}
const pause=ms=>new Promise(r=>setTimeout(r,ms));
async function tree(){run(['shell','uiautomator','dump','--compressed','/sdcard/facet-graphs.xml']);return run(['exec-out','cat','/sdcard/facet-graphs.xml']);}
function tap(xml,description,occurrence=0){const node=(xml.match(/<node\b[^>]*>/g)||[]).filter(n=>n.includes('content-desc="'+description+'"')||n.includes('text="'+description+'"'))[occurrence];const m=node&&/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/.exec(node);if(!m)throw Error('Missing '+description);const[x,y,a,b]=m.slice(1).map(Number);run(['shell','input','tap',String(Math.round((x+a)/2)),String(Math.round((y+b)/2))]);}
async function shot(name){await writeFile(path.join(root,'.data/native-parity/android-graph-'+name+'.png'),run(['exec-out','screencap','-p'],true));}
function assertMetric(xml,label,trend){if(!xml.includes('Distribution '+label+', 20 of 20 solves'))throw Error('Distribution did not switch to '+label);if(trend&&!xml.includes('Mean '+label+','))throw Error('Trend did not switch to '+label);}
async function resize(size,ready){run(['shell','wm','size',size]);await pause(600);for(let i=0;i<10;i++){const xml=await tree();if(xml.includes(ready))return xml;await pause(300);}throw Error('History layout not ready after resize '+size);}
async function state(label,trend){let xml='';for(let i=0;i<8;i++){xml=await tree();try{assertMetric(xml,label,trend);return xml;}catch{}await pause(250);}assertMetric(xml,label,trend);return xml;}
run(['install','-r',path.join(root,'native/androidApp/build/outputs/apk/debug/androidApp-debug.apk')]);run(['logcat','-c']);run(['shell','wm','size','1440x1000']);run(['shell','wm','density','160']);run(['shell','am','force-stop','org.cstimer.nativeapp']);run(['shell','run-as','org.cstimer.nativeapp','rm','-f','files/parity-ready']);run(['shell','am','start','-S','-n','org.cstimer.nativeapp/lab.cube.android.ParityActivity','--es','theme','paper','--es','view','history']);
for(let i=0;i<90;i++){let ready=false;try{ready=run(['exec-out','run-as','org.cstimer.nativeapp','cat','files/parity-ready']).trim()==='true';}catch{}if(ready)break;if(i===89)throw Error('Profile not ready');await pause(300);}await pause(500);
let xml=await tree();assertMetric(xml,'Time',true);
for(const label of ['TPS','Moves']){tap(xml,'Graph '+label);xml=await state(label,true);await shot('history-'+label.toLowerCase());}
// Physical emulator-resolution changes can recreate the debug in-memory profile.
// Persistent controller preference restoration is covered by GraphMetricTest.
xml=await resize('390x844','Graph Moves');tap(xml,'Graph Moves');xml=await state('Moves',false);await shot('phone-moves');tap(xml,'Graph TPS');xml=await state('TPS',false);await shot('phone-tps');
xml=await resize('450x560','Distribution ↗');tap(xml,'Distribution ↗');xml=await tree();tap(xml,'Graph TPS');xml=await state('TPS',false);tap(xml,'Graph Moves');xml=await state('Moves',false);await shot('compact-moves');
const logs=run(['logcat','-d','-s','AndroidRuntime:E']);if(logs.includes('FATAL EXCEPTION'))throw Error(logs);
await writeFile(path.join(root,'.data/native-parity/graph-android-checks.json'),JSON.stringify({synthetic:true,linkedTrendAndDistribution:true,metrics:['Time','TPS','Moves'],sizes:['1440x1000','390x844','450x560'],compactStatistics:true,hardwareBluetooth:false},null,2));console.log('Android metric graphs: trend, distribution, phone and compact selectors passed.');
