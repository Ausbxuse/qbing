// Hardware-keyboard replay and grouped reconstruction in an isolated Compose profile.
import {spawnSync} from 'node:child_process';import {writeFile} from 'node:fs/promises';import path from 'node:path';
const root=path.resolve(new URL('..',import.meta.url).pathname),serial=process.env.ANDROID_SERIAL||'emulator-5580';if(!serial.startsWith('emulator-'))throw Error('Isolated emulator required');
const adb=path.join(process.env.HOME,'Android/Sdk/platform-tools/adb');
function run(args,binary=false){const r=spawnSync(adb,['-s',serial,...args],{encoding:binary?null:'utf8',timeout:20000,maxBuffer:12*1024*1024});if(r.status!==0)throw Error(String(r.stderr||r.error));return r.stdout;}
const pause=ms=>new Promise(r=>setTimeout(r,ms));
async function tree(){for(let n=0;n<5;n++){const r=run(['shell','uiautomator','dump','--compressed','/sdcard/facet-review.xml']);if(r.includes('dumped to:'))return run(['exec-out','cat','/sdcard/facet-review.xml']);await pause(300);}throw Error('Review hierarchy not idle');}
async function shot(name){await writeFile(path.join(root,'.data/native-parity/'+name+'.png'),run(['exec-out','screencap','-p'],true));}
run(['install','-r',path.join(root,'native/androidApp/build/outputs/apk/debug/androidApp-debug.apk')]);run(['logcat','-c']);run(['shell','wm','size','390x844']);run(['shell','wm','density','160']);
run(['shell','am','force-stop','org.cstimer.nativeapp']);run(['shell','run-as','org.cstimer.nativeapp','rm','-f','files/parity-ready']);run(['shell','am','start','-S','-n','org.cstimer.nativeapp/lab.cube.android.ParityActivity','--es','theme','carbon','--es','view',process.env.FACET_REVIEW_VIEW||'review']);
for(let n=0;n<90;n++){if(run(['exec-out','run-as','org.cstimer.nativeapp','cat','files/parity-ready']).trim()==='true')break;if(n===89)throw Error('Fixture not ready');await pause(300);}await pause(900);
const before=await tree();if(!/(?:text|content-desc)="Play"/.test(before))throw Error('Replay did not open');
if(process.env.FACET_CFOP_TOTALS){
 for(const total of ['0.16','1.54','0.60','0.87'])if(!before.includes('content-desc="Total '+total+' seconds"'))throw Error('Missing primary CFOP total '+total);
 if(!before.includes('content-desc="Gap 0.36 seconds, execution 1.18 seconds"'))throw Error('Missing secondary addition');
}
if(process.env.FACET_REVIEW_VIEW==='review-live-frame'&&!before.includes('◈'))throw Error('Recorded rotation rows missing');
let stepChecks=false;
if(process.env.FACET_REVIEW_VIEW==='review-analysis'){
 if(!before.includes('content-desc="Previous turn"')||!before.includes('content-desc="Next turn"'))throw Error('Missing step controls');
 run(['shell','input','keyevent','KEYCODE_DPAD_RIGHT']);run(['shell','input','keyevent','KEYCODE_DPAD_RIGHT']);
 let xml=await tree();if(!xml.includes('content-desc="Replay position 2/38"'))throw Error('Arrow right did not advance two turns');
 run(['shell','input','keyevent','KEYCODE_DPAD_LEFT']);xml=await tree();if(!xml.includes('content-desc="Replay position 1/38"'))throw Error('Arrow left did not reverse one turn');
 function tapStep(description){const node=(xml.match(/<node\b[^>]*>/g)||[]).find(n=>n.includes('content-desc="'+description+'"'));const m=node&&/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/.exec(node);if(!m)throw Error('Missing '+description);const[x,y,a,b]=m.slice(1).map(Number);run(['shell','input','tap',String(Math.round((x+a)/2)),String(Math.round((y+b)/2))]);}
 tapStep('Next turn');xml=await tree();if(!xml.includes('content-desc="Replay position 2/38"'))throw Error('Next button did not step');
 tapStep('Previous turn');xml=await tree();if(!xml.includes('content-desc="Replay position 1/38"'))throw Error('Previous button did not step');
 await shot('android-reconstruction-step');stepChecks=true;
}
run(['shell','input','keyevent','KEYCODE_SPACE']);await pause(450);run(['shell','input','keyevent','KEYCODE_SPACE']);await pause(150);
const paused=await tree();const pos=/(?:text="|content-desc="Replay position )(\d+)\/(\d+)"/.exec(paused);if(!/(?:text|content-desc)="Play"/.test(paused)||!pos||+pos[1]<=0)throw Error('Space did not play and pause');
run(['shell','input','keyevent','KEYCODE_SPACE']);await pause(400);run(['shell','input','keyevent','KEYCODE_SPACE']);await pause(150);
const resumed=await tree(),next=/(?:text="|content-desc="Replay position )(\d+)\/(\d+)"/.exec(resumed);if(!next||+next[1]<=+pos[1])throw Error('Space did not resume the playhead');
await shot('android-reconstruction-space');
// Locate the inner reconstruction list independently of screen coordinates.
let seen=new Set();for(let attempt=0;attempt<18;attempt++){
 const xml=await tree();if(!xml.includes('package="org.cstimer.nativeapp"'))throw Error('Review lost foreground focus');for(const stage of ['Cross','F2L','OLL','PLL'])if(xml.includes('text="'+stage+'"'))seen.add(stage);
 const nodes=xml.match(/<node\b[^>]*scrollable="true"[^>]*>/g)||[];
 const rects=nodes.map(n=>/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/.exec(n)).filter(Boolean).map(m=>m.slice(1).map(Number));
 const inner=rects.filter(r=>r[3]-r[1]>=60&&r[3]-r[1]<=360&&r[2]-r[0]>200).sort((a,b)=>(a[3]-a[1])-(b[3]-b[1]))[0];
 console.log('Visited', [...seen], 'scroll bounds',inner);
 if(inner){const x=Math.round((inner[0]+inner[2])/2);run(['shell','input','swipe',String(x),String(Math.min(inner[3]-15,772)),String(x),String(inner[1]+15),'300']);}
 else run(['shell','input','swipe','195','650','195','350','400']);
 await pause(300);if(seen.size===4)break;
}
await shot('android-reconstruction-steps');
function tapText(xml,value){const node=(xml.match(/<node\b[^>]*>/g)||[]).find(n=>n.includes('text="'+value+'"'));const m=node&&/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/.exec(node);if(!m)throw Error('Missing '+value);const [x,y,a,b]=m.slice(1).map(Number);run(['shell','input','tap',String(Math.round((x+a)/2)),String(Math.round((y+b)/2))]);}
tapText(await tree(),'Analysis');let analysis=await tree();if(!analysis.includes('Copy scramble'))throw Error('Analysis did not show the scramble directly');
let pairs=new Set();
if(process.env.FACET_REVIEW_VIEW==='review-analysis'){
 for(let n=0;n<60;n++){analysis=await tree();if(analysis.includes('text="Pair 1"'))break;if(n===59)throw Error('Verified F2L analysis not ready');await pause(500);}
}
await shot('android-review-analysis');
if(process.env.FACET_REVIEW_VIEW==='review-analysis'){
 for(let n=0;n<20;n++){
  analysis=await tree();for(let i=1;i<=4;i++)if(analysis.includes('text="Pair '+i+'"'))pairs.add(i);
  if(n===1)await shot('android-review-analysis-pairs');
  if(pairs.size===4)break;
  const nodes=analysis.match(/<node\b[^>]*scrollable="true"[^>]*>/g)||[];
  const bounds=nodes.map(v=>/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/.exec(v)).filter(Boolean).map(v=>v.slice(1).map(Number));
  const inner=bounds.filter(b=>b[3]-b[1]>=60&&b[3]-b[1]<=400).sort((a,b)=>(a[3]-a[1])-(b[3]-b[1]))[0];
  if(!inner)throw Error('Analysis list not scrollable');
  const x=String(Math.round((inner[0]+inner[2])/2));run(['shell','input','swipe',x,String(Math.min(inner[3]-20,772)),x,String(Math.max(inner[1]+20,Math.min(inner[3]-20,772)-130)),'450']);await pause(300);
 }
 if(pairs.size!==4)throw Error('Missing measured F2L pair cards: '+[...pairs]);
}
tapText(analysis,'Moves');await shot('android-review-moves');
const logs=run(['logcat','-d','-s','AndroidRuntime:E']);if(logs.includes('FATAL EXCEPTION'))throw Error(logs);
await writeFile(path.join(root,'.data/native-parity/reconstruction-android-checks.json'),JSON.stringify({stepButtonsAndArrows:stepChecks,spacePlayPause:true,resumesAtPlayhead:true,stagesVisible:[...seen],detailTabs:true,verifiedPairCards:[...pairs],directScramble:true,liveFrame:process.env.FACET_REVIEW_VIEW==='review-live-frame',synthetic:true,hardwareBluetooth:false},null,2));
if(seen.size!==4)throw Error('Could not visit all CFOP groups: '+[...seen]);
console.log('Native Android: Space play/pause/resume and all four CFOP groups verified.');

if(process.env.FACET_CFOP_TOTALS){
 const xml=await tree(),node=(xml.match(/<node\b[^>]*>/g)||[]).find(n=>n.includes('content-desc="Close review"'));
 const bounds=node&&/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/.exec(node);if(!bounds)throw Error('No close review control');const[x,y,a,b]=bounds.slice(1).map(Number);run(['shell','input','tap',String(Math.round((x+a)/2)),String(Math.round((y+b)/2))]);await pause(400);
 for(const[name,size]of [['phone','390x844'],['tiled','450x560'],['landscape','800x600']]){
  run(['shell','wm','size',size]);await pause(500);const xml=await tree();if(!xml.includes('content-desc="Total 1.54 seconds"'))throw Error('CFOP total missing at '+size);await shot('android-cfop-total-'+name);
 }
}
