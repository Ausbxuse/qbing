import {spawn,spawnSync} from 'node:child_process';
import {mkdir,readFile,writeFile,unlink} from 'node:fs/promises';
import path from 'node:path';
const root=path.resolve(new URL('..',import.meta.url).pathname),out=path.join(root,'.data/native-parity');await mkdir(out,{recursive:true});
const display=':'+ (300+process.pid%100),xvfb=spawn('Xvfb',[display,'-screen','0','3840x2160x24','-nolisten','tcp'],{stdio:'ignore'});
await new Promise((resolve,reject)=>{xvfb.once('error',reject);setTimeout(resolve,600)});
try{
 const variants=process.env.NATIVE_CAPTURE_ALL?[['1440x1000','paper','result'],['1440x1000','carbon','result'],['1280x800','carbon','result'],['390x844','paper','result'],['390x844','carbon','result'],['1440x1000','paper','history'],['390x844','carbon','history']]:[['1440x1000','paper','result']];
 if(process.env.NATIVE_CAPTURE_WORKFLOWS)variants.push(...['connect','bluetooth-off','correction','half','reconstruction','companion','tools-last','replay-yellow'].map(view=>['1440x1000','paper',view]));
 if(process.env.NATIVE_CAPTURE_TILES)variants.push(...['360x400','450x560','640x480','800x600','960x540','960x1080','1080x720','1440x540'].flatMap(size=>['paper','carbon'].map(theme=>[size,theme,'result'])));
 if(process.env.NATIVE_CAPTURE_TILES)variants.push(...['360x400','450x560','960x540'].map(size=>[size,'carbon','history']));
 if(process.env.NATIVE_CAPTURE_CLEANUP){variants.length=0;variants.push(...[['1440x1000','paper','settings'],['450x740','carbon','settings'],['1440x1000','paper','history'],['390x844','carbon','history'],['450x560','carbon','history']]);}
 if(process.env.NATIVE_CAPTURE_ROTATIONS){variants.length=0;variants.push(['1440x1000','paper','replay-rotations'],['450x740','carbon','replay-rotations']);}
 if(process.env.NATIVE_CAPTURE_REVIEW){variants.length=0;variants.push(['1440x1000','paper','reconstruction'],['450x740','carbon','reconstruction'],['360x400','carbon','reconstruction'],['1440x1000','paper','result']);}
 if(process.env.NATIVE_CAPTURE_SCRAMBLE_ANALYSIS){variants.length=0;variants.push(...['case-analysis-f2l','case-analysis-oll','case-analysis-pll','half','correction','result'].map(view=>['1440x1000','paper',view]),['450x740','carbon','case-analysis-f2l']);}
 if(process.env.NATIVE_CAPTURE_CFOP){variants.length=0;variants.push(['1440x1000','paper','result'],['450x740','carbon','result'],['960x540','paper','result'],['450x560','carbon','result'],['1440x1000','paper','review-analysis'],['450x740','carbon','review-analysis']);}
 if(process.env.NATIVE_CAPTURE_GRAPHS){variants.length=0;variants.push(...['history','history-tps','history-moves','result-tps','result-moves'].flatMap(view=>[['1440x1000','paper',view],['450x740','carbon',view]]));}
 if(process.env.NATIVE_CAPTURE_VIEW){variants.length=0;variants.push(['1440x1000','paper',process.env.NATIVE_CAPTURE_VIEW],['450x740','carbon',process.env.NATIVE_CAPTURE_VIEW]);}
 for(const[size,theme,view]of variants){
 if(view==='companion')await unlink(out+'/companion-window.png').catch(e=>{if(e.code!=='ENOENT')throw e;});
 const child=spawn(process.env.CSTIMER_NATIVE_BINARY||path.join(root,'native/linux/target/release/cstimer-native'),['--capture-native'],{cwd:root,env:{...process.env,DISPLAY:display,LIBGL_ALWAYS_SOFTWARE:'1',CUBE_LAB_RENDER_DIR:out,CUBE_LAB_RENDER_FIXTURE:out+'/fixture.json',CUBE_LAB_CAPTURE_SIZE:size,CUBE_LAB_CAPTURE_RESIZE:process.env.NATIVE_CAPTURE_TILES?'1':'',CUBE_LAB_CAPTURE_THEME:theme,CUBE_LAB_CAPTURE_VIEW:view},stdio:['ignore','pipe','pipe']});let logs='';child.stderr.on('data',d=>logs+=d);child.stdout.on('data',d=>logs+=d);const timeout=setTimeout(()=>child.kill('SIGTERM'),20000);const done=new Promise((r,j)=>{child.once('error',j);child.once('exit',r)});
 if(view==='companion'){
  await new Promise(r=>setTimeout(r,1200));
  const capture=spawnSync('python3',[path.join(root,'scripts/capture-native-window.py'),display,out+'/companion-window.png'],{env:{...process.env,DISPLAY:display},encoding:'utf8'});
  if(capture.status!==0){child.kill('SIGTERM');throw Error(capture.stderr);}
  console.log(capture.stdout.trim());
 }
 const code=await done;clearTimeout(timeout);if(code!==0||/graphics uses software|shader.*error/i.test(logs))throw Error(logs);console.log(logs.trim());
 }
}finally{xvfb.kill('SIGTERM')}
