import {spawnSync} from 'node:child_process';import {mkdir,copyFile,writeFile,chmod,rename,readFile} from 'node:fs/promises';import path from 'node:path';import os from 'node:os';
const root=path.resolve(new URL('..',import.meta.url).pathname),data=process.env.XDG_DATA_HOME||path.join(os.homedir(),'.local/share'),dest=path.join(data,'cstimer-native'),bin=path.join(os.homedir(),'.local/bin');
const source=path.join(root,'native/linux/target/release/cstimer-native');await mkdir(path.join(dest,'bin'),{recursive:true});await mkdir(bin,{recursive:true});
await copyFile(source,path.join(dest,'bin/cstimer-native.new'));await chmod(path.join(dest,'bin/cstimer-native.new'),0o755);await rename(path.join(dest,'bin/cstimer-native.new'),path.join(dest,'bin/cstimer-native'));
const libraries=process.env.LD_LIBRARY_PATH||'',quote=s=>"'"+s.replaceAll("'","'\\''")+"'";
const linked=spawnSync('ldd',[source],{encoding:'utf8'});if(linked.status!==0)throw Error(linked.stderr);
await mkdir(path.join(dest,'runtime-roots'),{recursive:true});for(const store of new Set((libraries+linked.stdout).match(/\/nix\/store\/[^/:\s]+/g)||[])){const r=spawnSync('nix-store',['--add-root',path.join(dest,'runtime-roots',path.basename(store)),'--indirect','--realise',store],{stdio:'ignore'});if(r.status!==0)throw Error('Could not retain runtime '+store);}
const launcher=path.join(bin,'cstimer-native');await writeFile(launcher,'#!/bin/sh\nexport LD_LIBRARY_PATH='+quote(libraries)+'${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}\ncase "${1:-}" in\n  ""|--companion|--floating-companion)\n    if systemctl --user cat facet.service >/dev/null 2>&1; then\n      systemctl --user start facet.service || exit $?\n    fi\n    ;;\nesac\nexec '+quote(path.join(dest,'bin/cstimer-native'))+' "$@"\n');await chmod(launcher,0o755);
const iconRoot=path.join(data,'icons/hicolor'),icons=path.join(iconRoot,'scalable/apps');
await mkdir(icons,{recursive:true});await copyFile(path.join(root,'native/assets/app-icon/facet.svg'),path.join(icons,'facet-mark.svg'));
for(const size of [32,48,64,128,256,512]){const target=path.join(iconRoot,`${size}x${size}/apps`);await mkdir(target,{recursive:true});await copyFile(path.join(root,`native/assets/app-icon/facet-${size}.png`),path.join(target,'facet-mark.png'));}
spawnSync('gtk-update-icon-cache',['-f','-t',iconRoot],{stdio:'ignore'});
await mkdir(path.join(data,'applications'),{recursive:true});await writeFile(path.join(data,'applications/org.cstimer.Native.desktop'),'[Desktop Entry]\nType=Application\nName=Facet\nComment=Rust desktop smart-cube timer\nExec='+launcher+'\nIcon=facet-mark\nStartupWMClass=org.cstimer.Native\nTerminal=false\nCategories=Education;Sports;\nActions=Companion;Floating;Quit;\n\n[Desktop Action Companion]\nName=Companion\nExec='+launcher+' --companion\n\n[Desktop Action Floating]\nName=Floating companion\nExec='+launcher+' --floating-companion\n\n[Desktop Action Quit]\nName=Quit Facet\nExec='+launcher+' --quit\n');
await mkdir(path.join(dest,'licenses'),{recursive:true});for(const[name,source]of [['GPL-3.0.txt','LICENSE'],['OFL-geist.txt','src/fonts/OFL-geist.txt'],['OFL-maple.txt','src/fonts/OFL-maple.txt'],['LICENSE-alg-trainer.txt','src/assets/LICENSE-alg-trainer.txt'],['LICENSE-gan-web-bluetooth.txt','native/shared/src/commonMain/composeResources/files/LICENSE-gan-web-bluetooth.txt']])await copyFile(path.join(root,source),path.join(dest,'licenses',name));
await mkdir(path.join(root,'.data/native-release'),{recursive:true});const apk=path.join(root,'.data/native-release/facet.apk');await copyFile(path.join(root,'native/androidApp/build/outputs/apk/release/androidApp-release.apk'),apk);console.log('Installed '+launcher+' and Facet desktop entry.\nAndroid APK: '+apk);

const extensionId='cstimer-companion@local',extensionDir=path.join(data,'gnome-shell/extensions',extensionId);
await mkdir(extensionDir,{recursive:true});for(const file of ['extension.js','notation.js','stylesheet.css','metadata.json'])await copyFile(path.join(root,'native/gnome',extensionId,file),path.join(extensionDir,file));
console.log('Installed optional GNOME top-bar extension: '+extensionId);

const companionLauncher=path.join(bin,'cstimer-companion');
await writeFile(companionLauncher,'#!/bin/sh\nexec '+quote(launcher)+' --companion "$@"\n');await chmod(companionLauncher,0o755);
await writeFile(path.join(data,'applications/cstimer-companion.desktop'),'[Desktop Entry]\nType=Application\nName=Facet Companion\nComment=GNOME top-bar companion for Facet\nExec='+companionLauncher+'\nIcon=facet-mark\nStartupWMClass=org.cstimer.Native\nTerminal=false\nCategories=Education;Sports;\n');

// Friendly commands; existing launchers and stable application/storage IDs remain compatible.
for(const [name,target] of [['facet',launcher],['facet-companion',companionLauncher]]){
 const file=path.join(bin,name);await writeFile(file,'#!/bin/sh\nexec '+quote(target)+' "$@"\n');await chmod(file,0o755);
}

// Enable login startup without restarting a running app or the user's session.
const unitDir=path.join(process.env.XDG_CONFIG_HOME||path.join(os.homedir(),'.config'),'systemd/user');
await mkdir(unitDir,{recursive:true});
const unit=(await readFile(path.join(root,'native/systemd/facet.service.in'),'utf8'))
 .replaceAll('@LAUNCHER@',launcher).replaceAll('@DOCUMENTATION@',path.join(dest,'NATIVE.md'));
await copyFile(path.join(root,'NATIVE.md'),path.join(dest,'NATIVE.md'));
await writeFile(path.join(unitDir,'facet.service'),unit);
for(const args of [['--user','daemon-reload'],['--user','enable','facet.service']]){
 const result=spawnSync('systemctl',args,{encoding:'utf8'});
 if(result.status!==0)throw Error(result.stderr||'Could not enable Facet background service');
}
console.log('Enabled facet.service for graphical login. Start with systemctl --user start facet.service.');
// Preserve every unrelated extension preference, and enable only our UUID.
const enabled=spawnSync('gsettings',['get','org.gnome.shell','enabled-extensions'],{encoding:'utf8'});
if(enabled.status===0&&!enabled.stdout.includes("'"+extensionId+"'")){
 const previous=enabled.stdout.trim().replace(/^@as /,'');
 const next=previous==='[]'?"['"+extensionId+"']":previous.slice(0,-1)+", '"+extensionId+"']";
 const result=spawnSync('gsettings',['set','org.gnome.shell','enabled-extensions',next],{encoding:'utf8'});
 if(result.status!==0)throw Error(result.stderr);
}
