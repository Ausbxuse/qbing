const test=require('node:test');const assert=require('node:assert/strict');
const {Motion}=require('../src/js/local-cube-motion.js');
const apply=(state,move)=>state+' '+move;
test('new packets preserve current interpolation and queue the next verified turn',()=>{
 const motion=new Motion('S',apply);motion.accept('S R',['R'],0,true);const first=motion.frame(0).active;
 const p=motion.frame(40).progress;motion.accept('S R U',['U','R'],40,true);
 assert.equal(motion.active.id,first.id);assert.ok(Math.abs(motion.frame(40).progress-p)<1e-10);
 assert.equal(motion.target,'S R U');assert.equal(motion.display,'S');
 motion.frame(200);assert.equal(motion.display,'S R');assert.equal(motion.active.move,'U');motion.frame(400);assert.equal(motion.display,motion.target);assert.equal(motion.busy(),false);
});
test('recoverable packet batches animate in chronological order; duplicate snapshots do not interrupt',()=>{
 const motion=new Motion('S',apply);motion.accept('S R U',["U",'R'],0,true);assert.deepEqual(motion.queue.map(s=>s.move),['R','U']);
 motion.frame(0);const active=motion.active;assert.equal(motion.accept('S R U',['U','R'],10,true),false);assert.equal(motion.active,active);
});
test('unverified state, reduced motion, and hidden views resynchronize without fabricated turns',()=>{
 const motion=new Motion('S',apply);motion.accept('S R',['R'],0,true);motion.frame(0);
 motion.accept('unknown',['U'],20,true);assert.equal(motion.display,'unknown');assert.equal(motion.busy(),false);
 motion.accept('unknown F',['F'],30,false);assert.equal(motion.display,'unknown F');assert.equal(motion.busy(),false);
 motion.snap(motion.target);assert.equal(motion.queue.length,0);
});
test('bursts stay bounded and eventually match the latest authoritative state',()=>{
 const motion=new Motion('S',apply);let target='S';
 for(let i=0;i<100;i++){target=apply(target,'R');motion.accept(target,['R'],i*10,true);motion.frame(i*10);assert.ok(motion.queue.length<=8);}
 for(let t=1000;t<1500;t+=16)motion.frame(t);
 assert.equal(motion.display,target);assert.equal(motion.busy(),false);
});

test('pending same-direction quarters become a coherent half turn, including inverse direction',()=>{
 for(const move of ['U',"U'"]){const motion=new Motion('S',apply),target=apply(apply('S',move),move);motion.accept(target,[move,move],0,true);assert.equal(motion.queue.length,1);assert.equal(motion.queue[0].move,'U2');assert.equal(motion.queue[0].turns,move==='U'?2:-2);motion.frame(0);motion.frame(200);assert.equal(motion.display,target);}
});

test('the remaining queue drains against one shared deadline instead of slowing down at its tail',()=>{
 const motion=new Motion('S',apply);let target='S';
 const faces=['R','U','F','L','D','B'];
 for(let now=0;now<800;now+=16){if(now%32===0){const m=faces[(now/32)%faces.length];target=apply(target,m);motion.accept(target,[m],now,true);}motion.frame(now);}
 let end=800;while(motion.busy()&&end<1200){motion.frame(end);end+=16;}
 assert.equal(motion.display,target);assert.ok(end<=1008,'queue stays near its 120ms budget');
});
