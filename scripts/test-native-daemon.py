"""Exercise the real native lifecycle on a private bus/X display, without BLE/storage."""
import ast
import ctypes as C
import ctypes.util
import json
import os
from pathlib import Path
import subprocess as S
import time

assert os.environ.get('FACET_PRIVATE_TEST') == '1'
root = Path(__file__).resolve().parent.parent
display = ':' + str(400 + os.getpid() % 100)
env = dict(os.environ, DISPLAY=display, WINIT_UNIX_BACKEND='x11')
env.pop('WAYLAND_DISPLAY', None)
clean = dict(env)
clean.pop('LD_LIBRARY_PATH', None)
binary = root / 'native/linux/target/debug/cstimer-native'
xvfb = S.Popen(['Xvfb', display, '-screen', '0', '1600x1000x24', '-nolisten', 'tcp'], env=env, stdout=S.DEVNULL, stderr=S.DEVNULL)
app = None
log_path = root / '.data/daemon-runtime.log'

def call(method, *args):
    result = S.run(['gdbus', 'call', '--session', '--dest', 'org.cstimer.Native.Companion', '--object-path', '/org/cstimer/Native/Companion', '--method', 'org.cstimer.Native.Companion.' + method, *args], env=clean, capture_output=True, text=True, timeout=5)
    if result.returncode:
        raise RuntimeError(result.stderr)
    return result.stdout

def snapshot():
    return json.loads(ast.literal_eval(call('GetSnapshot'))[0])

def wait_for(test, label):
    until = time.monotonic() + 15
    while time.monotonic() < until:
        try:
            value = snapshot()
            if test(value):
                return value
        except (RuntimeError, ValueError):
            pass
        if app.poll() is not None:
            raise AssertionError(log_path.read_text())
        time.sleep(.15)
    raise AssertionError(label + ': ' + log_path.read_text())

try:
    time.sleep(.6)
    with log_path.open('w') as log:
        app = S.Popen([str(binary), '--daemon', '--isolated'], env=env, stdout=log, stderr=log)
    first = wait_for(lambda s: s['background'] and not s['windowVisible'], 'hidden startup')
    call('ConnectCube', 'unknown')
    wait_for(lambda s:s.get('connection',{}).get('message')=='Cube unavailable. Scan again.' and not s['windowVisible'],'hidden connection command')
    call('ScanCubes')
    wait_for(lambda s:s.get('connection',{}).get('message')!='Cube unavailable. Scan again.' and not s['windowVisible'],'hidden scan command')

    # No polling during this interval: hidden mode must keep its own event loop alive.
    time.sleep(1.6)
    second = snapshot()
    assert second['updatedAt'] > first['updatedAt'], 'Hidden model stopped ticking'
    assert time.time() * 1000 - second['updatedAt'] < 1500, 'Stale hidden snapshot'
    S.run([str(binary)], env=env, timeout=10, check=True)
    wait_for(lambda s: s['windowVisible'] and s['pid'] == first['pid'], 'reuse main window')

    x = C.CDLL(C.util.find_library('X11') or 'libX11.so.6')
    x.XOpenDisplay.argtypes = [C.c_char_p]; x.XOpenDisplay.restype = C.c_void_p
    x.XDefaultRootWindow.argtypes = [C.c_void_p]; x.XDefaultRootWindow.restype = C.c_ulong
    x.XQueryTree.argtypes = [C.c_void_p,C.c_ulong,C.POINTER(C.c_ulong),C.POINTER(C.c_ulong),C.POINTER(C.POINTER(C.c_ulong)),C.POINTER(C.c_uint)]
    x.XFetchName.argtypes = [C.c_void_p,C.c_ulong,C.POINTER(C.c_char_p)]
    x.XFree.argtypes = [C.c_void_p]
    x.XInternAtom.argtypes = [C.c_void_p,C.c_char_p,C.c_int]; x.XInternAtom.restype = C.c_ulong
    x.XSendEvent.argtypes = [C.c_void_p,C.c_ulong,C.c_int,C.c_long,C.c_void_p]
    x.XSync.argtypes = [C.c_void_p,C.c_int]
    d = x.XOpenDisplay(display.encode()); assert d
    def windows(parent):
        name = C.c_char_p()
        result = []
        if x.XFetchName(d,parent,C.byref(name)) and name.value:
            if name.value.decode() == 'Qbing': result.append(parent)
            x.XFree(name)
        a=C.c_ulong(); b=C.c_ulong(); children=C.POINTER(C.c_ulong)(); count=C.c_uint()
        if x.XQueryTree(d,parent,C.byref(a),C.byref(b),C.byref(children),C.byref(count)):
            ids=list(children[:count.value]);x.XFree(children)
            for child in ids: result.extend(windows(child))
        return result
    ids = windows(x.XDefaultRootWindow(d)); assert len(ids) == 1, ids
    class Data(C.Union): _fields_=[('l',C.c_long*5),('b',C.c_char*20)]
    class Message(C.Structure): _fields_=[('type',C.c_int),('serial',C.c_ulong),('send_event',C.c_int),('display',C.c_void_p),('window',C.c_ulong),('message_type',C.c_ulong),('format',C.c_int),('data',Data)]
    class Event(C.Union): _fields_=[('message',Message),('padding',C.c_long*24)]
    event = Event(); event.message.type=33;event.message.send_event=1;event.message.display=d;event.message.window=ids[0];event.message.format=32
    event.message.message_type=x.XInternAtom(d,b'WM_PROTOCOLS',0)
    event.message.data.l[0]=x.XInternAtom(d,b'WM_DELETE_WINDOW',0)
    S.run([str(binary), '--hide'], env=env, timeout=10, check=True)
    wait_for(lambda s:not s['windowVisible'],'explicit hide')
    call('ShowMain');wait_for(lambda s:s['windowVisible'],'reopen after explicit hide')
    assert x.XSendEvent(d,ids[0],0,0,C.byref(event));x.XSync(d,0)
    closed = wait_for(lambda s:not s['windowVisible'] and s['background'], 'window close keeps companion')
    time.sleep(1.6)
    after_close = snapshot()
    assert after_close['updatedAt'] > closed['updatedAt'], 'Closing window stopped companion'
    call('ShowMain');wait_for(lambda s:s['windowVisible'], 'reopen after window close')
    call('Quit')
    assert app.wait(timeout=10)==0, log_path.read_text()
    report={'isolated':True,'hardwareBluetooth':False,'hiddenStartup':True,'connectionControlsWithoutWindow':True,'freshHiddenSnapshots':True,'oneProcess':True,'explicitHideKeepsServiceAlive':True,'reopen':True,'windowCloseKeepsCompanionAlive':True,'reopenAfterClose':True,'explicitQuitExits':True,'gracefulShutdown':True}
    (root/'.data/daemon-runtime-checks.json').write_text(json.dumps(report,indent=2))
    print('Native lifecycle: explicit background mode, reopen, and companion survives window close, and explicit quit verified.')

finally:
    if app is not None and app.poll() is None:
        app.terminate();app.wait(timeout=10)
    xvfb.terminate();xvfb.wait(timeout=10)
