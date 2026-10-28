'use strict';
const test=require('node:test');const assert=require('node:assert/strict');
const {historyIndices}=require('../src/js/local-review-data.js');
const rows=[[[0,12440],"R U R'",'',1700000000],[[2000,10000],'F2 U','Try alternate PLL',1700000030,['R@0','333']],[[-1,9000],'L U','',1700000060,['L@0','333']],[[0,13020],'B2 D','   ',1700000090],[[0,14000],'R2 F','Cross pause',1700000120,['F@0','333']]];
const source={length:rows.length,recordAt:i=>rows[i],timeTextAt:i=>['12.44','12.00+','DNF','13.02','14.00'][i]};
test('history filters retain original solve numbers and newest-first order without changing records',()=>{
 const before=JSON.stringify(rows);
 assert.deepEqual(historyIndices(source,'all',''),[4,3,2,1,0]);
 assert.deepEqual(historyIndices(source,'moves',''),[4,2,1]);
 assert.deepEqual(historyIndices(source,'dnf',''),[2]);
 assert.deepEqual(historyIndices(source,'notes',''),[4,1]);
 assert.equal(JSON.stringify(rows),before);
});
test('history search combines filter with exact number, formatted upstream time, scramble or note terms',()=>{
 assert.deepEqual(historyIndices(source,'all','#2'),[1]);
 assert.deepEqual(historyIndices(source,'notes','#1'),[]);
 assert.deepEqual(historyIndices(source,'all','12.00+'),[1]);
 assert.deepEqual(historyIndices(source,'all','PLL ALTERNATE'),[1]);
 assert.deepEqual(historyIndices(source,'all','B2 D'),[3]);
 assert.deepEqual(historyIndices(source,'all','absent'),[]);
});
test('unqueried history filters avoid time formatting and tolerate malformed imported entries',()=>{
 const cheap={length:rows.length+1,recordAt:i=>rows[i],timeTextAt:()=>{throw Error('must stay lazy');}};
 assert.deepEqual(historyIndices(cheap,'all',''),[4,3,2,1,0]);
 assert.deepEqual(historyIndices(cheap,'dnf',''),[2]);
});
