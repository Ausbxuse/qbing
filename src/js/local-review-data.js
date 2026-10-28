/* Read-only review summaries. Upstream reconstruction and times remain authoritative. */
var localReviewData = (function() {
 'use strict';
 var phaseNames = ['Cross', 'F2L', 'OLL', 'PLL'];
 var phaseGroups = [[6], [5, 4, 3, 2], [1], [0]];
 function finite(value) { return typeof value === 'number' && isFinite(value); }
 function groupPhases(reconstruction) {
  var data = reconstruction && reconstruction.data;
  if (!Array.isArray(data) || data.length < 7) return null;
  for (var i = 0; i < 7; i++) {
   var p = data[i];
   if (!Array.isArray(p) || p.length < 4 || !p.slice(0, 4).every(finite) || p[0] < 0 || p[1] < p[0] || p[2] < p[1] || p[3] < 0) return null;
  }
  return phaseGroups.map(function(indices, order) {
   return indices.reduce(function(total, index) {
    var p = data[index]; total.time += p[2] - p[0]; total.pause += p[1] - p[0]; total.execution += p[2] - p[1]; total.moves += p[3];
    return total;
   }, { name: phaseNames[order], time: 0, pause: 0, execution: 0, moves: 0 });
  });
 }
 // The last 50 session records form the window. DNFs/manual/malformed records
 // stay out of the phase baseline; +2 never adds artificial execution time.
 function phaseBaseline(source, windowSize) {
  var end = source.length, start = Math.max(0, end - windowSize), samples = [];
  for (var i = start; i < end; i++) {
   var record = source.recordAt(i);
   if (!record || !record[0] || record[0][0] < 0 || !finite(record[0][1]) || record[0][1] <= 0 || !record[4] || !record[4][0]) continue;
   var groups = groupPhases(source.reconstructionAt(i));
   if (groups) samples.push(groups);
  }
  return phaseNames.map(function(_, order) {
   var values = samples.map(function(groups) { return groups[order].time; }).sort(function(a, b) { return a - b; });
   var middle = Math.floor(values.length / 2);
   return { samples: values.length, window: end - start, median: values.length < 3 ? null : values.length % 2 ? values[middle] : (values[middle - 1] + values[middle]) / 2 };
  });
 }
 function recentTimes(source, windowSize) {
  var points = [];
  for (var i = Math.max(0, source.length - windowSize); i < source.length; i++) {
   var value = source.timeAt(i);
   points.push({ index: i, time: finite(value) && value >= 0 ? value : null });
  }
  return points;
 }
 // Filtering only chooses presentation indices. It never sorts or mutates the
 // scorebook, and formats times only when a text query actually needs them.
 function historyIndices(source, filter, query) {
  var result = [], text = String(query || '').trim().toLocaleLowerCase();
  var number = /^#(\d+)$/.exec(text), terms = text.split(/\s+/).filter(Boolean);
  for (var i = source.length - 1; i >= 0; i--) {
   var record = source.recordAt(i);
   if (!Array.isArray(record) || !Array.isArray(record[0])) continue;
   var note = String(record[2] || '').trim();
   if (filter === 'moves' && !(record[4] && record[4][0])) continue;
   if (filter === 'dnf' && !(record[0][0] < 0)) continue;
   if (filter === 'notes' && !note) continue;
   var tags = record[5] && record[5].localReview || {};
   if (filter === 'favorites' && tags.favorite !== true) continue;
   if (source.tagFilter && source.tagFilter !== 'all' && source.tagFilter !== 'color:' + tags.color && source.tagFilter !== 'group:' + tags.group) continue;
   if (number) { if (i + 1 !== Number(number[1])) continue; }
   else if (terms.length) {
    var haystack = [i + 1, source.timeTextAt(i), note, record[1] || '', tags.favorite ? 'favorite' : '', tags.color || '', tags.group || ''].join(' ').toLocaleLowerCase();
    if (!terms.every(function(term) { return haystack.indexOf(term) >= 0; })) continue;
   }
   result.push(i);
  }
  return result;
 }
 // Equal-width, half-open bins in milliseconds, using upstream scored times
 // (including +2). DNFs are counted separately; non-finite data is excluded.
 function timeDistribution(values) {
  var times = [], dnf = 0;
  values.forEach(function(value) { if (!finite(value)) return; if (value < 0) dnf++; else times.push(value); });
  if (!times.length) return { bins: [], count: 0, dnf: dnf, step: 0 };
  var low = Infinity, high = 0;
  times.forEach(function(time) { low = Math.min(low, time); high = Math.max(high, time); });
  var raw = Math.max(100, (high - low) / 8), power = Math.pow(10, Math.floor(Math.log(raw) / Math.LN10));
  var step = [1, 2, 5, 10].filter(function(n) { return n * power >= raw; })[0] * power;
  var start = Math.floor(low / step) * step, bins = [];
  for (var i = 0; i <= Math.floor((high - start) / step); i++) bins.push({ from: start + i * step, to: start + (i + 1) * step, count: 0 });
  times.forEach(function(time) { bins[Math.floor((time - start) / step)].count++; });
  return { bins: bins, count: times.length, dnf: dnf, step: step };
 }
 // Descriptive statistics of the same scored times as the histogram. No
 // reconstruction is needed; invalid values never enter the denominator.
 function timeSummary(values) {
  var times = [], dnf = 0, mean = 0, m2 = 0;
  values.forEach(function(value) {
   if (!finite(value)) return;
   if (value < 0) { dnf++; return; }
   times.push(value);
   var delta = value - mean; mean += delta / times.length; m2 += delta * (value - mean);
  });
  times.sort(function(a, b) { return a - b; });
  var count = times.length, mid = Math.floor(count / 2);
  return { count: count, dnf: dnf, median: count ? count % 2 ? times[mid] : (times[mid - 1] + times[mid]) / 2 : null,
   spread: count > 1 ? Math.sqrt(Math.max(0, m2 / count)) : null,
   p90: count ? times[Math.ceil(count * 0.9) - 1] : null, dnfRate: count + dnf ? dnf / (count + dnf) : null };
 }
 // Best refers to one scored solve; Mean uses all non-DNF solves and only
 // genuine reconstruction for sensor means. Penalties never inflate TPS.
 function sessionMetrics(source) {
  var best = null, bestIndex = -1, sum = 0, count = 0, tps = 0, moves = 0, samples = 0, moveSamples = 0;
  for (var i = 0; i < source.length; i++) {
   var time = source.timeAt(i);
   if (!finite(time) || time < 0) continue;
   sum += time; count++;
   if (best == null || time < best) { best = time; bestIndex = i; }
   var metric = source.metricsAt(i);
   if (metric && finite(metric.moves)) { moves += metric.moves; moveSamples++; }
   if (metric && finite(metric.tps)) { tps += metric.tps; samples++; }
  }
  return { best: best, bestIndex: bestIndex, mean: count ? sum / count : null, count: count,
   tps: samples ? tps / samples : null, moves: moveSamples ? moves / moveSamples : null, samples: samples, moveSamples: moveSamples };
 }
 var api = { groupPhases: groupPhases, phaseBaseline: phaseBaseline, recentTimes: recentTimes, historyIndices: historyIndices, timeDistribution: timeDistribution, timeSummary: timeSummary, sessionMetrics: sessionMetrics };
 if (typeof module !== 'undefined' && module.exports) module.exports = api;
 return api;
})();
