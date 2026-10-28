/* Headless csTimer solver bridge. GPL-3.0-only. No UI, DOM, network or history. */
function nativeMoves(text) { return text.trim().split(/\s+/).filter(function(x){return !!x;}); }
function nativeCube(state) { var c=new mathlib.CubieCube(); if(c.fromFacelet(state)===-1) throw Error('Invalid cube state');return c; }
function nativeApply(state,moves) {var c=nativeCube(state);moves.forEach(function(m){c.selfMoveStr(m);});return c.toFaceCube();}

function nativeVerifiedCrossRoute(state,face,rotation,moves,kind){
 var c=nativeCube(state),fixed=[];
 nativeMoves(rotation).forEach(function(m){c.selfMoveStr(m);});
 moves.forEach(function(m){var index=c.selfMoveStr(m);if(typeof index!=="number")throw Error('Unsupported solver turn');fixed.push('URFDLB'.charAt(Math.floor(index/3))+['','2',"'"][index%3]);});
 var end=c.toFaceCube(),solved=mathlib.SOLVED_FACELET;
 var edges=[[5,10],[7,19],[3,37],[1,46],[32,16],[28,25],[30,43],[34,52],[23,12],[21,41],[50,39],[48,14]];
 var corners=[[8,9,20],[6,18,38],[0,36,47],[2,45,11],[29,26,15],[27,44,24],[33,53,42],[35,17,51]];
 function correct(ids){return ids.every(function(i){return end.charAt(i)===solved.charAt(i);});}
 var ok=edges.filter(function(e){return e.some(function(i){return solved.charAt(i)===face;});}).every(correct);
 if(kind==='xcross')ok=ok&&corners.some(function(corner){
  if(!corner.some(function(i){return solved.charAt(i)===face;})||!correct(corner))return false;
  var sides=corner.map(function(i){return solved.charAt(i);}).filter(function(f){return f!==face;});
  return edges.some(function(e){return e.every(function(i){return sides.indexOf(solved.charAt(i))>=0;})&&correct(e);});
 });
 if(!ok)throw Error('Cross route failed state verification');
 return {face:face,rotation:rotation,moves:moves,length:moves.length,kind:kind,fixedMoves:fixed,verified:true};
}

function nativeRequest(raw) {
 var request=JSON.parse(raw), body=request.body||{}, result;
 if(request.route==='native-review') { return JSON.stringify(nativeReview(body.record)); }
 if(request.route==='native-algorithms') {
  var r=body.record,method=body.stage||'PLL',splits=r.analysis&&r.analysis.splits||[],split=splits.filter(function(s){return s.stage===method;})[0];
  if(!split||!r.moves||!r.moves.length)throw Error('No reconstructed case');
  var before=0;for(var i=0;i<splits.length&&splits[i]!==split;i++)before+=splits[i].rawMoves||splits[i].moves;
  var start=nativeCube(r.initialState);r.moves.slice(0,before).forEach(function(m){start.selfMoveStr(m.move);});
  var own=r.moves.slice(before,before+(split.rawMoves||split.moves)).map(function(m){return m.move;}).join(' '),variants=[],seen={},matches=[];
  var cases=nativeCatalog.cases.filter(function(c){return c.stage===method&&(!split.caseId||c.id===split.caseId);});
  cases.forEach(function(item){item.variants.forEach(function(alg,variantIndex){var candidate=align(alg,start,method);if(candidate&&!matches.some(function(m){return m.id===item.id;}))matches.push({id:item.id,name:item.name});if(!candidate||seen[candidate.algorithm]||tokens(candidate.algorithm).join(' ')===tokens(own).join(' '))return;seen[candidate.algorithm]=true;candidate.name=item.name;candidate.caseId=item.id;candidate.variantIndex=variantIndex;variants.push(candidate);});});
  result={own:own,stage:method,caseId:split.caseId,matches:matches,variants:variants.sort(function(a,b){return a.moves-b.moves||a.rotations-b.rotations;})};
 } else if(request.route==='native-technique') {
  var record=body.record;
  result=technique([null,null,null,null,[record.moves.map(function(m){return m.move+'@'+Math.round(m.t);}).join(' '),'333']]);
 } else if(request.route==='scramble') {
  var state=min2phase.randomCube(), text=new min2phase.Search().solution(state,21,100000000,50,2).trim();
  if(/^Error/.test(text)||nativeApply(mathlib.SOLVED_FACELET,nativeMoves(text))!==state)throw Error('Scramble verification failed');
  result={scramble:text,state:state};
 } else if(request.route==='recovery') {
  var current=nativeCube(body.state),target=nativeCube(body.target),inverse=new mathlib.CubieCube(),relative=new mathlib.CubieCube();
  inverse.invFrom(target);mathlib.CubieCube.CubeMult(inverse,current,relative);
  var text=new min2phase.Search().solution(relative.toFaceCube(),21,100000000,50,0).trim(),moves=nativeMoves(text);
  if(/^Error/.test(text)||nativeApply(body.state,moves)!==body.target)throw Error('Recovery verification failed');
  result={moves:moves};
 } else if(request.route==='cross'||request.route==='native-tools'||request.route==='native-cross-dashboard') {
  var scramble=body.scramble,state=body.state;
  if(state){scramble=new min2phase.Search().solution(state,21,100000000,50,2).trim();if(/^Error/.test(scramble)||nativeApply(mathlib.SOLVED_FACELET,nativeMoves(scramble))!==state)throw Error('Initial-state scramble verification failed');}
  else {if(!scramble)throw Error('No scramble or initial state');state=nativeApply(mathlib.SOLVED_FACELET,nativeMoves(scramble));}
  var parsed=cubeutil.parseScramble(scramble,'FRUBLD');
  var faces='DULRFB',rotations=['','z2',"z'",'z',"x'",'x'];
  var all=cross.solve(parsed),routes=[];
  if(request.route==='native-cross-dashboard') {
   var requested=body.faces||'UD';
   requested.split('').forEach(function(face){var i=faces.indexOf(face);if(i<0)throw Error('Unknown cross face');
    routes.push(nativeVerifiedCrossRoute(state,face,rotations[i],all[i].map(function(m){return m.trim();}),'cross'));
    routes.push(nativeVerifiedCrossRoute(state,face,rotations[i],cross.solveXCross(parsed,i).map(function(m){return m.trim();}),'xcross'));
   });
   result={routes:routes};
  } else {
   var indices=body.kind==='xcross'?[body.face||0]:[0,1,2,3,4,5];
   routes=indices.map(function(i){var moves=body.kind==='xcross'?cross.solveXCross(parsed,i):all[i];return nativeVerifiedCrossRoute(state,faces[i],rotations[i],moves.map(function(m){return m.trim();}),body.kind==='xcross'?'xcross':'cross');});
   if(request.route==='cross') {var r=routes[faces.indexOf(body.cross||'U')];result={distance:r.length,moves:(r.rotation?[r.rotation]:[]).concat(r.moves),cross:r.face};}
   else result={routes:routes};
  }
 } else throw Error('Unsupported local solver request');
 return JSON.stringify(result);
}
