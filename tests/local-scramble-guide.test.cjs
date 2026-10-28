const {test} = require('node:test');
const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const factory = require('../src/js/local-scramble-guide.js');
const context = vm.createContext({ DEBUG: false, $: { isArray: Array.isArray } });
vm.runInContext(readFileSync(path.join(__dirname,'../src/js/lib/isaac.js'),'utf8'),context);
vm.runInContext(readFileSync(path.join(__dirname,'../src/js/lib/mathlib.js'),'utf8'),context);
const Cube = context.mathlib.CubieCube;
const Guide = factory(Cube, s => s);
function state(alg) { const c = new Cube(); alg.split(/\s+/).filter(Boolean).forEach(m=>c.selfMoveStr(m)); return c.toFaceCube(); }
function undo(tokens) { return tokens.find(t=>t.kind==='correction')?.text.replace(/^↶ /,'') || ''; }
test('a half turn displays a remaining quarter and symbol without losing the original target',()=>{
 const guide=new Guide('R2 U F');
 const tokens=guide.update(state('R'),['R']);
 assert.equal(tokens[0].text,'R');assert.equal(tokens[0].partial,true);assert.equal(tokens[0].kind,'current');
 const done=guide.update(state('R2'),['R']);assert.equal(done[0].text,'R2');assert.equal(done[0].kind,'done');
});
test('opposite faces may be completed in either order',()=>{
 const guide=new Guide("U2 D R F2");
 let tokens=guide.update(state('D'),['D']);
 assert.equal(tokens[1].kind,'done');assert.equal(tokens[0].kind,'current');assert.equal(undo(tokens),'');
 tokens=guide.update(state('D U2'),['U2']);assert.equal(tokens[0].kind,'done');assert.equal(tokens[1].kind,'done');assert.equal(tokens[2].kind,'current');
});
test('wrong turns insert an exact inverse at the cursor and recover to the same partial checkpoint',()=>{
 const guide=new Guide('R2 U F');guide.update(state('R'),['R']);
 let tokens=guide.update(state('R F'),['F']);assert.equal(undo(tokens),"F'");assert.equal(tokens[0].kind,'correction');assert.equal(tokens[1].partial,true);
 tokens=guide.update(state('R F U'),['U']);assert.equal(undo(tokens),"U' F'");
 const corrected=state("R F U U' F'");tokens=guide.update(corrected,["F'","U'",'U','F']);assert.equal(undo(tokens),'');assert.equal(tokens[0].partial,true);
});
test('recovered packets infer all reported turns and cancel redundant recovery',()=>{
 const guide=new Guide('R U F');guide.update(state('R'),['R']);
 let tokens=guide.update(state('R F L'),['L','F']);assert.equal(undo(tokens),"L' F'");
 tokens=guide.update(state("R F L L'"),["L'"]);assert.equal(undo(tokens),"F'");
});
test('an unexplained jump or unsupported move defers to the upstream guide',()=>{
 const guide=new Guide('R U F');assert.equal(guide.update(state('F L B'),['B']),null);
 assert.equal(new Guide('Rw U').enabled,false);
 assert.equal(new Guide('R L R').enabled,false);
});
