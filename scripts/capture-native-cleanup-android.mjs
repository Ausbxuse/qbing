// Actual native Compose screenshots, using only the debug in-memory profile.
import {spawnSync} from 'node:child_process';
import {writeFile} from 'node:fs/promises';
import path from 'node:path';
const root=path.resolve(new URL('..',import.meta.url).pathname);
const serial=process.env.ANDROID_SERIAL||'emulator-5580';
if(!serial.startsWith('emulator-'))throw Error('An isolated emulator is required');
const adb=path.join(process.env.HOME,'Android/Sdk/platform-tools/adb');
function run(args,binary=false){const r=spawnSync(adb,['-s',serial,...args],{encoding:binary?null:'utf8',timeout:20000,maxBuffer:12*1024*1024});if(r.status!==0)throw Error(String(r.stderr||r.error));return r.stdout;}
const pause=ms=>new Promise(r=>setTimeout(r,ms));
run(['install','-r',path.join(root,'native/androidApp/build/outputs/apk/debug/androidApp-debug.apk')]);
for(const [name,size,theme,view] of [['settings-phone','390x844','carbon','settings'],['settings-wide','1440x1000','paper','settings'],['history-phone','390x844','carbon','history'],['history-small','450x560','carbon','history']]){
 run(['shell','wm','size',size]);run(['shell','wm','density','160']);
 run(['shell','am','force-stop','org.cstimer.nativeapp']);
 run(['shell','run-as','org.cstimer.nativeapp','rm','-f','files/parity-ready']);
 run(['shell','am','start','-S','-n','org.cstimer.nativeapp/lab.cube.android.ParityActivity','--es','theme',theme,'--es','view',view]);
 for(let i=0;i<80;i++){let ready=false;try{ready=run(['exec-out','run-as','org.cstimer.nativeapp','cat','files/parity-ready']).trim()==='true';}catch{}if(ready)break;if(i===79)throw Error('Fixture initialization timed out');await pause(300);}
 await pause(700);
 let xml='';
 for(let attempt=0;attempt<5;attempt++){const output=run(['shell','uiautomator','dump','--compressed','/sdcard/cstimer-cleanup.xml']);if(output.includes('dumped to:')){xml=run(['exec-out','cat','/sdcard/cstimer-cleanup.xml']);break;}await pause(350);}
 if(!xml.startsWith('<?xml'))throw Error('Accessibility hierarchy did not become idle');
 for(const label of view==='settings'?['Settings','Appearance','Practice']:['Best','Mean','ao5','ao12'])if(!xml.includes('text="'+label+'"'))throw Error('Missing '+label+' in '+name);
 await writeFile(path.join(root,'.data/native-parity/android-'+name+'.png'),run(['exec-out','screencap','-p'],true));
 console.log('Captured native Compose '+name);
}
const log=run(['logcat','-d','-s','AndroidRuntime:E']);if(log.includes('FATAL EXCEPTION'))throw Error(log);
