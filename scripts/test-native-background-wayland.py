"""Prove background snapshots advance while a different Wayland app has focus."""
import ast,json,math,os,subprocess as S,time
from pathlib import Path
assert os.environ.get('FACET_PRIVATE_TEST')=='1'
root=Path(__file__).resolve().parent.parent
folder=root/'.data/wayland-background';folder.mkdir(exist_ok=True)
base=dict(os.environ);base.pop('LD_LIBRARY_PATH',None);base.pop('DISPLAY',None)
base['XDG_DATA_HOME']=str(root/'.data/panel-shell/share');base['XDG_DATA_DIRS']=base['XDG_DATA_HOME']+':'+base.get('XDG_DATA_DIRS','/run/current-system/sw/share');base['XDG_CONFIG_HOME']=str(root/'.data/panel-shell/config')
base['FACET_FOCUS_REPORT']=str(folder/'shell-focus.json');base['GSETTINGS_BACKEND']='memory';base['LIBGL_ALWAYS_SOFTWARE']='1';base['WAYLAND_DISPLAY']='facet-background-test';base['NO_AT_BRIDGE']='1'
client=dict(base,LD_LIBRARY_PATH=os.environ.get('FACET_TEST_LIBS',''))
children=[];logs=[]
def launch(args,env,name):
 log=(folder/(name+'.log')).open('w');logs.append(log);p=S.Popen(args,env=env,stdout=log,stderr=log);children.append(p);return p

def call(method):
 r=S.run(['gdbus','call','--session','--dest','org.cstimer.Native.Companion','--object-path','/org/cstimer/Native/Companion','--method','org.cstimer.Native.Companion.'+method],env=base,capture_output=True,text=True,timeout=5)
 if r.returncode:raise RuntimeError(r.stderr)
 return r.stdout

def snapshot():return json.loads(ast.literal_eval(call('GetSnapshot'))[0])
def shell_eval(expression):
 r=S.run(['gdbus','call','--session','--dest','org.gnome.Shell','--object-path','/org/gnome/Shell','--method','org.gnome.Shell.Eval',expression],env=base,capture_output=True,text=True,timeout=5)
 assert r.returncode==0,r.stderr
 ok=r.stdout.startswith('(true,');value=ast.literal_eval(r.stdout.split(',',1)[1].rsplit(')',1)[0].strip());assert ok,value
 return value

def key(pressed):
 shell_eval("Main.extensionManager.lookup('cstimer-panel-test@local').stateObj.key(32,"+str(pressed).lower()+"); true")

def wait(test,label):
 deadline=time.monotonic()+20
 while time.monotonic()<deadline:
  try:
   value=snapshot()
   if 'windowVisible' in value and test(value):return value
  except (RuntimeError,ValueError):pass
  if 'app' in globals() and app.poll() is not None:raise AssertionError((folder/'app.log').read_text())
  time.sleep(.2)
 raise AssertionError(label)
try:
 shell=launch(['gnome-shell','--headless','--wayland','--no-x11','--wayland-display=facet-background-test','--virtual-monitor=1600x900','--mode=cstimer-test'],base,'shell')
 socket=Path(os.environ['XDG_RUNTIME_DIR'])/'facet-background-test'
 deadline=time.monotonic()+20
 while not socket.exists():
  if shell.poll() is not None or time.monotonic()>deadline:raise AssertionError((folder/'shell.log').read_text())
  time.sleep(.2)
 native_binary=os.environ.get('FACET_TEST_BINARY',str(root/'native/linux/target/debug/cstimer-native'))
 app=launch([native_binary,'--daemon','--isolated'],base if 'FACET_TEST_BINARY' in os.environ else client,'app')
 first=wait(lambda s:not s['windowVisible'],'hidden startup');time.sleep(1.6);second=snapshot()
 assert second['updatedAt']>first['updatedAt'] and time.time()*1000-second['updatedAt']<1000,second
 call('ShowMain');wait(lambda s:s['windowVisible'],'show command while hidden')
 time.sleep(1)
 shot=S.run(['gdbus','call','--session','--dest','org.gnome.Shell.Screenshot','--object-path','/org/gnome/Shell/Screenshot','--method','org.gnome.Shell.Screenshot.Screenshot','false','false',str(folder/'main-open.png')],env=base,capture_output=True,text=True,timeout=5)
 assert shot.returncode==0,shot.stderr
 active=folder/'shell-focus.json';active.unlink(missing_ok=True)
 cover_env=dict(client,FACET_COVER_REPORT=str(folder/'client-focus.json'))
 other=launch([str(root/'native/linux/target/debug/examples/background_cover')],cover_env,'other-app')
 deadline=time.monotonic()+20
 while not active.exists() or not json.loads(active.read_text()).get('active'):
  if other.poll() is not None or time.monotonic()>deadline:raise AssertionError((folder/'other-app.log').read_text()+' focus: '+(active.read_text() if active.exists() else 'no shell report'))
  time.sleep(.2)
 time.sleep(1.5);first=snapshot();sample_started=time.monotonic();time.sleep(3);second=snapshot()
 assert second['updatedAt']>first['updatedAt'] and time.time()*1000-second['updatedAt']<1000,second
 # egui counts layout passes, not presented frames: a 1Hz idle repaint may
 # have two passes. Use elapsed time and a boundary allowance, not a frame
 # count assumption. A continuous render loop still fails by a large margin.
 assert second['renderPass']-first['renderPass']<=math.ceil(2*(time.monotonic()-sample_started))+1,(first['renderPass'],second['renderPass'])
 call('HideMain');wait(lambda s:not s['windowVisible'],'hide while other app focused');time.sleep(1.6)
 assert time.time()*1000-snapshot()['updatedAt']<1000
 # Test idle rendering before keyboard timing creates a solve and starts
 # asynchronous scramble/cross searches with their legitimate busy indicators.
 other.terminate();other.wait(timeout=5)
 call('ShowMain');wait(lambda s:s['windowVisible'],'show after cover closes');time.sleep(.8)
 # Exercise actual compositor close/reopen and keyboard input, not only a visibility flag.
 for _ in range(2):
  shell_eval("global.get_window_actors().find(a=>a.meta_window.get_title()==='Facet').meta_window.delete(global.get_current_time()); true")
  wait(lambda s:not s['windowVisible'],'compositor close keeps model alive')
  call('ShowMain');wait(lambda s:s['windowVisible'],'reopen after compositor close');time.sleep(.8)
 key(True);time.sleep(.7);key(False)
 wait(lambda s:s['running'],'keyboard timer after reopening')
 time.sleep(.3);key(True);key(False)
 wait(lambda s:not s['running'] and s['time']!='0.00','keyboard timer stop after reopening')
 call('Quit');assert app.wait(timeout=10)==0
 report={'isolated':True,'hardwareBluetooth':False,'wayland':True,'freshHiddenSnapshots':True,'freshSnapshotsUnderOtherApp':True,'otherAppFocused':True,'occludedRenderPasses':second['renderPass']-first['renderPass'],'commandsWorkWithoutRedraw':True,'gracefulQuit':True,'repeatedCompositorCloseReopen':True,'keyboardWorksAfterReopen':True}
 (folder/'checks.json').write_text(json.dumps(report,indent=2));print(json.dumps(report))
finally:
 for p in reversed(children):
  if p.poll() is None:p.terminate()
 for p in reversed(children):
  try:p.wait(timeout=10)
  except S.TimeoutExpired:p.kill();p.wait()
 for log in logs:log.close()
