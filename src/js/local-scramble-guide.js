/* A presentation guide using csTimer's cubie model. It never starts a solve or
   changes the target scramble; the original matcher remains authoritative. */
(function(root, factory) {
 if (typeof module !== 'undefined' && module.exports) module.exports = factory;
 else root.LocalScrambleGuide = factory(mathlib.CubieCube, cubeutil.getConjMoves);
})(typeof window === 'undefined' ? globalThis : window, function(Cube, conjugate) {
 'use strict';
 var faces = 'URFDLB';
 function move(face, power) { return face + ['', '', '2', "'"][power]; }
 function inverse(text) { return text.endsWith('2') ? text : text.endsWith("'") ? text.slice(0, -1) : text + "'"; }
 function apply(state, moves) { var cube = new Cube(); cube.fromFacelet(state); moves.forEach(function(m) { cube.selfMoveStr(m); }); return cube.toFaceCube(); }
 function simplify(list) {
  var blocks = [];
  list.forEach(function(m) {
   var face = faces.indexOf(m[0]), power = m.endsWith('2') ? 2 : m.endsWith("'") ? 3 : 1;
   var block = blocks[blocks.length - 1];
   if (!block || block.axis !== face % 3) { block = { axis: face % 3, powers: {} }; blocks.push(block); }
   block.powers[m[0]] = ((block.powers[m[0]] || 0) + power) % 4;
   if (Object.keys(block.powers).every(function(f) { return block.powers[f] === 0; })) blocks.pop();
  });
  return blocks.flatMap(function(block) { return Object.keys(block.powers).filter(function(f) { return block.powers[f]; }).map(function(f) { return move(f, block.powers[f]); }); });
 }
 function Guide(scramble) {
  this.enabled = false; this.entries = new Map(); this.recovery = [];
  var solved = new Cube().toFaceCube(); this.last = solved; this.checkpoint = solved; this.trusted = true;
  var original = scramble.trim().split(/\s+/), physical = conjugate(scramble).trim().split(/\s+/);
  if (!physical.length || physical.length !== original.length || !physical.every(function(m) { return /^[URFDLB](2|')?$/.test(m); })) return;
  this.original = original;
  var cube = new Cube(), self = this;
  for (var start = 0; start < physical.length;) {
   var end = start + 1, axis = faces.indexOf(physical[start][0]) % 3;
   while (end < physical.length && faces.indexOf(physical[end][0]) % 3 === axis) end++;
   var block = physical.slice(start, end), letters = block.map(function(m) { return m[0]; });
   if (new Set(letters).size !== letters.length) return; // defer uncommon repeated-axis groups to upstream
   var options = letters.map(function() { return [0, 1, 2, 3]; });
   var combinations = options.reduce(function(rows, values) { return rows.flatMap(function(row) { return values.map(function(value) { return row.concat(value); }); }); }, [[]]);
   var base = cube.toFaceCube();
   combinations.forEach(function(powers) {
    var moves = letters.map(function(f, i) { return powers[i] ? move(f, powers[i]) : ''; }).filter(Boolean);
    var state = apply(base, moves), done = 0, cursor = end;
    var tokens = original.map(function(text, index) { return { text: text, kind: index < start ? 'done' : '', partial: false }; });
    block.forEach(function(target, offset) {
     var power = target.endsWith('2') ? 2 : target.endsWith("'") ? 3 : 1;
     var remaining = (power - powers[offset] + 4) % 4, index = start + offset;
     if (!remaining) { tokens[index].kind = 'done'; done++; }
     else {
      cursor = Math.min(cursor, index);
      if (powers[offset]) { tokens[index].text = conjugate(move(target[0], remaining), true); tokens[index].partial = power === 2 && remaining % 2 === 1; }
     }
    });
    if (cursor < original.length) tokens[cursor].kind = 'current';
    var entry = { tokens: tokens, cursor: cursor, score: start + done };
    var previous = self.entries.get(state);
    if (!previous || entry.score >= previous.score) self.entries.set(state, entry);
   });
   block.forEach(function(m) { cube.selfMoveStr(m); });
   start = end;
  }
  this.enabled = true;
  this.entry = this.entries.get(solved);
 }
 Guide.prototype.update = function(state, newestMoves) {
  if (!this.enabled || !/^[URFDLB]{54}$/.test(state)) return null;
  var matched = this.entries.get(state);
  if (matched) {
   this.entry = matched; this.checkpoint = state; this.recovery = []; this.trusted = true;
  } else if (this.trusted && this.entry && state !== this.last) {
   var sequence = null, reported = newestMoves || [];
   for (var count = 1; count <= reported.length; count++) {
    var candidate = reported.slice(0, count).reverse().map(function(m) { return m.trim(); });
    if (!candidate.every(function(m) { return /^[URFDLB](2|')?$/.test(m); })) continue;
    if (apply(this.last, candidate) === state) { sequence = candidate; break; }
   }
   if (!sequence) this.trusted = false;
   else this.recovery = simplify(this.recovery.concat(sequence));
  }
  this.last = state;
  if (!this.trusted || !this.entry) return null;
  var tokens = this.entry.tokens.map(function(token) { return Object.assign({}, token); });
  if (this.recovery.length) {
   var undo = this.recovery.slice().reverse().map(inverse);
   if (apply(state, undo) !== this.checkpoint) { this.trusted = false; return null; }
   tokens.splice(this.entry.cursor, 0, { text: '↶ ' + conjugate(undo.join(' '), true), kind: 'correction', partial: false });
  }
  return tokens;
 };
 return Guide;
});
