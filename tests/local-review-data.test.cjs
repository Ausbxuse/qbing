'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const {groupPhases, phaseBaseline, recentTimes} = require('../src/js/local-review-data.js');
function reconstruction(scale = 1) {
 return {data: [[800,850,1000,8],[700,750,800,4],[600,650,700,3],[500,520,600,2],[400,420,500,2],[200,250,400,4],[0,40,200,8]].map(p => p.map((v,i) => i < 3 ? v * scale : v))};
}
function record(scale=1, penalty=0) {return [[penalty,1000*scale],'','',0,['R@0','333']];}
function source(records, recs) {return {length:records.length,recordAt:i=>records[i],reconstructionAt:i=>recs[i]};}
test('groups all four F2L slots while conserving duration, pauses, execution and HTM counts',()=>{
 const groups=groupPhases(reconstruction());
 assert.deepEqual(groups[1],{name:'F2L',time:500,pause:140,execution:360,moves:11});
 assert.equal(groups.reduce((sum,p)=>sum+p.time,0),1000);
 assert.ok(groups.every(p=>p.pause+p.execution===p.time));
 assert.equal(groups.reduce((sum,p)=>sum+p.moves,0),31);
});
test('phase baseline excludes DNFs, manual entries and invalid reconstructions; +2 does not inflate stage time',()=>{
 const records=[record(100),record(1),record(2,2000),record(3),record(80,-1),record(90),record(100)];
 records[5].pop();
 const recs=[reconstruction(100),reconstruction(1),reconstruction(2),reconstruction(3),reconstruction(80),reconstruction(90),reconstruction(100)];
 recs[6].data[0][1]=Infinity;
 const baseline=phaseBaseline(source(records,recs),6);
 assert.deepEqual(baseline[0],{samples:3,window:6,median:400});
 assert.equal(baseline[1].median,1000);
});
test('fewer than three eligible phase samples do not become a personal baseline',()=>{
 const baseline=phaseBaseline(source([record(),record(2)],[reconstruction(),reconstruction(2)]),50);
 assert.deepEqual(baseline[0],{samples:2,window:2,median:null});
 assert.equal(groupPhases({data:[]}),null);
 const invalid=reconstruction();invalid.data[2][1]=invalid.data[2][2]+1;
 assert.equal(groupPhases(invalid),null);
});
test('recent time trace retains chronological indices and DNF gaps without connecting invented times',()=>{
 const values=[99,1000,-1,NaN,2000,Infinity,0];
 assert.deepEqual(recentTimes({length:values.length,timeAt:i=>values[i]},6),[
  {index:1,time:1000},{index:2,time:null},{index:3,time:null},{index:4,time:2000},{index:5,time:null},{index:6,time:0}
 ]);
});

const {timeDistribution} = require('../src/js/local-review-data.js');
test('distribution uses equal half-open bins, counts boundaries exactly once and separates DNFs', () => {
 const values = [10000, 10000, 12000, 14000, 18000, -1, NaN, Infinity];
 const d = timeDistribution(values);
 assert.equal(d.count, 5); assert.equal(d.dnf, 1); assert.equal(d.step, 1000);
 assert.equal(d.bins.reduce((sum, b) => sum + b.count, 0), 5);
 assert.equal(d.bins[0].count, 2); assert.equal(d.bins.at(-1).count, 1);
 d.bins.forEach(b => assert.equal(b.to - b.from, d.step));
 assert.deepEqual(values.slice(0, 5), [10000,10000,12000,14000,18000]);
});
test('distribution handles empty, identical, zero and widely separated times without losing samples', () => {
 assert.deepEqual(timeDistribution([-1, NaN]), {bins:[], count:0, dnf:1, step:0});
 for (const values of [[13730,13730], [0], [1, 9000000]]) {
  const d = timeDistribution(values);
  assert.ok(d.bins.length <= 10); assert.equal(d.bins.reduce((s,b)=>s+b.count,0),values.length);
  values.forEach(t => assert.equal(d.bins.filter(b => t >= b.from && t < b.to).length,1));
 }
});

const {timeSummary} = require('../src/js/local-review-data.js');
test('distribution insights use scored times, exclude DNFs and invalid values, and conserve input', () => {
 const values=[1000, 3000, 2000, 4000, -1, NaN, Infinity];
 const summary=timeSummary(values);
 assert.equal(summary.median,2500);
 assert.equal(summary.p90,4000);
 assert.equal(summary.spread,Math.sqrt(1250000));
 assert.equal(summary.dnfRate,0.2);
 assert.deepEqual(values.slice(0,5),[1000,3000,2000,4000,-1]);
 assert.deepEqual(timeSummary([]),{count:0,dnf:0,median:null,spread:null,p90:null,dnfRate:null});
 assert.deepEqual(timeSummary([-1]),{count:0,dnf:1,median:null,spread:null,p90:null,dnfRate:1});
 assert.deepEqual(timeSummary([2000]),{count:1,dnf:0,median:2000,spread:null,p90:2000,dnfRate:0});
 assert.equal(timeSummary(Array.from({length:10},(_,i)=>(i+1)*1000)).p90,9000);
});
