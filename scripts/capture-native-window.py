"""Capture one named window on the isolated Xvfb display; no real desktop access."""
import ctypes as C
import ctypes.util
import json
import os
from pathlib import Path
import struct
import sys
import time
import zlib

assert os.environ.get('DISPLAY') == sys.argv[1]
x = C.CDLL(ctypes.util.find_library('X11') or 'libX11.so.6')
x.XOpenDisplay.argtypes=[C.c_char_p];x.XOpenDisplay.restype=C.c_void_p
x.XDefaultRootWindow.argtypes=[C.c_void_p];x.XDefaultRootWindow.restype=C.c_ulong
x.XQueryTree.argtypes=[C.c_void_p,C.c_ulong,C.POINTER(C.c_ulong),C.POINTER(C.c_ulong),C.POINTER(C.POINTER(C.c_ulong)),C.POINTER(C.c_uint)]
x.XFetchName.argtypes=[C.c_void_p,C.c_ulong,C.POINTER(C.c_char_p)]
x.XFree.argtypes=[C.c_void_p]
x.XGetGeometry.argtypes=[C.c_void_p,C.c_ulong,C.POINTER(C.c_ulong),C.POINTER(C.c_int),C.POINTER(C.c_int),C.POINTER(C.c_uint),C.POINTER(C.c_uint),C.POINTER(C.c_uint),C.POINTER(C.c_uint)]
x.XGetImage.argtypes=[C.c_void_p,C.c_ulong,C.c_int,C.c_int,C.c_uint,C.c_uint,C.c_ulong,C.c_int];x.XGetImage.restype=C.c_void_p
x.XGetPixel.argtypes=[C.c_void_p,C.c_int,C.c_int];x.XGetPixel.restype=C.c_ulong
x.XDestroyImage.argtypes=[C.c_void_p]
x.XRaiseWindow.argtypes=[C.c_void_p,C.c_ulong]
x.XSync.argtypes=[C.c_void_p,C.c_int]
class ClassHint(C.Structure):_fields_=[('name',C.c_char_p),('class_name',C.c_char_p)]
x.XGetClassHint.argtypes=[C.c_void_p,C.c_ulong,C.POINTER(ClassHint)]
d=x.XOpenDisplay(sys.argv[1].encode());assert d,'Open isolated X display'
def find(window):
 name=C.c_char_p()
 if x.XFetchName(d,window,C.byref(name)) and name.value:
  title=name.value.decode();x.XFree(name)
  if title in ('Facet Companion','Cube Companion'):return window
 root=C.c_ulong();parent=C.c_ulong();children=C.POINTER(C.c_ulong)();count=C.c_uint()
 if x.XQueryTree(d,window,C.byref(root),C.byref(parent),C.byref(children),C.byref(count)):
  ids=list(children[:count.value]);x.XFree(children)
  for child in ids:
   found=find(child)
   if found:return found
window=find(x.XDefaultRootWindow(d));assert window,'Detached companion window exists'
root=C.c_ulong();px=C.c_int();py=C.c_int();w=C.c_uint();h=C.c_uint();border=C.c_uint();depth=C.c_uint()
x.XGetGeometry(d,window,C.byref(root),C.byref(px),C.byref(py),C.byref(w),C.byref(h),C.byref(border),C.byref(depth))
assert (w.value,h.value)==(210,58),(w.value,h.value)
hint=ClassHint();assert x.XGetClassHint(d,window,C.byref(hint));identity=hint.class_name.decode()
assert 'cstimer-companion' in (identity,hint.name.decode()),(identity,hint.name)
x.XRaiseWindow(d,window);x.XSync(d,0);time.sleep(.4)
image=x.XGetImage(d,window,0,0,w.value,h.value,C.c_ulong(-1).value,2);assert image
rows=bytearray();colors=set()
for row in range(h.value):
 rows.append(0)
 for col in range(w.value):
  p=x.XGetPixel(image,col,row);colors.add(p);rows.extend(((p>>16)&255,(p>>8)&255,p&255,255))
x.XDestroyImage(image)
assert len(colors)>10,"Companion must contain visible rendered text, not a blank window"
def chunk(name,data):return struct.pack('>I',len(data))+name+data+struct.pack('>I',zlib.crc32(name+data)&0xffffffff)
output=Path(sys.argv[2]);output.write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w.value,h.value,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(rows))+chunk(b'IEND',b''))
output.with_suffix('.json').write_text(json.dumps({'synthetic':True,'detachedNativeWindow':True,'width':w.value,'height':h.value,'wmClass':identity}))
print('Companion: detached 210 × 58 native window; Forge floating identity verified.')
