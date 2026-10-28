// Real Compose deletion workflow, using only the isolated synthetic emulator profile.
import {spawnSync} from 'node:child_process';
import {writeFile} from 'node:fs/promises';
import path from 'node:path';
const root=path.resolve(new URL('..',import.meta.url).pathname);
const serial=process.env.ANDROID_SERIAL||'emulator-5580';
if(!serial.startsWith('emulator-'))throw Error('Isolated emulator required');
const adb=path.join(process.env.HOME,'Android/Sdk/platform-tools/adb');
function run(args,binary=false){const r=spawnSync(adb,['-s',serial,...args],{encoding:binary?null:'utf8',timeout:20000,maxBuffer:12*1024*1024});if(r.status!==0)throw Error(String(r.stderr||r.error));return r.stdout;}
const pause=ms=>new Promise(r=>setTimeout(r,ms));
async function tree(){for(let i=0;i<6;i++){const result=run(['shell','uiautomator','dump','--compressed','/sdcard/facet-delete.xml']);if(result.includes('dumped to:'))return run(['exec-out','cat','/sdcard/facet-delete.xml']);await pause(300);}throw Error('UI did not settle');}
function tap(xml,attribute,value){const node=(xml.match(/<node\b[^>]*>/g)||[]).find(n=>n.includes(`${attribute}="${value}"`));const m=node&&/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/.exec(node);if(!m)throw Error('Missing '+value);const [x,y,a,b]=m.slice(1).map(Number);run(['shell','input','tap',String(Math.round((x+a)/2)),String(Math.round((y+b)/2))]);}
run(['install','-r',path.join(root,'native/androidApp/build/outputs/apk/debug/androidApp-debug.apk')]);
run(['logcat','-c']);run(['shell','wm','size','390x844']);run(['shell','wm','density','160']);
run(['shell','am','force-stop','org.cstimer.nativeapp']);run(['shell','run-as','org.cstimer.nativeapp','rm','-f','files/parity-ready']);
run(['shell','am','start','-S','-n','org.cstimer.nativeapp/lab.cube.android.ParityActivity','--es','theme','carbon','--es','view','history']);
for(let i=0;i<90;i++){if(run(['exec-out','run-as','org.cstimer.nativeapp','cat','files/parity-ready']).trim()==='true')break;if(i===89)throw Error('Fixture not ready');await pause(300);}
await pause(500);
const before=await tree(),count=Number(/text="(\d+) solves"/.exec(before)?.[1]);if(!(count>1))throw Error('Missing fixture solves');
tap(before,'content-desc','Solve actions');tap(await tree(),'text','Delete solve…');
let xml=await tree();if(!xml.includes('Delete this solve?'))throw Error('No confirmation');
await writeFile(path.join(root,'.data/native-parity/android-delete-confirmation.png'),run(['exec-out','screencap','-p'],true));
tap(xml,'text','Cancel');xml=await tree();if(!xml.includes(`text="${count} solves"`))throw Error('Cancel changed history');
tap(xml,'content-desc','Solve actions');tap(await tree(),'text','Delete solve…');tap(await tree(),'text','Delete');await pause(500);
xml=await tree();if(!xml.includes(`text="${count-1} solves"`))throw Error('Deletion did not update history');
if(run(['logcat','-d','-s','AndroidRuntime:E']).includes('FATAL EXCEPTION'))throw Error('Android runtime crashed');
await writeFile(path.join(root,'.data/native-parity/android-delete-result.png'),run(['exec-out','screencap','-p'],true));
await writeFile(path.join(root,'.data/native-parity/android-deletion-checks.json'),JSON.stringify({synthetic:true,initialSolves:count,remainingSolves:count-1,cancelPreservesHistory:true,confirmedDeleteUpdatesHistory:true},null,2));
console.log('Android deletion: cancel preserves history; confirmed deletion removes one solve.');
