const {test} = require('node:test');
const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
function fixture({failInit=false, failClear=false}={}) {
 const events = {}, calls = [];
 const device = {name:'Test cube', addEventListener:(name,fn)=>events[name]=fn, removeEventListener:name=>delete events[name], gatt:{connected:true,disconnect(){calls.push('disconnect');this.connected=false}}};
 const model={prefix:'Test',init:()=>failInit?Promise.reject(new Error('connection failed')):Promise.resolve(),clear:()=>{calls.push('clear');return failClear?Promise.reject(new Error('radio off')):Promise.resolve()}};
 const context=vm.createContext({DEBUG:false,DEBUGBL:false,Promise,AbortController,setTimeout,window:{navigator:{bluetooth:{requestDevice:()=>Promise.resolve(device)}}},$: {isArray:Array.isArray,noop:()=>{}},giikerutil:{chkAvail:()=>Promise.resolve(),log:()=>{}},execMain:fn=>fn()});
 vm.runInContext(readFileSync(path.join(__dirname,'../src/js/hardware/bluetooth.js'),'utf8'),context);
 const api=context.GiikerCube;api.regCubeModel(model);
 return {api,calls,events};
}
test('radio shutdown releases connection even when protocol cleanup rejects',async()=>{
 const {api,calls}=fixture({failClear:true});await api.init();assert.equal(api.isConnected(),true);
 const stopping=api.stop(true);assert.equal(api.isConnected(),false);await stopping;
 assert.deepEqual(calls,['clear','disconnect']);assert.equal(Boolean(api.getCube()),false);
});
test('a failed initial connection does not leave a connected device behind',async()=>{
 const {api,calls}=fixture({failInit:true,failClear:true});await assert.rejects(api.init(),/connection failed/);
 assert.equal(api.isConnected(),false);assert.deepEqual(calls,['clear','disconnect']);
});
test('hardware disconnection notifies timer despite failed cleanup',async()=>{
 const {api,events}=fixture({failClear:true});let event;api.setEventCallback(info=>event=info);
 await api.init();await events.gattserverdisconnected();assert.equal(event,'disconnect');assert.equal(api.isConnected(),false);
});
