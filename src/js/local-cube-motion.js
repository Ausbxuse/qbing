/* A bounded visual queue. It never owns timing, move counts, or cube state. */
(function(root, factory) {
 var api = factory();
 if (typeof module !== 'undefined' && module.exports) module.exports = api;
 else root.LocalCubeMotion = api;
})(globalThis, function() {
 'use strict';
 function Motion(state, apply) {
  this.display = this.target = state; this.apply = apply; this.queue = []; this.active = null;
  this.lastSteps = []; this.revision = 0; this.serial = 0; this.maxLag = 120; this.deadline = 0;
 }
 Motion.prototype.snap = function(state) {
  this.display = this.target = state; this.active = null; this.queue.length = 0; this.deadline = 0; this.revision++;
 };
 Motion.prototype.accept = function(state, moves, now, animate) {
  this.lastSteps = [];
  if (state === this.target) return false;
  var sequence = null, source = Array.isArray(moves) ? moves.slice(0, 8).map(function(m) { return String(m).trim(); }) : [];
  // GAN packets contain the newest move first, then recent history. Validate
  // the shortest suffix we missed against the last accepted authoritative state.
  for (var size = 1; size <= source.length; size++) {
   var candidate = source.slice(0, size).reverse(), next = this.target, steps = [];
   for (var i = 0; i < candidate.length; i++) {
    if (!/^[URFDLB][2']?$/.test(candidate[i])) { steps = []; break; }
    next = this.apply(next, candidate[i]); if (!next) { steps = []; break; }
    steps.push({move:candidate[i], state:next, turns:candidate[i].endsWith('2')?2:candidate[i].endsWith("'")?-1:1});
   }
   if (steps.length && next === state) { sequence = steps; break; }
  }
  if (!sequence) { this.snap(state); return true; }
  this.lastSteps = sequence;
  if (!animate) { this.snap(state); return true; }
  this.target = state; this.deadline = now + this.maxLag;
  sequence.forEach(function(step) {
   var tail = this.queue[this.queue.length - 1];
   if (tail && tail.move === step.move && Math.abs(step.turns) === 1) {
    tail.turns += step.turns; tail.move = step.move[0] + '2'; tail.state = step.state;
   } else this.queue.push(step);
  }, this);
  // Never accumulate a replay backlog during a burst or a stalled frame.
  if (this.queue.length > 8) { this.snap(state); return true; }
  if (this.active) {
   var p = Math.max(0, Math.min(1, (now - this.active.start) / this.active.duration));
   var duration = Math.min(this.active.duration, Math.max(16, (this.deadline - now) / (this.queue.length + 1)));
   this.active.start = now - p * duration; this.active.duration = duration;
  }
  return true;
 };
 Motion.prototype.frame = function(now) {
  var completed = [];
  if (this.active && now >= this.active.start + this.active.duration) {
   this.display = this.active.state; completed.push(this.active); this.active = null;
  }
  if (!this.active && this.queue.length) {
   var next = this.queue.shift(), nominal = next.move.endsWith('2') ? 125 : 90;
   next.start = now; next.duration = Math.min(nominal, Math.max(16, (this.deadline - now) / (this.queue.length + 1))); next.id = ++this.serial;
   this.active = next;
  }
  return {completed:completed, active:this.active, progress:this.active ? Math.max(0, Math.min(1, (now - this.active.start) / this.active.duration)) : 0};
 };
 Motion.prototype.busy = function() { return !!this.active || this.queue.length > 0; };
 return {Motion:Motion};
});
