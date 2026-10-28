/* Optional interface over csTimer's timer, statistics and Bluetooth state. */
var localInterface = (function() {
 'use strict';
 var selected = -1, previousLength = -1, view = 'timer', limit = 100;
 var historyFilter = 'all', historyQuery = '', historyTagFilter = 'all', historyMatches = [], historySession = null, historyBest = Infinity, historyFrame;
 var historyDate = new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
 var owner = true, cubeState, scrambleMoves = [], scrambleStart = null, scrambleTps = null;
 var publishTimer, clockTimer, lastClock = '', lastScramble = '', lastConnection = '';
 var toolsAnchor, originalTools, channel, actionMenu;
 var metricsCache = new WeakMap(), updateFrame, pendingUpdate;
 var palettes = ['paper', 'carbon'];
 var paletteLabels = ['Light', 'Dark'];
 var guide, guideTokens = null;
 var selectedPhase = 1, phaseExpanded = false, phaseDetails = [];
 var pacePoints = [], paceMarker, paceDot, sessionMean = null;
 var reviewTools, tagMenu, tagTarget = -1;
 function node(id) { return document.getElementById(id); }
 function text(id, value) { var el = node(id); if (el) el.textContent = value; }
 function finite(value) { return typeof value === 'number' && isFinite(value); }
 function plain(html) { var span = document.createElement('span'); span.innerHTML = html; return span.textContent; }
 function pretty(ms) { return finite(ms) && ms >= 0 ? plain(kernel.pretty(Math.round(ms))) : '—'; }
 function recordMetrics(record, index) {
  if (!record || !record[4] || !record[4][0]) return { moves: null, tps: null };
  var cached = metricsCache.get(record), raw = record[4][0], duration = record[0][1];
  if (cached && cached.raw === raw && cached.duration === duration) return cached.metrics;
  var moves = null;
  try { moves = stats.getExtraInfo('mvcnt_htm', index); if (!finite(moves) || moves < 0) moves = recons.getMoveCnt(record[4][0]); } catch (_) {}
  if (!finite(moves) || moves < 0) return { moves: null, tps: null };
  var value = {moves:moves, tps:duration > 0 ? moves * 1000 / duration : null};
  metricsCache.set(record, {raw:raw, duration:duration, metrics:value}); return value;
 }
 function length() { return stats.getTimesStatsList().timesLen; }
 function record() { return selected >= 0 ? stats.timesAt(selected) : null; }
 function toast(message) { text('toast', message); node('toast').classList.add('visible'); setTimeout(function() { node('toast').classList.remove('visible'); }, 3000); }
 function syncSelect(target, source) {
  if (!source || !source.length) return;
  var options = source[0].cloneNode(true);
  if (target.id === 'session-button') Array.prototype.forEach.call(options.options, function(option) { if (/^\d+$/.test(option.textContent.trim())) option.textContent = 'Session ' + option.textContent.trim(); });
  if (target.innerHTML !== options.innerHTML) target.innerHTML = options.innerHTML;
  target.value = source.val();
 }
 function currentSnapshot() {
  var last = length() ? stats.timesAt(length() - 1) : null;
  var metrics = recordMetrics(last, length() - 1);
  var connection = giikerutil.getViewState();
  var segments = [];
  var source = document.querySelector('#scrambleTxt > div');
  if (guideTokens) segments = guideTokens.map(function(token) { return { text: token.text + (token.partial ? '◐' : '') + ' ', kind: token.kind }; });
  else if (source) {
   var walker = document.createTreeWalker(source, NodeFilter.SHOW_TEXT);
   var current;
   while ((current = walker.nextNode())) {
    var parent = current.parentElement;
    if (parent.closest('.smrtScrMrk')) continue;
    var kind = parent.closest('.smrtScrDim') ? 'done' : parent.closest('.smrtScrCur') ? 'current' : '';
    segments.push({ text: current.textContent + (parent.closest('.smrtScrDim,.smrtScrCur,.smrtScrAct') ? ' ' : ''), kind: kind });
   }
  }
  return { version: 1, owner: window.localInterfaceId, connected: !!connection.connected, device: connection.name,
   scramble: tools.getCurScramble()[1] || '', segments: segments,
   time: last ? plain(stats.pretty(last[0], false)) : '—', tps: metrics.tps,
   moves: metrics.moves, scrambleTps: scrambleTps, running: timer.status() > 0, updatedAt: Date.now() };
 }
 function publish() {
  publishTimer = null;
  if (!owner) return;
  var state = currentSnapshot();
  if (channel) channel.postMessage({ type: 'state', state: state });
  fetch('/api/companion/state', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(state), keepalive: true }).catch(function() {});
 }
 function schedulePublish() { if (!publishTimer) publishTimer = setTimeout(publish, 60); }
 function syncScramble() {
  if (guideTokens && GiikerCube.isConnected()) {
   var signature = JSON.stringify(guideTokens);
   if (signature === lastScramble) return;
   node('scramble-line').replaceChildren();
   guideTokens.forEach(function(token) {
    var span = document.createElement('span'); span.textContent = token.text + (token.partial ? '◐' : '');
    span.className = token.kind === 'done' ? 'smrtScrDim' : token.kind === 'current' ? 'smrtScrCur' : token.kind === 'correction' ? 'local-correction' : '';
    if (token.partial) span.title = 'Half turn: one quarter turn remains';
    node('scramble-line').append(span, ' ');
   });
   lastScramble = signature; schedulePublish(); return;
  }
  var source = document.querySelector('#scrambleTxt > div');
  var html = source ? source.innerHTML : '';
  if (html !== lastScramble) { node('scramble-line').innerHTML = html; lastScramble = html; schedulePublish(); }
 }
 function renderClock() {
  if (!node('local-app')) return;
  var status = timer.status(), running = status > 0;
  document.body.dataset.state = running ? 'solve' : record() ? 'review' : 'scramble';
  document.body.dataset.liveStatus = status === -2 ? 'ready' : status === -3 ? 'inspection' : '';
  var html;
  if ((status === -1 || status === 0) && record()) html = stats.pretty(record()[0], true);
  else {
   var original = document.querySelector('#lcd.activetimer, #multiphase > .activetimer');
   if (original) { var copy = original.cloneNode(true); copy.querySelectorAll('.difflabel,.insplabel').forEach(function(el) { el.remove(); }); html = copy.innerHTML; }
  }
  html = html || kernel.pretty(0, true);
  if (html !== lastClock) { node('clock').innerHTML = html; lastClock = html; }
 }
 function setView(next) {
  var changed = view !== next; view = next;
  document.body.dataset.view = next;
  node('local-practice').hidden = next !== 'practice';
  document.querySelectorAll('#local-app [data-view]').forEach(function(button) {
   var active = button.dataset.view === next; button.classList.toggle('active', active); button.setAttribute('aria-current', active ? 'page' : 'false');
  });
  if (changed) { document.body.dataset.toolsOpen = 'false'; resetHistory(false); }
  if (next === 'practice') { kernel.setProp('tools', true); $(window).trigger('resize'); }
  window.dispatchEvent(new Event('local-cube-redraw'));
 }
 function resetHistory(resetLimit) {
  if (resetLimit) { limit = 100; document.querySelector('.history-scroll').scrollTop = 0; }
  historyMatches = localReviewData.historyIndices({ length: length(), recordAt: stats.timesAt, tagFilter: view === 'history' ? historyTagFilter : 'all', timeTextAt: function(index) { return plain(stats.pretty(stats.timesAt(index)[0], false)); } }, view === 'history' ? historyFilter : 'all', view === 'history' ? historyQuery : '');
  node('history-rows').replaceChildren(); node('history-table').classList.toggle('empty-list', !historyMatches.length); appendHistory();
  var total = length();
  document.querySelector('.history-footer').children[0].textContent = historyMatches.length === total ? total + ' solves' : historyMatches.length + ' / ' + total + ' solves';
  renderDistribution();
  node('history-tag-filter').value = historyTagFilter;
  node('history-query-clear').hidden = !historyQuery;
  document.querySelectorAll('[data-history-filter]').forEach(function(button) { button.setAttribute('aria-pressed', String(button.dataset.historyFilter === historyFilter)); });
 }
 function renderDistribution() {
  if (view !== 'history') return;
  var values = historyMatches.map(function(index) { return stats.timeAt(index); });
  var data = localReviewData.timeDistribution(values), summary = localReviewData.timeSummary(values);
  text('distribution-median', pretty(summary.median));
  text('distribution-spread', pretty(summary.spread));
  text('distribution-p90', pretty(summary.p90));
  text('distribution-dnf', summary.dnfRate == null ? '—' : (100 * summary.dnfRate).toFixed(summary.dnfRate === 0 || summary.dnfRate === 1 ? 0 : 1) + '%');
  var bars = node('distribution-bars'), detail = node('distribution-detail'); bars.replaceChildren();
  function seconds(ms) { return String(Math.round(ms) / 1000); }
  var caption = data.count + ' solves' + (data.dnf ? ' · ' + data.dnf + ' DNF' : '') + (data.step ? ' · ' + seconds(data.step) + 's bins' : '');
  detail.textContent = caption; detail.title = caption;
  if (!data.count) { var empty = document.createElement('span'); empty.className = 'distribution-empty'; empty.textContent = 'No timed solves'; bars.appendChild(empty); return; }
  var peak = Math.max.apply(null, data.bins.map(function(bin) { return bin.count; }));
  data.bins.forEach(function(bin, index) {
   var column = document.createElement('div'); column.className = 'distribution-bin'; column.tabIndex = index === 0 ? 0 : -1; column.setAttribute('role', 'img');
   var label = seconds(bin.from) + '–<' + seconds(bin.to) + 's · ' + bin.count + (bin.count === 1 ? ' solve' : ' solves') + ' · ' + (100 * bin.count / data.count).toFixed(1) + '%';
   column.setAttribute('aria-label', label); column.title = label; column.dataset.count = bin.count;
   var track = document.createElement('span'); track.className = 'distribution-track';
   var bar = document.createElement('span'); bar.className = 'distribution-bar'; bar.style.height = (100 * bin.count / peak) + '%';
   var count = document.createElement('span'); count.className = 'distribution-count'; count.textContent = bin.count || ''; bar.appendChild(count); track.appendChild(bar);
   var tick = document.createElement('span'); tick.className = 'distribution-tick'; tick.textContent = seconds(bin.from); column.append(track, tick);
   var compact = seconds(bin.from) + '–<' + seconds(bin.to) + 's · ' + bin.count + ' (' + (100 * bin.count / data.count).toFixed(1) + '%)';
   column.onpointerenter = function() { detail.textContent = compact; detail.title = label; };
   column.onfocus = function() {
    bars.querySelectorAll('.distribution-bin').forEach(function(item) { item.tabIndex = item === column ? 0 : -1; });
    column.onpointerenter();
   };
   column.onpointerleave = function() { if (document.activeElement !== column) { detail.textContent = caption; detail.title = caption; } };
   column.onblur = function() { detail.textContent = caption; detail.title = caption; };
   column.onkeydown = function(event) {
    event.stopPropagation();
    var next = event.key === 'ArrowLeft' ? index - 1 : event.key === 'ArrowRight' ? index + 1 : event.key === 'Home' ? 0 : event.key === 'End' ? data.bins.length - 1 : null;
    if (next != null) { event.preventDefault(); bars.children[Math.max(0, Math.min(data.bins.length - 1, next))].focus(); }
   };
   column.onkeyup = function(event) { event.stopPropagation(); };
   bars.appendChild(column);
  });
 }
 function appendHistory() {
  var body = node('history-rows'), have = body.children.length, end = Math.min(historyMatches.length, limit);
  if (!historyMatches.length) { body.innerHTML = '<tr><td class="empty" colspan="12">' + (length() ? 'No matching solves' : 'No solves yet') + '</td></tr>'; return; }
  var fragment = document.createDocumentFragment();
  var focus = body.querySelector('[tabindex="0"]'), focusIndex = focus ? Number(focus.dataset.index) : historyMatches.indexOf(selected) < end && historyMatches.indexOf(selected) >= 0 ? selected : historyMatches[0];
  for (var offset = have; offset < end; offset++) {
   var index = historyMatches[offset], result = stats.timesAt(index), metrics = recordMetrics(result, index);
   var row = document.createElement('tr'); row.dataset.index = index; row.tabIndex = index === focusIndex ? 0 : -1; row.setAttribute('role', 'row');
   row.setAttribute('aria-selected', String(selected === index)); row.classList.toggle('selected', selected === index);
   row.classList.toggle('has-favorite', LocalSolveTags.read(result).favorite); row.classList.toggle('is-dnf', result[0][0] < 0);
   var time = plain(stats.pretty(result[0], false));
   [String(index + 1), time, metrics.tps == null ? '—' : metrics.tps.toFixed(2), metrics.moves == null ? '—' : String(metrics.moves)].forEach(function(value, column) {
    var cell = document.createElement('td'); cell.textContent = value; if (column === 1 && stats.timeAt(Number(row.dataset.index)) === historyBest) cell.classList.add('best'); row.appendChild(cell);
   });
   var phases = null;
   if (view === 'history' && window.matchMedia('(min-width:761px)').matches && result[4] && result[4][0]) {
    try { phases = localReviewData.groupPhases(stats.getExtraInfo('recons_cf4op', index)); } catch (_) {}
   }
   ['cross', 'f2l', 'oll', 'pll'].forEach(function(name, order) {
    var phase = document.createElement('td'); phase.className = 'history-extended history-phase history-' + name + (order === 0 ? ' history-phase-first' : '');
    phase.textContent = phases ? pretty(phases[order].time) : '—'; row.appendChild(phase);
   });
   var pause = document.createElement('td'); pause.className = 'history-extended history-pause';
   pause.textContent = phases ? pretty(phases.reduce(function(total, phase) { return total + phase.pause; }, 0)) : '—';
   pause.title = phases ? 'Total recorded CFOP pause time' : 'No recorded CFOP reconstruction'; row.appendChild(pause);
   var when = document.createElement('td'); when.className = 'history-extended history-when';
   if (finite(result[3]) && result[3] > 0) {
    var date = new Date(result[3] < 1e12 ? result[3] * 1000 : result[3]);
    if (!isNaN(date.getTime())) { var stamp = document.createElement('time'); stamp.dateTime = date.toISOString(); stamp.textContent = historyDate.format(date); stamp.title = date.toLocaleString(); when.appendChild(stamp); }
   }
   if (!when.firstChild) when.textContent = '—'; row.appendChild(when);
   var tags = LocalSolveTags.read(result), tagCell = document.createElement('td'), editTags = document.createElement('button');
   tagCell.className = 'history-tags'; editTags.className = 'history-tags-edit'; editTags.tabIndex = -1;
   editTags.setAttribute('aria-label', 'Tags for solve ' + (index + 1) + (tags.favorite ? ', favorite' : '') + (tags.color ? ', ' + tags.color : '') + (tags.group ? ', ' + tags.group : ''));
   editTags.title = 'Tag solve · T'; editTags.dataset.index = index;
   var star = document.createElement('button'); star.className = tags.favorite ? 'history-favorite tag-star is-favorite' : 'history-favorite tag-star'; star.textContent = tags.favorite ? '★' : '☆'; star.tabIndex = -1; star.dataset.index = index; star.setAttribute('aria-label', 'Favorite solve ' + (index + 1)); star.setAttribute('aria-pressed', String(tags.favorite));
   star.onclick = function(event) { event.stopPropagation(); var idx = Number(event.currentTarget.dataset.index); saveTags(idx, {favorite:!LocalSolveTags.read(stats.timesAt(idx)).favorite}); };
   tagCell.appendChild(star);
   if (!tags.color) editTags.innerHTML = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 4h8l10 10-8 8L3 12z"/><circle cx="8" cy="9" r="1"/></svg>';
   if (tags.color) { var dot = document.createElement('i'); dot.className = 'tag-dot'; dot.dataset.color = tags.color; editTags.appendChild(dot); }
   if (tags.group) { var group = document.createElement('span'); group.className = 'tag-group'; group.textContent = tags.group; editTags.appendChild(group); }
   editTags.onclick = function(event) { event.stopPropagation(); tagTarget = Number(event.currentTarget.dataset.index); tagMenu.open(event.currentTarget); };
   tagCell.appendChild(editTags); row.appendChild(tagCell);
   var action = document.createElement('td'), button = document.createElement('button'); action.className = 'history-row-actions'; button.className = 'icon-button history-row-detail'; button.tabIndex = -1;
   button.setAttribute('aria-label', 'Details for solve ' + (index + 1)); button.title = 'Solve details';
   button.innerHTML = '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/></svg>';
   button.onkeydown = function(event) { if (event.key === 'ArrowLeft' || event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); event.currentTarget.closest('tr').focus(); } };
   button.onclick = function(event) { event.stopPropagation(); stats.showSolve(Number(event.currentTarget.closest('tr').dataset.index)); }; action.appendChild(button); row.appendChild(action);
   row.title = 'Solve ' + (index + 1) + (result[2] ? ': ' + result[2] : '') + '\nEnter to review';
   row.setAttribute('aria-label', 'Solve ' + (index + 1) + ', ' + time + (metrics.tps == null ? '' : ', ' + metrics.tps.toFixed(2) + ' tps') + (metrics.moves == null ? '' : ', ' + metrics.moves + ' moves') + (result[2] ? ', ' + result[2] : ''));
   row.addEventListener('click', function(event) { selectSolve(Number(event.currentTarget.dataset.index)); });
   row.addEventListener('dblclick', function(event) { stats.showSolve(Number(event.currentTarget.dataset.index)); });
   row.addEventListener('focus', function(event) { body.querySelectorAll('[data-index]').forEach(function(item) { item.tabIndex = item === event.currentTarget ? 0 : -1; }); });
   row.addEventListener('keyup', function(event) { event.stopPropagation(); });
   row.addEventListener('keydown', function(event) {
    if (event.target.closest('button')) return;
    if (event.key.toLowerCase() === 't') { event.preventDefault(); event.stopPropagation(); event.currentTarget.querySelector('.history-tags-edit').click(); return; }
    event.stopPropagation();
    if (event.key === 'ArrowRight' && view === 'history') { event.preventDefault(); event.currentTarget.querySelector('.history-row-detail').focus(); return; }
    if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); event.stopPropagation(); selectSolve(Number(event.currentTarget.dataset.index)); return; }
    var current = historyMatches.indexOf(Number(event.currentTarget.dataset.index));
    var target = event.key === 'ArrowDown' ? current + 1 : event.key === 'ArrowUp' ? current - 1 : event.key === 'Home' ? 0 : event.key === 'End' ? Math.min(limit, historyMatches.length) - 1 : null;
    if (target == null) return; event.preventDefault(); event.stopPropagation(); target = Math.max(0, Math.min(historyMatches.length - 1, target));
    if (target >= limit) { limit = Math.ceil((target + 1) / 100) * 100; appendHistory(); }
    var next = body.querySelector('[data-index="' + historyMatches[target] + '"]'); if (next) next.focus();
   });
   fragment.appendChild(row);
  }
  body.appendChild(fragment);
 }
 function sessionStat(label, value, className) {
  var stat = document.createElement('span'); stat.className = 'history-summary-stat' + (className ? ' ' + className : '');
  var name = document.createElement('span'); name.className = 'history-summary-label'; name.textContent = label;
  var number = document.createElement('strong'); number.className = 'history-summary-value'; number.textContent = value;
  stat.appendChild(name); stat.appendChild(number); node('local-session-summary').appendChild(stat); return stat;
 }
 function selectSolve(index) {
  if (timer.status() > 0) return;
  selected = index; renderSelected(); setView('timer');
  document.querySelectorAll('#history-rows [data-index]').forEach(function(row) { var active = Number(row.dataset.index) === selected; row.classList.toggle('selected', active); row.setAttribute('aria-selected', String(active)); });
 }
 function choosePhase(order) {
  selectedPhase = order; phaseExpanded = true; renderPhaseDetail();
  var phase = phaseDetails[order];
  reviewTools.choose(order === 0 ? 'cross' : order === 1 ? 'technique' : 'algorithms', phase && phase.name);
  if (matchMedia('(max-width:760px)').matches) document.body.dataset.toolsOpen = 'true';
 }
 function renderPhaseDetail() {
  document.querySelectorAll('#phase-rows tr[data-phase]').forEach(function(row) { row.classList.toggle('phase-selected', Number(row.dataset.phase) === selectedPhase && phaseExpanded); });
  document.querySelectorAll('#phase-strip button').forEach(function(button) { button.setAttribute('aria-pressed', String(Number(button.dataset.phase) === selectedPhase && phaseExpanded)); });
 }
 function saveTags(index, patch) {
  if (timer.status() > 0 || index < 0) return;
  var target = stats.timesAt(index);
  var savedSession = kernel.getProp('session');
  var active = document.activeElement, row = active && active.closest('#history-rows tr'), fromMenu = active && active.closest('#local-tag-menu');
  var focusClass = row ? active.classList.contains('history-favorite') ? '.history-favorite' : '.history-tags-edit' : fromMenu && view === 'history' ? '.history-tags-edit' : null;
  return stats.setReviewTags(index, Object.assign({}, LocalSolveTags.read(target), patch)).then(function() {
   if (!focusClass) return;
   requestAnimationFrame(function() {
    if (kernel.getProp('session') !== savedSession) return;
    var now = document.activeElement;
    if (now !== active && now !== document.body && now.isConnected) return;
    var currentRow = node('history-rows').querySelector('tr[data-index="' + index + '"]');
    if (currentRow) currentRow.querySelector(focusClass).focus({preventScroll:true});
    else { var next = node('history-rows').querySelector('tr[data-index]') || document.querySelector('[data-history-filter="' + historyFilter + '"]'); if (next) next.focus({preventScroll:true}); }
   });
  }).catch(function() { toast('Could not save solve tags'); });
 }
 function refreshTagControls() {
  var tags = LocalSolveTags.read(stats.timesAt(tagTarget));
  node('local-tag-menu').querySelectorAll('button').forEach(function(button) {
   var checked = button.hasAttribute('data-tag-favorite') ? tags.favorite : button.hasAttribute('data-tag-color') ? tags.color === button.dataset.tagColor : tags.group === button.dataset.tagGroup;
   button.setAttribute('aria-checked', String(checked));
  });
 }
 function renderSelected() {
  var result = record(), metrics = recordMetrics(result, selected);
  text('local-tps', metrics.tps == null ? '—' : metrics.tps.toFixed(2));
  text('local-moves', metrics.moves == null ? '—' : metrics.moves);
  node('replay-button').disabled = !(result && result[4] && result[4][0]);
  document.querySelectorAll('[data-penalty]').forEach(function(button) {
   button.disabled = !result;
   button.setAttribute('aria-pressed', String(!!result && result[0][0] === (button.dataset.penalty === 'plus' ? 2000 : -1)));
   button.classList.toggle('selected', !!result && result[0][0] === (button.dataset.penalty === 'plus' ? 2000 : -1));
  });
  node('favorite-button').disabled = node('tag-button').disabled = !result;
  node('favorite-button').setAttribute('aria-pressed', String(LocalSolveTags.read(result).favorite));
  var rows = node('phase-rows'), strip = node('phase-strip'); rows.replaceChildren(); strip.replaceChildren();
  phaseDetails = localReviewData.groupPhases(result && stats.getExtraInfo('recons_cf4op', selected)) || [];
  strip.hidden = !phaseDetails.length;
  node('phase-table').querySelector('thead').hidden = !phaseDetails.length;
  var total = phaseDetails.reduce(function(sum, p) { return sum + p.time; }, 0);
  if (phaseDetails.length) phaseDetails.forEach(function(p, order) {
   var row = document.createElement('tr');
   row.dataset.phase = order; row.tabIndex = 0; row.setAttribute('role', 'button'); row.setAttribute('aria-controls', 'review-tools');
   row.setAttribute('aria-label', p.name + ', ' + pretty(p.time) + ' seconds. Review tools.');
   row.onclick = function() { choosePhase(order); };
   row.onkeydown = function(event) { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); event.stopPropagation(); choosePhase(order); } };
   var cell = document.createElement('td'), label = document.createElement('span'); label.className = 'phase-name'; label.textContent = p.name; cell.appendChild(label); row.appendChild(cell);
   [pretty(p.time), pretty(p.pause), pretty(p.execution), p.moves, p.time > 0 ? (p.moves * 1000 / p.time).toFixed(2) : '—'].forEach(function(value, i) {
    var td = document.createElement('td');
    if (i === 0) { td.className = 'phase-time-pair'; td.innerHTML = '<span class="phase-pause-value"></span><span class="phase-plus">+</span><span class="phase-execution-value"></span>'; td.children[0].textContent = pretty(p.pause); td.children[2].textContent = pretty(p.execution); td.title = 'Pause ' + pretty(p.pause) + 's + execution ' + pretty(p.execution) + 's = ' + pretty(p.time) + 's'; td.setAttribute('aria-label', td.title); } else td.textContent = value; if (i === 1) td.className = 'phase-pause-cell'; if (i === 2) td.className = 'phase-execution-cell'; row.appendChild(td);
   }); rows.appendChild(row);
   var segment = document.createElement('button'), pause = document.createElement('span');
   segment.className = 'phase-segment'; segment.dataset.phase = order; segment.hidden = p.time === 0;
   segment.style.flexGrow = p.time; segment.style.flexBasis = '0px';
   segment.setAttribute('aria-label', p.name + ', ' + pretty(p.time) + ' seconds, ' + (total > 0 ? (p.time * 100 / total).toFixed(1) : '0') + '% of solve');
   segment.title = p.name + ' · ' + pretty(p.time) + ' s'; segment.setAttribute('aria-controls', 'review-tools');
   pause.className = 'pause-portion'; pause.style.width = (p.time > 0 ? p.pause * 100 / p.time : 0) + '%'; segment.appendChild(pause);
   segment.onclick = function() { choosePhase(order); }; strip.appendChild(segment);
  });
  else rows.innerHTML = '<tr><td colspan="6" class="phase-empty">' + (result ? 'No move data' : 'No solve selected') + '</td></tr>';
  if (reviewTools) reviewTools.update({record: result, index: selected, phases: phaseDetails});
  renderPhaseDetail(); renderClock(); syncPace();
 }
 function paceShape(tag, attributes) {
  var element = document.createElementNS('http://www.w3.org/2000/svg', tag);
  Object.keys(attributes).forEach(function(name) { element.setAttribute(name, attributes[name]); });
  node('pace-plot').appendChild(element); return element;
 }
 function syncPace(index) {
  if (!pacePoints.length) return;
  var point = pacePoints.find(function(p) { return p.index === (index == null ? selected : index); }) || pacePoints[pacePoints.length - 1];
  var ahead = point.time != null && sessionMean != null && point.time < sessionMean;
  text('pace-value', '#' + (point.index + 1) + ' · ' + (point.time == null ? 'DNF' : pretty(point.time)) + (ahead ? ' ↓' : ''));
  node('pace-value').classList.toggle('pace-ahead', ahead);
  node('pace-value').title = point.time == null || sessionMean == null ? '' : point.time === sessionMean ? 'Equal to session mean' : pretty(Math.abs(point.time - sessionMean)) + 's ' + (point.time < sessionMean ? 'faster' : 'slower') + ' than session mean';
  paceMarker.setAttribute('x1', point.x); paceMarker.setAttribute('x2', point.x);
  paceDot.style.display = point.time == null ? 'none' : '';
  paceDot.setAttribute('cx', point.x); paceDot.setAttribute('cy', point.y);
  node('pace-plot').querySelectorAll('[data-index]').forEach(function(target) {
   target.setAttribute('tabindex', Number(target.dataset.index) === point.index ? '0' : '-1');
   target.setAttribute('aria-pressed', String(Number(target.dataset.index) === selected));
  });
 }
 function renderPace() {
  var svg = node('pace-plot'); svg.replaceChildren();
  pacePoints = localReviewData.recentTimes({ length: length(), timeAt: stats.timeAt }, 32);
  node('session-pace').hidden = !pacePoints.length;
  if (!pacePoints.length) return;
  text('pace-caption', sessionMean == null ? 'Recent ' + pacePoints.length : 'Mean ' + pretty(sessionMean));
  node('pace-caption').title = 'Session mean of non-DNF scored times; plot shows the last ' + pacePoints.length + ' solves';
  var times = pacePoints.filter(function(p) { return p.time != null; }).map(function(p) { return p.time; });
  var low = times.length ? Math.min.apply(null, times) : 0, high = times.length ? Math.max.apply(null, times) : 1;
  if (sessionMean != null) { low = Math.min(low, sessionMean); high = Math.max(high, sessionMean); }
  var spread = high - low, path = '', open = false;
  if (sessionMean != null) { var meanY = spread > 0 ? 52 - (sessionMean - low) * 42 / spread : 31; paceShape('line', {'class':'pace-mean', x1:4, x2:204, y1:meanY, y2:meanY, 'aria-label':'Session mean ' + pretty(sessionMean) + ' seconds'}); }
  paceShape('path', { 'class': 'pace-guide', d: 'M4 10H204M4 52H204' });
  pacePoints.forEach(function(point, offset) {
   point.x = pacePoints.length === 1 ? 104 : 4 + offset * 200 / (pacePoints.length - 1);
   point.y = point.time == null ? 56 : spread > 0 ? 52 - (point.time - low) * 42 / spread : 31;
   if (point.time == null) { open = false; paceShape('path', { 'class': 'pace-dnf', d: 'M' + (point.x - 2) + ' 54l4 4m0 -4l-4 4' }); }
   else { path += (open ? ' L' : ' M') + point.x + ' ' + point.y; open = true; paceShape('circle', { 'class': 'pace-dot', cx: point.x, cy: point.y, r: 1.5 }); }
  });
  paceShape('path', { 'class': 'pace-path', d: path });
  paceMarker = paceShape('line', { 'class': 'pace-selection', y1: 4, y2: 60 });
  paceDot = paceShape('circle', { 'class': 'pace-point', r: 3 });
  pacePoints.forEach(function(point, offset) {
   var width = pacePoints.length === 1 ? 208 : 200 / (pacePoints.length - 1);
   var target = paceShape('rect', { 'class': 'pace-target', x: Math.max(0, point.x - width / 2), y: 0, width: Math.min(width, 208), height: 64, 'data-index': point.index, role: 'button', 'aria-label': 'Solve ' + (point.index + 1) + ', ' + (point.time == null ? 'DNF' : pretty(point.time) + ' seconds') });
   target.onpointerenter = target.onfocus = function() { syncPace(point.index); };
   target.onclick = function() { selectSolve(point.index); };
   target.onkeydown = function(event) {
    if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); event.stopPropagation(); selectSolve(point.index); return; }
    var next = event.key === 'ArrowLeft' ? offset - 1 : event.key === 'ArrowRight' ? offset + 1 : event.key === 'Home' ? 0 : event.key === 'End' ? pacePoints.length - 1 : null;
    if (next != null) { event.preventDefault(); event.stopPropagation(); next = Math.max(0, Math.min(pacePoints.length - 1, next)); svg.querySelector('[data-index="' + pacePoints[next].index + '"]').focus(); }
   };
  });
  svg.onpointerleave = function() { syncPace(); };
  svg.onfocusout = function(event) { if (!svg.contains(event.relatedTarget)) syncPace(); };
  syncPace();
 }
 function appendSummaryMetrics(stat, metrics, description) {
  var block = document.createElement('span'); block.className = 'history-summary-metrics';
  [[metrics.tps, 'tps', 2], [metrics.moves, 'moves', 1]].forEach(function(pair) {
   var item = document.createElement('span'), value = document.createElement('b'); value.textContent = pair[0] == null ? '—' : pair[0].toFixed(pair[2]); item.append(value, document.createTextNode(' ' + pair[1])); block.appendChild(item);
  }); stat.appendChild(block); stat.title = description;
 }
 function renderAverages() {
  var rows = node('local-average-rows'); rows.replaceChildren();
  var source = { TimeStat: TimeStat, length: length(), timeAt: stats.timeAt, recordAt: stats.timesAt, moveCount: function(i) { return stats.getExtraInfo('mvcnt_htm', i); } };
  [5, 12].forEach(function(size) {
   var avg = localAverageMetrics.calculateWindow(size, source);
   var row = document.createElement('tr'), name = document.createElement('th'); name.textContent = 'ao' + size; row.appendChild(name);
   [!avg ? '—' : avg.time < 0 ? 'DNF' : plain(kernel.pround(avg.time)), avg && avg.tps != null ? avg.tps.toFixed(2) : '—', avg && avg.moves != null ? avg.moves.toFixed(1) : '—'].forEach(function(value) { var td = document.createElement('td'); td.textContent = value; row.appendChild(td); });
   if (avg) row.title = 'TPS: ' + avg.tpsSamples + '/' + avg.retained + ' retained solves; moves: ' + avg.moveSamples + '/' + avg.retained + '. csTimer HTM.';
   rows.appendChild(row);
   var stat = sessionStat('ao' + size, !avg ? '—' : avg.time < 0 ? 'DNF' : plain(kernel.pround(avg.time)), 'history-summary-average');
   if (avg) {
    stat.title = row.title;
    var metrics = document.createElement('span'); metrics.className = 'history-summary-metrics';
    [[avg.tps, 'tps', 2], [avg.moves, 'moves', 1]].forEach(function(pair) {
     if (pair[0] == null) return;
     var item = document.createElement('span'), value = document.createElement('b'); value.textContent = pair[0].toFixed(pair[2]); item.appendChild(value); item.appendChild(document.createTextNode(' ' + pair[1])); metrics.appendChild(item);
    }); stat.appendChild(metrics);
   }
  });
 }
 function scheduleUpdate(info) {
  if (info && info[0] === 'session' || !pendingUpdate) pendingUpdate = info;
  if (updateFrame) return;
  updateFrame = requestAnimationFrame(function() { updateFrame = null; var next = pendingUpdate; pendingUpdate = null; update(next); });
 }
 function update(info) {
  if (!node('local-app')) return;
  var count = length(), session = kernel.getProp('session');
  if (session !== historySession) { if (tagMenu) tagMenu.close(false); tagTarget = -1; historySession = session; historyFilter = 'all'; historyQuery = ''; historyTagFilter = 'all'; node('history-query').value = ''; limit = 100; }
  if (count) delete document.body.dataset.empty; else document.body.dataset.empty = 'true';
  if (count !== previousLength || info && info[0] === 'session') { selected = count - 1; previousLength = count; }
  if (selected >= count) selected = count - 1;
  var best = Infinity, sum = 0, valid = 0;
  for (var i = 0; i < count; i++) { var time = stats.timeAt(i); if (time >= 0) { best = Math.min(best, time); sum += time; valid++; } }
  node('local-session-summary').hidden = !count; historyBest = best;
  node('local-session-summary').replaceChildren();
  document.querySelectorAll('[data-drill]').forEach(function(button) { button.setAttribute('aria-pressed', String(kernel.getProp('scrType') === button.dataset.drill)); });
  var summaries = localReviewData.sessionMetrics({length:count, timeAt:stats.timeAt, metricsAt:function(i){return recordMetrics(stats.timesAt(i),i);}});
  sessionMean = summaries.mean;
  var bestStat = sessionStat('Best', valid ? pretty(best) : '—');
  var bestMetrics = summaries.bestIndex < 0 ? {} : recordMetrics(stats.timesAt(summaries.bestIndex), summaries.bestIndex);
  appendSummaryMetrics(bestStat, bestMetrics, 'Solve ' + (summaries.bestIndex + 1));
  var meanStat = sessionStat('Mean', valid ? pretty(sum / valid) : '—'); appendSummaryMetrics(meanStat, summaries, 'TPS: ' + summaries.samples + '/' + summaries.count + '; moves: ' + summaries.moveSamples + '/' + summaries.count + '. Arithmetic means; non-DNF solves only');
  document.querySelectorAll('#local-app .session-count').forEach(function(el) { el.textContent = count + ' solves'; });
  var footer = document.querySelector('#local-app .history-footer'); footer.children[0].textContent = count + ' solves'; footer.children[1].textContent = valid ? Math.floor(sum / 60000) + 'm ' + Math.floor(sum / 1000) % 60 + 's' : '0s';
  syncSelect(node('session-button'), stats.getSessionManager().getSelect());
  syncSelect(node('puzzle-button'), $('#scrambleDiv > .title select').eq(1));
  renderPace(); resetHistory(false); renderSelected(); renderAverages(); schedulePublish();
  window.dispatchEvent(new Event('local-cube-redraw'));
 }
 function connect() {
  if (!owner) { toast('The cube is managed by another csTimer window'); if (channel) channel.postMessage({ type: 'focus' }); return; }
  if (GiikerCube.isConnected()) { actionMenu.toggle(node('device-button')); return; }
  if (kernel.getProp('input') !== 'g') kernel.setProp('input', 'g');
  else giikerutil.init().catch(function(error) { toast(error && error.message || 'Could not connect to the cube'); });
 }
 function refreshConnection() {
  var data = giikerutil.getViewState();
  var status = JSON.stringify([data.connected, data.name, owner]);
  var button = node('device-button');
  if (status !== lastConnection) {
   button.setAttribute('aria-haspopup', data.connected ? 'menu' : 'dialog');
   if (data.connected) button.setAttribute('aria-controls', 'local-more'); else { button.removeAttribute('aria-controls'); if (actionMenu) actionMenu.close(false); }
   button.classList.toggle('connected', !!data.connected); button.querySelector('span').textContent = data.connected ? data.name || 'Cube' : 'Connect';
   button.title = data.connected ? 'Connected: ' + (data.name || 'cube') : 'Connect a Bluetooth cube'; button.setAttribute('aria-label', button.title); lastConnection = status; schedulePublish();
  }
  if (data.facelets && (data.connected || !cubeState) && data.facelets !== cubeState) {
   cubeState = data.facelets; window.localCubeState = { facelets: cubeState, moves: [], puzzle: data.puzzle };
   window.dispatchEvent(new CustomEvent('local-cube-state', { detail: window.localCubeState }));
  }
 }
 function onCube(_, value) {
  cubeState = value[0];
  window.localCubeState = { facelets: value[0], moves: value[1], timestamps: value[2], puzzle: value[3] };
  window.dispatchEvent(new CustomEvent('local-cube-state', { detail: window.localCubeState }));
  if (guide && timer.status() <= 0 && giikerutil.isSync()) guideTokens = guide.update(value[0], value[1]);
  if (timer.status() <= 0 && value[1] && value[1].length && value[0] !== mathlib.SOLVED_FACELET) {
   var stamp = finite(value[2][0]) ? value[2][0] : value[2][1];
   if (scrambleStart == null) scrambleStart = stamp;
   scrambleMoves.push(value[1][0]);
   if (stamp > scrambleStart) { scrambleTps = recons.getMoveCnt(scrambleMoves.join(' ')) * 1000 / (stamp - scrambleStart); text('scramble-tps', scrambleTps.toFixed(2)); node('scramble-tps').parentElement.hidden = false; }
  }
  syncScramble(); refreshConnection(); schedulePublish();
 }
 function apply() {
  var enabled = kernel.getProp('localInterface') === true && new URLSearchParams(location.search).get('interface') !== 'classic';
  node('local-interface-css').disabled = !enabled;
  if (enabled) { var palette = kernel.getProp('localPalette'); document.documentElement.dataset.palette = palettes.indexOf(palette) >= 0 ? palette : 'paper'; }
  else delete document.documentElement.dataset.palette;
  if (!enabled && actionMenu) actionMenu.close(false);
  document.body.classList.toggle('local-interface', enabled);
  node('local-app').hidden = !enabled;
  if (enabled && !node('local-cube-module')) {
   var module = document.createElement('script'); module.id = 'local-cube-module'; module.type = 'module'; module.src = 'js/local-cube-view.js';
   module.onerror = function() { module.remove(); toast('Cube view could not load. Reload to retry.'); };
   document.body.appendChild(module);
  }

  if (enabled) node('local-tools-host').appendChild(originalTools);
  else toolsAnchor.parentNode.insertBefore(originalTools, toolsAnchor.nextSibling);
  document.documentElement.style.setProperty('--local-scale', (Number(kernel.getProp('timerSize')) || 12) / 12);
  if (enabled) { update(); renderClock(); }
 }
 function openWidget() {
  fetch('/api/companion/open', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: '{}' }).then(function(response) {
   if (!response.ok) throw new Error('Native companion unavailable');
  }).catch(function() { window.open('/companion.html', 'cstimer-companion', 'popup,width=320,height=112'); });
 }
 function init() {
  originalTools = node('toolsDiv'); toolsAnchor = document.createComment('Original tools position'); originalTools.parentNode.insertBefore(toolsAnchor, originalTools);
  window.localInterfaceId = (crypto.randomUUID ? crypto.randomUUID() : String(Date.now()) + Math.random());
  if (window.BroadcastChannel) { channel = new BroadcastChannel('cstimer-companion'); channel.onmessage = function(e) { if (e.data.type === 'request') publish(); if (e.data.type === 'focus' && owner) window.focus(); }; }
  if (navigator.locks) { owner = false; navigator.locks.request('cstimer-cube-owner', { ifAvailable: true }, function(lock) { owner = !!lock; refreshConnection(); if (lock) return new Promise(function(resolve) { window.addEventListener('pagehide', resolve, { once: true }); }); }); }
  kernel.regListener('localInterfaceAppearance', 'property', apply, /^(localInterface|timerSize|localPalette)$/);
  kernel.regProp('ui', 'localInterface', 0, 'Modern interface', [true]);
  kernel.regProp('ui', 'localPalette', 1, 'Appearance', ['paper', palettes, paletteLabels]);
  if (palettes.indexOf(kernel.getProp('localPalette')) < 0) kernel.setProp('localPalette', 'paper');
  kernel.regListener('localInterface', 'timerStatus', function() { renderClock(); schedulePublish(); window.dispatchEvent(new Event('local-cube-redraw')); });
  kernel.regListener('localInterface', 'localcube', onCube);
  kernel.regListener('localInterface', 'scramble', function(_, value) { guide = new LocalScrambleGuide(value[1] || ''); guideTokens = null; scrambleMoves = []; scrambleStart = null; scrambleTps = null; text('scramble-tps', '—'); node('scramble-tps').parentElement.hidden = true; setTimeout(function() { syncScramble(); scheduleUpdate(); }, 0); });
  kernel.regListener('localInterfaceData', 'property', function() { scheduleUpdate(); }, /^(session|sessionData|scrType|useMilli|timeFormat|trimr?)$/);
  stats.regUtil('localInterface', scheduleUpdate);
  new MutationObserver(function() { if (!clockTimer) clockTimer = requestAnimationFrame(function() { clockTimer = null; renderClock(); }); }).observe(node('multiphase'), { childList: true, subtree: true, attributes: true });
  new MutationObserver(renderClock).observe(node('lcd'), { childList: true, subtree: true });
  new MutationObserver(syncScramble).observe(node('scrambleTxt'), { childList: true, subtree: true, characterData: true });
  document.querySelectorAll('#local-app button[data-view]').forEach(function(button) { button.onclick = function() { if (timer.status() <= 0) setView(button.dataset.view); }; });
  // Focused review controls own their keys. csTimer otherwise blurs the active
  // element on keyup and treats a control's Space key as a timer trigger.
  function controlKeys(event) {
   if (timer.status() <= 0 && event.target.closest('button,select,a,textarea,input,[role=button]')) event.stopPropagation();
  }
  node('local-app').addEventListener('keydown', controlKeys);
  node('local-app').addEventListener('keyup', controlKeys);
  reviewTools = LocalReviewTools.create(node('review-tools'), {pretty:pretty, toast:toast, drill:function(type){kernel.setProp('scrType',type);document.body.dataset.toolsOpen='false';setView('timer');}});
  tagMenu = LocalActionMenu.create(node('local-tag-menu'), [node('tag-button')], function(){if(tagTarget<0)return false;refreshTagControls();});
  node('local-tag-menu').querySelectorAll('button').forEach(function(button){button.setAttribute('role','menuitemcheckbox');button.onclick=function(){var tags=LocalSolveTags.read(stats.timesAt(tagTarget)), patch=button.hasAttribute('data-tag-favorite')?{favorite:!tags.favorite}:button.hasAttribute('data-tag-color')?{color:button.dataset.tagColor}:{group:button.dataset.tagGroup};saveTags(tagTarget,patch);};});
  node('history-tag-filter').onchange=function(){historyTagFilter=this.value;resetHistory(true);};
  node('review-tools-toggle').onclick=function(){if(timer.status()<=0)document.body.dataset.toolsOpen='true';};
  node('review-tools-close').onclick=function(){document.body.dataset.toolsOpen='false';node('review-tools-toggle').focus();};
  document.addEventListener('keydown',function(event){if(event.key==='Escape'&&document.body.dataset.toolsOpen==='true'){event.preventDefault();document.body.dataset.toolsOpen='false';node('review-tools-toggle').focus();}});
  var compactLayout = window.matchMedia('(max-width:760px)');
  var narrowLayout = window.matchMedia('(max-width:1100px)');
  var shortLayout = window.matchMedia('(max-height:740px)');
  var sessionControl = node('session-button'), averages = document.querySelector('.average-table');
  function placeAnalysis() {
   document.body.dataset.toolsOpen = 'false';
   var timerPane = document.querySelector('.timer-pane'), analysisPane = document.querySelector('.analysis-pane');
   var phase = document.querySelector('.phase-section'); timerPane.appendChild(phase);
   if (!compactLayout.matches && (narrowLayout.matches || shortLayout.matches)) document.querySelector('.history-pane').insertBefore(averages, document.querySelector('.history-tools'));
   else document.querySelector('.clock-anchor').appendChild(averages);
   if (compactLayout.matches) document.querySelector('.context-left').appendChild(sessionControl);
   else document.querySelector('.history-heading').prepend(sessionControl);
   if (view === 'history') resetHistory(false);
   window.dispatchEvent(new Event('local-cube-redraw'));
  }
  [compactLayout, narrowLayout, shortLayout].forEach(function(query) { query.addEventListener('change', placeAnalysis); }); placeAnalysis();
  actionMenu = LocalActionMenu.create(node('local-more'), [node('history-menu'), node('device-button')], function(trigger) {
   var connected = GiikerCube.isConnected();
   if (trigger.id === 'device-button' && !connected) return false;
   node('local-detail').disabled = !record();
   node('local-reset-cube').disabled = !connected;
   node('local-disconnect').disabled = !connected;
   return true;
  });
  document.querySelectorAll('[data-history-filter]').forEach(function(button) { button.onclick = function() { historyFilter = button.dataset.historyFilter; resetHistory(true); }; });
  node('history-query').oninput = function() {
   historyQuery = this.value.trim();
   if (!historyFrame) historyFrame = requestAnimationFrame(function() { historyFrame = null; resetHistory(true); });
  };
  function clearHistoryQuery() { node('history-query').value = ''; historyQuery = ''; resetHistory(true); node('history-query').focus(); }
  node('history-query-clear').onclick = clearHistoryQuery;
  node('history-query').onkeydown = function(event) { if (event.key === 'Escape' && this.value) { event.preventDefault(); event.stopPropagation(); clearHistoryQuery(); } };
  node('device-button').onclick = connect;
  node('settings-button').onclick = function() { $('#leftbar .c1').click(); };
  node('history-menu').onclick = function() { actionMenu.toggle(this); };
  node('local-widget-button').onclick = openWidget;
  node('local-manage-sessions').onclick = function() { stats.getSessionManager().showMgrTable(); };
  node('local-detail').onclick = function() { if (record()) stats.showSolve(selected); };
  node('favorite-button').onclick = function(){if(record())saveTags(selected,{favorite:!LocalSolveTags.read(record()).favorite});};
  node('tag-button').onclick=function(){tagTarget=selected;tagMenu.toggle(this);};
  document.querySelectorAll('[data-penalty]').forEach(function(button) { button.onclick = function() { if (record()) { var penalty = button.dataset.penalty === 'plus' ? 2000 : -1; stats.setPenalty(selected, record()[0][0] === penalty ? 0 : penalty); } }; });
  node('replay-button').onclick = function() { var r = record(); if (r && r[4]) replay.popupReplay(r[1], r[4][0], typeof r[4][1] === 'string' ? r[4][1] : '333'); };
  node('copy-scramble').onclick = function() { $.clipboardCopy(tools.getCurScramble()[1]).then(function() { toast('Scramble copied'); }); };
  node('session-button').onchange = function() { stats.getSessionManager().getSelect().val(this.value).trigger('change'); this.blur(); };
  node('puzzle-button').onchange = function() { $('#scrambleDiv > .title select').eq(1).val(this.value).trigger('change'); this.blur(); };
  node('local-reset-cube').onclick = function() { giikerutil.markSolved(); cubeState = null; refreshConnection(); guideTokens = guide ? guide.update(giikerutil.getViewState().facelets, []) : null; syncScramble(); schedulePublish(); };
  node('local-disconnect').onclick = function() { giikerutil.stop(); };
  node('local-legacy').onclick = function() { kernel.setProp('localInterface', false); };
  document.querySelectorAll('[data-stock]').forEach(function(button) { button.onclick = function() { $('#leftbar .' + button.dataset.stock).click(); }; });
  document.querySelectorAll('[data-drill]').forEach(function(button) { button.onclick = function() { kernel.setProp('scrType', button.dataset.drill); setView('timer'); }; });
  function scrambleOptions() { $('#scrambleDiv > .title input.icon').click(); }
  node('local-case-filter').onclick = scrambleOptions; node('local-scramble-options').onclick = scrambleOptions;
  document.querySelector('.history-scroll').addEventListener('scroll', function() { if (this.scrollHeight - this.scrollTop < this.clientHeight + 150) { limit += 100; appendHistory(); } });
  node('clock').addEventListener('pointerdown', function(event) { if (kernel.getProp('input') === 't') { event.preventDefault(); $(document).trigger($.Event('keydown', { which: 32 })); this.setPointerCapture(event.pointerId); } });
  node('clock').addEventListener('pointerup', function() { if (kernel.getProp('input') === 't') $(document).trigger($.Event('keyup', { which: 32 })); });
  document.addEventListener('click', function(event) { var button = event.target.closest('#local-app button'); if (button && event.detail > 0) button.blur(); });
  guide = new LocalScrambleGuide(tools.getCurScramble()[1] || '');
  node('scramble-tps').parentElement.hidden = scrambleTps == null;
  setView('timer'); apply(); syncScramble(); refreshConnection();
  setInterval(function() { refreshConnection(); if (owner) publish(); }, 1000);
  window.addEventListener('pagehide', function() {
   if (!owner) return;
   var state = currentSnapshot(); state.connected = false;
   fetch('/api/companion/state', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(state), keepalive: true }).catch(function() {});
  });
  if (navigator.bluetooth && navigator.bluetooth.addEventListener) navigator.bluetooth.addEventListener('availabilitychanged', function(event) { node('device-button').classList.toggle('unavailable', !event.value); refreshConnection(); });
 }
 $(init);
 return { snapshot: currentSnapshot, update: update, selectSolve: selectSolve, openWidget: openWidget };
})();
