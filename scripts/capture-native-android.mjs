import {spawnSync} from 'node:child_process';import {writeFile,mkdir,readFile} from 'node:fs/promises';import path from 'node:path';
const root=path.resolve(new URL('..',import.meta.url).pathname),out=path.join(root,'.data/native-parity'),serial=process.env.ANDROID_SERIAL||'emulator-5580';
if(!serial.startsWith('emulator-'))throw Error('This workflow only accepts an isolated emulator.');
const adb=path.join(process.env.HOME,'Android/Sdk/platform-tools/adb');
const run=(args,encoding='utf8')=>{const result=spawnSync(adb,['-s',serial,...args],{encoding,maxBuffer:10*1024*1024,timeout:15000});if(result.status!==0)throw Error(String(result.stderr||result.error));return result.stdout;};
const pause=ms=>new Promise(r=>setTimeout(r,ms));await mkdir(out,{recursive:true});
run(['install','-r',path.join(root,'native/androidApp/build/outputs/apk/debug/androidApp-debug.apk')]);run(['shell','wm','size','390x844']);run(['shell','wm','density','160']);run(['shell','settings','put','secure','immersive_mode_confirmations','confirmed']);run(['logcat','-c']);
async function tree(){
 for(let attempt=0;attempt<4;attempt++){
  const output=run(['shell','uiautomator','dump','--compressed','/sdcard/cstimer-window.xml']);
  if(output.includes('dumped to:'))return run(['exec-out','cat','/sdcard/cstimer-window.xml']);
  await pause(300);
 }
 throw Error('Android accessibility hierarchy did not become idle; refusing stale XML.');
}
async function tapLabel(label){const xml=await tree();const nodes=xml.match(/<node\b[^>]*>/g)||[];const node=nodes.find(n=>n.includes('text="'+label+'"')||n.includes('text="▷ '+label+'"')||n.includes('content-desc="'+label+'"'));if(!node)throw Error('Missing native control: '+label);const m=/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/.exec(node);run(['shell','input','tap',String((+m[1]+ +m[3])/2|0),String((+m[2]+ +m[4])/2|0)]);await pause(250);}
for(const[name,theme,view]of [['desktop-light','paper','timer'],['landscape','paper','timer'],['small-window','carbon','timer'],['history-small','carbon','history'],['phone-light','paper','timer'],['phone-dark','carbon','timer'],['history-phone','carbon','history']]){
 run(['shell','wm','size',name==='desktop-light'?'1440x1000':name==='landscape'?'800x600':name==='small-window'||name==='history-small'?'450x560':'390x844']);
 run(['shell','am','start','-S','-n','org.cstimer.nativeapp/lab.cube.android.ParityActivity','--es','theme',theme,'--es','view',view]);for(let tries=0;tries<90;tries++){let ready=false;try{ready=run(['exec-out','run-as','org.cstimer.nativeapp','cat','files/parity-ready']).trim()==='true';}catch{}if(ready)break;if(tries===89)throw Error('Synthetic controller did not finish initialization');await pause(500);}await pause(500);const xml=await tree();if(!xml.includes('13.73'))throw Error(name+': missing fixture data or failed startup');await writeFile(out+'/'+'android-'+name+'.png',run(['exec-out','screencap','-p'],null));await writeFile(out+'/android-'+name+'.json',run(['exec-out','run-as','org.cstimer.nativeapp','cat','files/parity-bounds.json']));
 if(name==='history-small'){await tapLabel('Distribution ↗');await pause(500);const stats=await tree();if(!stats.includes('Session statistics'))throw Error('Compact history statistics did not open');await writeFile(out+'/android-small-statistics.png',run(['exec-out','screencap','-p'],null));run(['shell','input','keyevent','4']);}

}
// Real native dialog/tag/replay interactions, against in-memory synthetic records.
await tapLabel('Timer');await tapLabel('Tag solve');await writeFile(out+'/android-tags.png',run(['exec-out','screencap','-p'],null));await tapLabel('Tag color blue');await tapLabel('Review');await tapLabel('Done');await tapLabel('Replay');await writeFile(out+'/android-replay.png',run(['exec-out','screencap','-p'],null));run(['shell','input','keyevent','4']);await pause(200);
// Selecting an older solve must not redirect the latest-solve tool review.
await tapLabel('History');await tapLabel('13.56');await tapLabel('Tools');
let toolTree='';for(let attempt=0;attempt<12;attempt++){await pause(500);toolTree=await tree();if(toolTree.includes('Yellow · Xcross'))break;}
if(!toolTree.includes('Yellow · Xcross'))throw Error('Current white/yellow cross dashboard did not load');
await writeFile(out+'/android-tools-current.png',run(['exec-out','screencap','-p'],null));
run(['shell','input','swipe','195','580','195','260','350']);await pause(350);toolTree=await tree();
for(let attempt=0;attempt<3&&!toolTree.includes('Last solve · #20');attempt++){run(['shell','input','swipe','195','580','195','260','350']);await pause(300);toolTree=await tree();}
if(!toolTree.includes('Last solve · #20'))throw Error('Tools did not keep the latest solve after history selection');
await writeFile(out+'/android-tools-last.png',run(['exec-out','screencap','-p'],null));run(['shell','input','keyevent','4']);
// Release variant has no preview activity or synthetic fixture entry point.
run(['install','-r',path.join(root,'native/androidApp/build/outputs/apk/release/androidApp-release.apk')]);run(['shell','am','start','-S','-n','org.cstimer.nativeapp/lab.cube.android.MainActivity']);await pause(7000);const releaseTree=await tree();if(!releaseTree.includes('csTimer'))throw Error('Release did not start');await writeFile(out+'/android-release.png',run(['exec-out','screencap','-p'],null));
run(['shell','pm','grant','org.cstimer.nativeapp','android.permission.BLUETOOTH_CONNECT']);run(['shell','pm','grant','org.cstimer.nativeapp','android.permission.BLUETOOTH_SCAN']);run(['shell','svc','bluetooth','disable']);await pause(500);await tapLabel('Connect Bluetooth cube');const powerTree=await tree();if(!powerTree.includes('Bluetooth is off'))throw Error('Bluetooth-off status not visible');await writeFile(out+'/android-bluetooth-off.png',run(['exec-out','screencap','-p'],null));
const logs=run(['logcat','-d','-s','AndroidRuntime:E','CubeLab:E']);if(/FATAL EXCEPTION|Cube drawing failed|Cube graphics unavailable/.test(logs))throw Error(logs);
await writeFile(out+'/android-checks.json',JSON.stringify({runtime:'Android emulator / native Compose + GLES3',serial,syntheticScreenshots:true,releaseStarts:true,tagActions:true,timedReplay:true,whiteYellowCrossDashboard:true,lastSolveIndependentOfSelection:true,crashes:0,adapterOffFeedback:true,hardwareBluetooth:false},null,2));console.log('Android debug workflows and optimized release startup passed.');
