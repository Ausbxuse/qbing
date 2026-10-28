const test=require('node:test');const assert=require('node:assert/strict');
const tags=require('../src/js/local-solve-tags.js');
const tools=require('../src/js/local-review-tools.js');
const {historyIndices,sessionMetrics}=require('../src/js/local-review-data.js');
test('tags are separate from notes, tolerant of older records and retained by JSON export',()=>{
 const record=[[0,12000],'R U','old note',1,['R@0','333'],{other:'preserved',localReview:{version:1,favorite:true,color:'mint',group:'practice'}}];
 assert.deepEqual(tags.read(JSON.parse(JSON.stringify(record))),{favorite:true,color:'mint',group:'practice'});
 assert.deepEqual(tags.read([]),{favorite:false,color:'',group:''});
 assert.deepEqual(tags.read([0,0,0,0,0,{localReview:{favorite:'yes',color:'invalid',group:'invalid'}}]),{favorite:false,color:'',group:''});
 assert.equal(record[2],'old note');assert.match(tags.words(record),/favorite mint practice/);
 const rows=[record,[[0,14000],'F U','',2]];
 const source={length:2,recordAt:i=>rows[i],timeTextAt:i=>''};
 assert.deepEqual(historyIndices(source,'favorites',''),[0]);
 assert.deepEqual(historyIndices({...source,tagFilter:'group:practice'},'all','mint'),[0]);
 assert.deepEqual(historyIndices({...source,tagFilter:'color:rose'},'favorites',''),[]);
});
test('Best metrics refer to the best scored solve; Mean excludes DNFs and handles missing sensor fields independently',()=>{
 const values=[11000,10000,-1,14000];const metrics=[{moves:20,tps:2},{moves:30,tps:3},{moves:99,tps:99},{moves:40,tps:null}];
 const result=sessionMetrics({length:4,timeAt:i=>values[i],metricsAt:i=>metrics[i]});
 assert.equal(result.bestIndex,1);assert.equal(result.best,10000);assert.equal(result.mean,35000/3);
 assert.equal(result.tps,2.5);assert.equal(result.moves,30);assert.equal(result.samples,2);assert.equal(result.moveSamples,3);
 const empty=sessionMetrics({length:0,timeAt:()=>0,metricsAt:()=>null});assert.equal(empty.best,null);assert.equal(empty.mean,null);assert.equal(empty.tps,null);
});
test('technique evidence uses logged gaps and actual notation without inventing finger measurements',()=>{
 const record=[[0,1600],'','',1,["R@0 U@100 R'@200 U'@300 U2@1000 y@1100 R@1600",'333']];
 const data=tools.technique(record);assert.deepEqual(data.gap,{time:700,before:"U'",after:'U2'});assert.equal(data.u2,1);assert.equal(data.rotations,1);assert.equal(data.trigger,true);
 assert.equal(tools.technique([[0,1000]]),null);
 assert.deepEqual(tools.tokens("r U2' Rw x'"),['Rw','U2','Rw',"x'"]);
});
