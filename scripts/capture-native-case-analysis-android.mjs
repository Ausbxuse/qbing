// Real per-case tabs, backed by local synthetic measurements in the debug profile.
import {spawnSync} from 'node:child_process';import {writeFile} from 'node:fs/promises';import path from 'node:path';
const root=path.resolve(new URL('..',import.meta.url).pathname),serial=process.env.ANDROID_SERIAL||'emulator-5580';if(!serial.startsWith('emulator-'))throw Error('Isolated emulator required');
const adb=path.join(process.env.HOME,'Android/Sdk/platform-tools/adb');
function run(args,binary=false){const r=spawnSync(adb,['-s',serial,...args],{encoding:binary?null:'utf8',timeout:20000,maxBuffer:12*1024*1024});if(r.status!==0)throw Error(String(r.stderr||r.error));return r.stdout;}
const pause=ms=>new Promise(r=>setTimeout(r,ms));
async function tree(){for(let n=0;n<5;n++){const out=run(['shell','uiautomator','dump','--compressed','/sdcard/facet-analysis.xml']);if(out.includes('dumped to:'))return run(['exec-out','cat','/sdcard/facet-analysis.xml']);await pause(250);}throw Error('Case hierarchy not idle');}
async function tap(label){const xml=await tree();const node=(xml.match(/<node\b[^>]*>/g)||[]).find(n=>n.includes('text="'+label+'"'));const m=node&&/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/.exec(node);if(!m)throw Error('Missing '+label);run(['shell','input','tap',String((+m[1]+ +m[3])/2|0),String((+m[2]+ +m[4])/2|0)]);await pause(250);}
run(['install','-r',path.join(root,'native/androidApp/build/outputs/apk/debug/androidApp-debug.apk')]);run(['logcat','-c']);run(['shell','wm','size','390x844']);run(['shell','wm','density','160']);
for(const stage of ['f2l','oll','pll']){
 run(['shell','am','force-stop','org.cstimer.nativeapp']);run(['shell','run-as','org.cstimer.nativeapp','rm','-f','files/parity-ready']);
 run(['shell','am','start','-S','-n','org.cstimer.nativeapp/lab.cube.android.ParityActivity','--es','theme','carbon','--es','view','case-analysis-'+stage]);
 for(let i=0;i<90;i++){if(run(['exec-out','run-as','org.cstimer.nativeapp','cat','files/parity-ready']).trim()==='true')break;if(i===89)throw Error('Case fixture not ready');await pause(300);}await pause(500);
 await tap('Analysis');const xml=await tree();for(const label of ['Your results','12 executions','1.47','0.39'])if(!xml.includes('text="'+label+'"'))throw Error(stage+': missing '+label);
 await writeFile(path.join(root,'.data/native-parity/android-case-analysis-'+stage+'.png'),run(['exec-out','screencap','-p'],true));
 await tap('Algorithms');if(!(await tree()).includes('text="Practice this algorithm"'))throw Error('Algorithm tab did not return');
 console.log(stage+': local case analysis and algorithm tab navigation verified.');
}
const errors=run(['logcat','-d','-s','AndroidRuntime:E']);if(errors.includes('FATAL EXCEPTION'))throw Error(errors);
