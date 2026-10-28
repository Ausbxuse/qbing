/* Run upstream csTimer solvers off the timing/UI thread. */
self.DEBUG = false; self.ISCSTIMER = false;
self.$ = { isArray: Array.isArray, noop: function() {} };
importScripts('../vendor/solver/isaac.js', '../vendor/solver/mathlib.js', '../vendor/solver/cross.js');
self.onmessage = function(event) {
 var input = event.data;
 try {
  var faces = 'DULRFB', rotations = ['', 'z2', "z'", 'z', "x'", 'x'];
  var solutions = input.kind === 'xcross' ? [cross.solveXCross(input.moves, input.face)] : cross.solve(input.moves);
  self.postMessage({ id: input.id, routes: solutions.map(function(moves, i) {
   var face = input.kind === 'xcross' ? input.face : i;
   return { face: faces[face], rotation: rotations[face], moves: moves.map(function(m) { return m.trim(); }), length: moves.length };
  }) });
 } catch (_) { self.postMessage({id: input.id, error: 'Could not solve this scramble'}); }
};
