/* Reconstruction evidence and state-verified case alternatives. GPL-3.0-only. */
var reviewCorners=[[8,9,20],[6,18,38],[0,36,47],[2,45,11],[29,26,15],[27,44,24],[33,53,42],[35,17,51]];
var reviewEdges=[[5,10],[7,19],[3,37],[1,46],[32,16],[28,25],[30,43],[34,52],[23,12],[21,41],[50,39],[48,14]];
function reviewSlots(face) {
 var solved=mathlib.SOLVED_FACELET;
 return reviewCorners.map(function(ids,i){var colors=ids.map(function(n){return solved[n];});if(colors.indexOf(face)<0)return null;
  var sides=colors.filter(function(c){return c!==face;}),edge=reviewEdges.findIndex(function(e){return e.every(function(n){return sides.indexOf(solved[n])>=0;});});
  return {corner:i,edge:edge,name:colors.join('')};
 }).filter(function(s){return s!==null;});
}
function reviewPairSolved(c,s){return c.ca[s.corner]===s.corner&&c.ea[s.edge]===s.edge*2;}
function reviewCrossSolved(c,face){return reviewEdges.every(function(e,i){return !e.some(function(n){return mathlib.SOLVED_FACELET[n]===face;})||c.ea[i]===i*2;});}
function reviewF2LVariants(start,face,targets,slots) {
 var preserved=slots.filter(function(s){return reviewPairSolved(start,s);}),seen={},out=[];
 var top='URFDLB'.indexOf({U:'D',D:'U',R:'L',L:'R',F:'B',B:'F'}[face]);
 nativeF2LCatalog.forEach(function(item){item.variants.forEach(function(algorithm,variantIndex){
  var best=null;
  ['', 'x','x2',"x'",'z',"z'"].forEach(function(tilt){['','y','y2',"y'"].forEach(function(y){
   var oriented=new mathlib.CubieCube().init(start.ca,start.ea);[tilt,y].filter(Boolean).forEach(function(m){oriented.selfMoveStr(m);});
   if(Math.floor(mathlib.CubieCube.rotMulM[oriented.ori||0][0]/3)!==top)return;
   ['','U','U2',"U'"].forEach(function(pre){
    var c=new mathlib.CubieCube().init(start.ca,start.ea),seq=[tilt,y,pre].filter(Boolean).concat(tokens(algorithm));
    seq.forEach(function(m){c.selfMoveStr(m);});
    if(!reviewCrossSolved(c,face)||!targets.every(function(s){return reviewPairSolved(c,s);})||!preserved.every(function(s){return reviewPairSolved(c,s);}))return;
    var moves=recons.getMoveCnt(seq.join(' ')),rotations=seq.filter(function(m){return /^[xyz]/.test(m);}).length;
    if(!best||moves<best.moves||moves===best.moves&&rotations<best.rotations)best={name:item.name,caseId:item.id,variantIndex:variantIndex,algorithm:seq.join(' '),moves:moves,rotations:rotations,verified:true,endState:c.toFaceCube()};
   });
  });});
  if(best&&!seen[best.algorithm]){seen[best.algorithm]=true;out.push(best);}
 });});
 return out.sort(function(a,b){return a.moves-b.moves||a.rotations-b.rotations;});
}
var reviewViewRotations={U:'z2',D:'',F:"x'",B:'x',R:'z',L:"z'"};
var reviewInverseView={U:'z2',D:'',F:'x',B:"x'",R:"z'",L:'z'};
var reviewRotationNames=null;
function reviewViewAlgorithm(algorithm,face,stage){
 var moves=tokens(algorithm);
 if(stage==='Cross'){
  var frame={U:'DLFURB',F:'BRUFLD',B:'FRDBLU',R:'LUFRDB',L:'RDFLUB',D:'URFDLB'}[face];
  return moves.map(function(m){return 'URFDLB'[frame.indexOf(m[0])]+m.slice(1);}).join(' ');
 }
 if(!reviewRotationNames){
  reviewRotationNames={0:[]};var queue=[{ori:0,moves:[]}];
  for(var i=0;i<queue.length;i++)['x',"x'",'x2','y',"y'",'y2','z',"z'",'z2'].forEach(function(m){var c=new mathlib.CubieCube();c.ori=queue[i].ori;c.selfMoveStr(m);if(reviewRotationNames[c.ori]===undefined){var seq=queue[i].moves.concat(m);reviewRotationNames[c.ori]=seq;queue.push({ori:c.ori,moves:seq});}});
 }
 var seq=(reviewInverseView[face]?[reviewInverseView[face]]:[]).concat(moves),c=new mathlib.CubieCube(),at=0;
 while(at<seq.length&&/^[xyz]/.test(seq[at]))c.selfMoveStr(seq[at++]);
 return reviewRotationNames[c.ori||0].concat(seq.slice(at)).join(' ');
}
function nativeReview(record) {
 var moves=record.moves||[],splits=record.analysis&&record.analysis.splits||[],face=record.analysis&&record.analysis.cross||record.cross||'D';
 var total=splits.reduce(function(n,s){return n+((s.rawMoves===undefined?s.moves:s.rawMoves)||0);},0);
 if(total!==moves.length||!splits.length)return {groups:[],classified:false};
 var state=nativeCube(record.initialState),states=[new mathlib.CubieCube().init(state.ca,state.ea)],groups=[],offset=0,slots=reviewSlots(face);
 moves.forEach(function(m){state.selfMoveStr(m.move);states.push(new mathlib.CubieCube().init(state.ca,state.ea));});
 function add(stage,label,start,end,targets){
  var own=moves.slice(start,end).map(function(m){return m.move;}).join(' '),variants=[],names=[],ids=[],group={stage:stage,label:label,rawStart:start,rawEnd:end,own:own,variants:variants};
  if(start===end){group.caseName='Skipped';groups.push(group);return;}
  if(stage==='F2L'&&targets.length)variants=reviewF2LVariants(states[start],face,targets,slots);
  else if(stage==='Cross'){
   var route=JSON.parse(nativeRequest(JSON.stringify({route:'native-cross-dashboard',body:{state:states[start].toFaceCube(),faces:face}})));
   variants=route.routes.map(function(r){return {name:r.kind==='xcross'?'Xcross':'Cross',algorithm:r.fixedMoves.join(' '),moves:r.fixedMoves.length,verified:r.verified};});
  }else if(stage==='OLL'||stage==='PLL'){
   // The existing verifier tests orientation/AUF and the complete phase goal.
   var data=JSON.parse(nativeRequest(JSON.stringify({route:'native-algorithms',body:{record:record,stage:stage}})));
   variants=data.variants.map(function(v){v.verified=true;return v;});
   (data.matches||[]).forEach(function(m){if(ids.indexOf(m.id)<0)ids.push(m.id);if(names.indexOf(m.name)<0)names.push(m.name);});
   if(data.caseId&&ids.indexOf(data.caseId)<0)ids.push(data.caseId);
  }
  variants.forEach(function(v){if(v.caseId&&ids.indexOf(v.caseId)<0)ids.push(v.caseId);if(v.name&&v.caseId&&names.indexOf(v.name)<0)names.push(v.name);});
  if(stage==='PLL'&&!ids.length){
   var top={U:'D',D:'U',R:'L',L:'R',F:'B',B:'F'}[face];
   ['', '2', "'"].forEach(function(suffix){var c=new mathlib.CubieCube().init(states[start].ca,states[start].ea);c.selfMoveStr(top+suffix);if(c.ca.every(function(v,i){return v===i;})&&c.ea.every(function(v,i){return v===i*2;}))variants.push({name:'AUF',algorithm:top+suffix,moves:1,verified:true});});
  }
  variants.forEach(function(v){v.viewAlgorithm=reviewViewAlgorithm(v.algorithm,face,stage);});
  group.variants=variants;group.caseIds=ids;
  group.caseName=stage==='Cross'?'Cross':stage==='PLL'&&variants.some(function(v){return v.name==='AUF';})?'AUF':ids.length===1?(names[0]||ids[0]):ids.length>1?'Multiple matching patterns':'Unclassified';
  groups.push(group);
 }
 splits.forEach(function(split){
  var n=split.rawMoves===undefined?split.moves:split.rawMoves,end=offset+n,stage=split.stage;
  if(stage!=='F2L'||!n){add(stage,stage,offset,end,[]);offset=end;return;}
  var events={};
  slots.forEach(function(slot){
   if(!reviewPairSolved(states[end],slot))return;
   // Opening a face for an insertion temporarily displaces already solved slots.
   // Compare cross-restored checkpoints, not those intermediate trigger states.
   var last=-1;for(var i=offset;i<end;i++)if(reviewCrossSolved(states[i],face)&&!reviewPairSolved(states[i],slot))last=i;
   if(last<0)return;
   var at=last+1;while(at<end&&(!reviewCrossSolved(states[at],face)||!reviewPairSolved(states[at],slot)))at++;
   (events[at]||(events[at]=[])).push(slot);
  });
  var before=offset,count=0;
  Object.keys(events).map(Number).sort(function(a,b){return a-b;}).forEach(function(at){count++;add('F2L','Pair '+count,before,at,events[at]);before=at;});
  if(before<end)add('F2L','F2L',before,end,slots.filter(function(s){return !reviewPairSolved(states[before],s)&&reviewPairSolved(states[end],s);}));
  offset=end;
 });
 return {groups:groups,classified:true};
}
