import test from 'node:test';import assert from 'node:assert/strict';
import {splitRows,notation,scrambleReady} from '../../native/gnome/cstimer-companion@local/notation.js';
test('two balanced rows retain every move and do not reflow as the cursor advances',()=>{
 const tokens="D2 B2 U F2 D R2 F2 L2 U' L2 D2 F' R2 U' B R' U' L' D R2".split(' ').map(text=>({text,kind:0}));
 const rows=splitRows(tokens);assert.deepEqual(rows.flat(),tokens);assert.equal(rows.length,2);
 const widths=rows.map(row=>row.map(t=>t.text).join(' ').length);assert.ok(Math.abs(widths[0]-widths[1])<=3);
 for(let i=0;i<tokens.length;i++){const changed=tokens.map((t,j)=>({...t,kind:j<i?1:j===i?2:0}));assert.equal(splitRows(changed)[0].length,rows[0].length);}
});
test('an early opposite-face turn stays dim without becoming a second cursor',()=>{
 const text=notation([{text:'U2',kind:2},{text:'D2',kind:1},{text:'F',kind:0}]);assert.equal((text.match(/underline=/g)||[]).length,1);assert.match(text,/alpha="35%">D2/);assert.match(notation([{text:'<>&',kind:3}]),/&lt;&gt;&amp;/);
});

test('cursor and partial highlights do not add glyphs or change token widths',()=>{
 for(const kind of [2,4]){const html=notation([{text:"R2",kind}]);assert.equal(html.replace(/<[^>]+>/g,''),'R2');assert.ok(!html.includes('background='));assert.match(html,/underline="single"/);}
});

test('scramble ready requires a connected cube and every move complete',()=>{
 const ready={connected:true,running:false,tokens:[{text:'U2',kind:1},{text:'D2',kind:1}]};
 assert.equal(scrambleReady(ready),true);
 for(const kind of [0,2,3,4]) assert.equal(scrambleReady({...ready,tokens:[ready.tokens[0],{text:'D2',kind}]}),false);
 for(const change of [{connected:false},{running:true},{tokens:[]},{tokens:null}]) assert.equal(scrambleReady({...ready,...change}),false);
 assert.equal(scrambleReady(null),false);
});
