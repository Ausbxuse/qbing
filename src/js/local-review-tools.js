/* Read-only solve review. Solvers and case identity stay upstream-owned. */
var LocalReviewTools = (function() {
 'use strict';
 function tokens(algorithm) {
  return String(algorithm || '').trim().split(/\s+/).filter(Boolean).map(function(m) {
   return m.replace(/^([urfdlb])/, function(_, f) { return f.toUpperCase() + 'w'; }).replace(/2'$/, '2');
  });
 }
 function technique(record) {
  var sequence = record && record[4] && record[4][0];
  if (typeof sequence !== 'string') return null;
  var moves = sequence.split(/\s+/).map(function(m) { var match = /^(.*?)@(\d+)$/.exec(m); return match ? {move:match[1],time:Number(match[2])} : null; }).filter(Boolean);
  var gap = null;
  for (var i = 1; i < moves.length; i++) {
   var duration = moves[i].time - moves[i - 1].time;
   if (duration >= 0 && (!gap || duration > gap.time)) gap = {time:duration, before:moves[i - 1].move, after:moves[i].move};
  }
  return {gap:gap, u2:moves.filter(function(m) { return /^U2'?$/i.test(m.move); }).length,
   rotations:moves.filter(function(m) { return /^[xyz]/.test(m.move); }).length,
   trigger:moves.map(function(m) { return m.move; }).join(' ').indexOf("R U R' U'") >= 0};
 }
 function create(host, options) {
  var body = host.querySelector('#review-tool-content'), current = null, key = '', tab = 'cross', stage = 'PLL', mode = 'cross', face = 1;
  var worker, job = 0, pending = null, routeCache = {}, algCache = {}, catalogPromise;
  function el(tag, className, text) { var e=document.createElement(tag); if(className)e.className=className; if(text!=null)e.textContent=text; return e; }
  function note(text) { body.appendChild(el('p','tool-note',text)); }
  function button(text, fn, className) { var b=el('button',className||'tool-button',text);b.type='button';b.onclick=fn;return b; }
  function copy(text) { $.clipboardCopy(text).then(function(){options.toast('Algorithm copied');}).catch(function(){options.toast('Could not copy');}); }
  function recordReady() { return current && current.record; }
  function reconstruction() { try { return stats.getExtraInfo('recons_cf4op',current.index); } catch (_) { return null; } }
  function crossKey() { return JSON.stringify([current.record[1],mode,face]); }
  function renderCross() {
   if (!recordReady()) { note('Finish a solve to compare cross routes.'); return; }
   var inputTokens=tokens(current.record[1]);
   if (!inputTokens.length||!inputTokens.every(function(m){return /^([URFDLB]w?|[EMSyxz])(['2]?)$/.test(m);})) {note('No usable 3×3 scramble recorded for this solve.');return;}
   if ((current.record[4] && current.record[4][1] !== '333') || !tools.isPuzzle('333')) { note('Cross review is available for 3×3 solves.');return; }
   var controls=el('div','tool-controls'), kind=el('select'), side=el('select');
   kind.setAttribute('aria-label','Cross route type');
   [['cross','Cross'],['xcross','X-cross']].forEach(function(v){var o=el('option','',v[1]);o.value=v[0];kind.appendChild(o);});kind.value=mode;
   kind.onchange=function(){mode=this.value;render();};controls.appendChild(kind);
   side.setAttribute('aria-label','X-cross face');
   ['D · yellow','U · white','L · orange','R · red','F · green','B · blue'].forEach(function(v,i){var o=el('option','',v);o.value=i;side.appendChild(o);});side.value=face;
   side.onchange=function(){face=Number(this.value);render();};side.hidden=mode!=='xcross';controls.appendChild(side);body.appendChild(controls);
   var cacheKey=crossKey(), cached=routeCache[cacheKey];
   if(cached) {
    if(cached.error){note(cached.error);body.appendChild(button('Try again',function(){delete routeCache[cacheKey];solveCross(cacheKey);}));return;}
    cached.routes.forEach(function(route){
     var item=el('div','tool-route'), header=el('div','tool-route-heading');header.append(el('strong','',route.face+' '+(mode==='xcross'?'X-cross':'cross')),el('span','',route.length+' moves'));item.appendChild(header);
     var algorithm=[route.rotation].concat(route.moves).filter(Boolean).join(' ');
     var value=el('code','tool-alg',algorithm||'Already solved');item.appendChild(value);
     if(algorithm)item.appendChild(button('Copy',function(){copy(algorithm);},'tool-copy'));body.appendChild(item);
    });
   } else {
     var run=button(pending?'Finding routes…':'Find '+(mode==='xcross'?'X-cross':'cross')+' routes',function(){solveCross(cacheKey);});run.disabled=!!pending;body.appendChild(run);
   }
   note('Routes use this solve’s scramble. Rotation prefixes set up the displayed face.');
  }
  function solveCross(cacheKey) {
   if(!recordReady()||timer.status()>0)return;
   if(pending)return;
   try {
    if(!worker){worker=new Worker('js/local-cross-worker.js');worker.onmessage=function(e){
     if(!pending||e.data.id!==pending.id)return;
     clearTimeout(pending.timeout);routeCache[pending.key]=e.data;pending=null;if(tab==='cross')render();
    };worker.onerror=function(){failCross('Solver unavailable. Try again.');};}
    var id=++job;
    pending={id:id,key:cacheKey,timeout:setTimeout(function(){failCross('Search timed out. Try a cross route first.');},30000)};
    worker.postMessage({id:id,kind:mode,face:face,moves:cubeutil.parseScramble(current.record[1],'FRUBLD')});render();
   } catch(_){failCross('Could not read this scramble.');}
  }
  function failCross(message){if(pending){clearTimeout(pending.timeout);routeCache[pending.key]={error:message};pending=null;}if(worker)worker.terminate();worker=null;if(tab==='cross')render();}
  function goal(cube, method){return method==='PLL'?cube.ca.every(function(v,i){return v===i;})&&cube.ea.every(function(v,i){return v===i*2;}):cubeutil.getProgress(cube,'cfop')<=1;}
  // Verify each proposed variant on the reconstructed start, including the
  // required orientation/AUF. No prediction of personal speed is made.
  function align(algorithm, start, method) {
   var best=null, alg=tokens(algorithm), tilts=['','x','x2',"x'",'z',"z'"];
   tilts.forEach(function(tilt){for(var y=0;y<4;y++)for(var pre=0;pre<4;pre++){
    var prefix=[tilt,['','y','y2',"y'"][y],['','U','U2',"U'"][pre]].filter(Boolean), cube=new mathlib.CubieCube().init(start.ca,start.ea);
    prefix.concat(alg).forEach(function(m){cube.selfMoveStr(m);});
    for(var post=0;post<(method==='PLL'?4:1);post++){
     var test=new mathlib.CubieCube().init(cube.ca,cube.ea);test.ori=cube.ori;
     var tail=['','U','U2',"U'"][post];if(tail)test.selfMoveStr(tail);
     if(!goal(test,method))continue;
     var moves=prefix.concat(alg,tail?[tail]:[]), rotations=moves.filter(function(m){return /^[xyz]/.test(m);}).length;
     var notation=moves.join(' '), count=recons.getMoveCnt(notation);
     if(!best||count<best.moves||count===best.moves&&rotations<best.rotations)best={algorithm:notation,moves:count,rotations:rotations};
    }
   }});
   return best;
  }
  function catalog() { return catalogPromise||(catalogPromise=fetch('assets/review-algorithms.json').then(function(r){if(!r.ok)throw Error();return r.json();})); }
  function buildVariants(data, method, caseIndex, rec) {
   var start=new mathlib.CubieCube();start.invFrom(rec.data[method==='PLL'?0:1][4]);
   var identify=cubeutil.getIdentData(method)[0], variants=[], seen={};
   data.cases.filter(function(c){return c.stage===method;}).forEach(function(item){item.variants.forEach(function(algorithm){
    var inverse=new mathlib.CubieCube(), seq=tokens(algorithm);
    for(var i=seq.length-1;i>=0;i--)inverse.selfMoveStr(seq[i],true);
    inverse.selfConj();if(identify(inverse.toFaceCube())!==caseIndex)return;
    var candidate=align(algorithm,start,method);if(!candidate||seen[candidate.algorithm])return;
    seen[candidate.algorithm]=true;candidate.name=item.name;variants.push(candidate);
   });});
   return variants.sort(function(a,b){return a.moves-b.moves||a.rotations-b.rotations;}).slice(0,3);
  }
  function renderAlgorithms() {
   if(!recordReady()){note('Record moves to compare case algorithms.');return;}
   var controls=el('div','tool-controls');['OLL','PLL'].forEach(function(name){var b=button(name,function(){stage=name;render();},'tool-tab');b.setAttribute('aria-pressed',String(stage===name));controls.appendChild(b);});body.appendChild(controls);
   var rec=reconstruction(), extra;
   try {extra=stats.getExtraInfo('recons_cf4op_'+stage,current.index);}catch(_){}
   if(!rec||!extra||extra[0]<0||extra[0]>=cubeutil.getIdentData(stage)[3]||!rec.data[stage==='PLL'?0:1][3]){note('No '+stage+' case reconstructed for this solve.');return;}
   var label=cubeutil.getIdentData(stage)[1](extra[0])[2];
   body.appendChild(el('h3','tool-case-title',stage==='PLL'?label+' perm':label));
   var own=rec.data[stage==='PLL'?0:1], ownAlg=own[5].map(function(move){return move[0].trim();}).join(' ');
   var played=el('div','tool-played');played.append(el('span','tool-caption','Your route · '+own[3]+' moves'),el('code','tool-alg',ownAlg));body.appendChild(played);
   var requestStage=stage, cacheKey=key+':'+stage, entry=algCache[cacheKey];
   if(!entry){
    note('Checking case variants…');algCache[cacheKey]={loading:true};
    catalog().then(function(data){return new Promise(function(resolve){setTimeout(function(){try{resolve(buildVariants(data,requestStage,extra[0],rec));}catch(_){resolve([]);}},0);});}).then(function(variants){algCache[cacheKey]={variants:variants};if(key+':'+stage===cacheKey&&tab==='algorithms')render();}).catch(function(){algCache[cacheKey]={error:true};if(key+':'+stage===cacheKey)render();});
    return;
   }
   if(entry.loading){note('Checking case variants…');return;}
   if(entry.error||!entry.variants.length){note('No verified variant available for this reconstruction.');return;}
   var alternatives=entry.variants.filter(function(v){return tokens(v.algorithm).join(' ')!==tokens(ownAlg).join(' ');});
   if(alternatives.length)body.appendChild(el('p','tool-caption','Variants to test'));
   else note('Your route already matches this catalog option.');
   alternatives.forEach(function(variant){
    var item=el('div','tool-route'), fewer=own[3]-variant.moves;
    item.append(el('div','tool-caption',variant.moves+' moves'+(fewer>0?' · '+fewer+' fewer':'')+' · '+variant.rotations+' rotations'),el('code','tool-alg',variant.algorithm));
    item.appendChild(button('Copy',function(){copy(variant.algorithm);},'tool-copy'));body.appendChild(item);
   });
   body.appendChild(button(stage+' practice',function(){options.drill(stage.toLowerCase());}));
   note('Compare execution in practice; fewer moves may not be faster.');
   var source=el('a','tool-source','Alg-Trainer · MIT');source.href='https://github.com/tao-yu/Alg-Trainer';source.target='_blank';source.rel='noopener noreferrer';body.appendChild(source);
  }
  function renderTechnique() {
   var measured=recordReady()&&technique(current.record);
   if(!measured){note('Recorded turns are needed for technique review.');return;}
   var phases=current.phases||[], slow=phases.slice().sort(function(a,b){return b.pause-a.pause;})[0];
   if(slow&&slow.pause>0){body.appendChild(el('h3','tool-case-title',slow.name+' · '+options.pretty(slow.pause)+'s pause'));note('Plan the next '+(slow.name==='F2L'?'pair':'step')+' before finishing the current one. Try slow, continuous turns.');}
   if(measured.gap){body.appendChild(el('p','tool-caption','Largest move gap · '+options.pretty(measured.gap.time)+'s'));body.appendChild(el('code','tool-alg',measured.gap.before+' → '+measured.gap.after));}
   if(measured.u2){body.appendChild(el('h3','tool-case-title','U2 × '+measured.u2));note('Try an index–middle double flick for U2. Compare controlled repetitions before timing.');}
   if(measured.trigger){body.appendChild(el('code','tool-alg',"R U R' U'"));note('Practice this trigger with a stable right-thumb grip and relaxed U/U′ flicks.');}
   if(measured.rotations){note(measured.rotations+' recorded rotations. Compare a rotation-free variant where it stays comfortable.');}
   if(!measured.u2&&!measured.trigger&&!measured.rotations)note('Use replay to inspect the move gap and test a smoother transition.');
   note('Move timing shows gaps; it does not measure your fingers or grip.');
  }
  function render(){body.replaceChildren();host.querySelectorAll('[data-review-tool]').forEach(function(b){b.setAttribute('aria-pressed',String(b.dataset.reviewTool===tab));});if(tab==='cross')renderCross();else if(tab==='algorithms')renderAlgorithms();else renderTechnique();}
  host.querySelectorAll('[data-review-tool]').forEach(function(b){b.onclick=function(){tab=b.dataset.reviewTool;render();};});
  return { update:function(value){current=value;var next=JSON.stringify([kernel.getProp('session'),value.index,value.record&&value.record[0],value.record&&value.record[1],value.record&&value.record[4]]);host.querySelector('#review-tool-solve').textContent=value.record?'#'+(value.index+1):'';if(key!==next){key=next;render();}},
   choose:function(name,method){tab=name;if(method==='PLL'||method==='OLL')stage=method;render();} };
 }
 var api={create:create,tokens:tokens,technique:technique};
 if(typeof module!=='undefined'&&module.exports)module.exports=api;
 return api;
})();
